use super::{ConnectionConfig, DatabaseConnector, DatabaseEngine};
use crate::db::schema::{
    ColumnInfo, ConstraintInfo, ConstraintType, IndexInfo, Row, SchemaInfo, TableInfo,
};
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use tokio_postgres::{Client, NoTls};

/// PostgreSQL connector using tokio-postgres
pub struct PostgresConnector {
    config: ConnectionConfig,
    client: Option<Client>,
}

impl PostgresConnector {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            client: None,
        }
    }

    /// Build a connection string from our ConnectionConfig
    fn build_connection_string(&self) -> String {
        if let Some(ref conn_str) = self.config.connection_string {
            return conn_str.clone();
        }

        let mut parts = Vec::new();

        if let Some(ref host) = self.config.host {
            parts.push(format!("host={}", host));
        } else {
            parts.push("host=localhost".to_string());
        }

        if let Some(port) = self.config.port {
            parts.push(format!("port={}", port));
        } else {
            parts.push("port=5432".to_string());
        }

        if let Some(ref db) = self.config.database {
            parts.push(format!("dbname={}", db));
        }

        if let Some(ref user) = self.config.username {
            parts.push(format!("user={}", user));
        }

        if let Some(ref pass) = self.config.password {
            parts.push(format!("password={}", pass));
        }

        parts.push(format!(
            "connect_timeout={}",
            self.config.connection_timeout_secs
        ));

        parts.join(" ")
    }

    /// Get a reference to the connected client, or return an error
    fn client(&self) -> anyhow::Result<&Client> {
        self.client
            .as_ref()
            .ok_or_else(|| anyhow!("Not connected to PostgreSQL"))
    }

    /// Convert a tokio_postgres::Row to our Row type (HashMap<String, Value>)
    fn row_to_map(row: &tokio_postgres::Row) -> Row {
        let mut map = std::collections::HashMap::new();
        for (i, col) in row.columns().iter().enumerate() {
            let name = col.name().to_string();
            let value = Self::column_to_json(row, i, col.type_());
            map.insert(name, value);
        }
        map
    }

    /// Convert a single column value to serde_json::Value
    fn column_to_json(
        row: &tokio_postgres::Row,
        idx: usize,
        pg_type: &tokio_postgres::types::Type,
    ) -> serde_json::Value {
        use tokio_postgres::types::Type;

        // Try to extract value based on PostgreSQL type
        match *pg_type {
            Type::BOOL => row
                .try_get::<_, Option<bool>>(idx)
                .ok()
                .flatten()
                .map(serde_json::Value::Bool)
                .unwrap_or(serde_json::Value::Null),

            Type::INT2 => row
                .try_get::<_, Option<i16>>(idx)
                .ok()
                .flatten()
                .map(|v| serde_json::json!(v))
                .unwrap_or(serde_json::Value::Null),

            Type::INT4 => row
                .try_get::<_, Option<i32>>(idx)
                .ok()
                .flatten()
                .map(|v| serde_json::json!(v))
                .unwrap_or(serde_json::Value::Null),

            Type::INT8 => row
                .try_get::<_, Option<i64>>(idx)
                .ok()
                .flatten()
                .map(|v| serde_json::json!(v))
                .unwrap_or(serde_json::Value::Null),

            Type::FLOAT4 => row
                .try_get::<_, Option<f32>>(idx)
                .ok()
                .flatten()
                .map(|v| serde_json::json!(v))
                .unwrap_or(serde_json::Value::Null),

            Type::FLOAT8 | Type::NUMERIC => row
                .try_get::<_, Option<f64>>(idx)
                .ok()
                .flatten()
                .map(|v| serde_json::json!(v))
                .unwrap_or(serde_json::Value::Null),

            Type::JSON | Type::JSONB => row
                .try_get::<_, Option<serde_json::Value>>(idx)
                .ok()
                .flatten()
                .unwrap_or(serde_json::Value::Null),

            _ => {
                // Fallback: try as String for text, varchar, date, timestamp, uuid, etc.
                row.try_get::<_, Option<String>>(idx)
                    .ok()
                    .flatten()
                    .map(serde_json::Value::String)
                    .unwrap_or(serde_json::Value::Null)
            }
        }
    }
}

#[async_trait]
impl DatabaseConnector for PostgresConnector {
    async fn connect(&mut self) -> anyhow::Result<()> {
        let conn_str = self.build_connection_string();

        let (client, connection) = tokio_postgres::connect(&conn_str, NoTls)
            .await
            .context("Failed to connect to PostgreSQL")?;

        // Spawn the connection handler in the background
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                log::error!("PostgreSQL connection error: {}", e);
            }
        });

        self.client = Some(client);
        Ok(())
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        self.client = None;
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        if let Some(ref client) = self.client {
            // Try a simple query to verify connection is alive
            client.simple_query("SELECT 1").await.is_ok()
        } else {
            false
        }
    }

    async fn get_schema(&self) -> anyhow::Result<SchemaInfo> {
        let db_name = self.config.database.clone().unwrap_or_default();
        let tables = self.get_tables().await?;
        let mut table_infos = Vec::new();

        for table_name in &tables {
            match self.get_table_info(table_name).await {
                Ok(info) => table_infos.push(info),
                Err(e) => {
                    log::warn!("Failed to get info for table {}: {}", table_name, e);
                }
            }
        }

        Ok(SchemaInfo {
            database_name: db_name,
            tables: table_infos,
        })
    }

    async fn get_tables(&self) -> anyhow::Result<Vec<String>> {
        let client = self.client()?;

        let rows = client
            .query(
                "SELECT table_name FROM information_schema.tables \
                 WHERE table_schema = 'public' AND table_type = 'BASE TABLE' \
                 ORDER BY table_name",
                &[],
            )
            .await
            .context("Failed to query tables")?;

        let tables: Vec<String> = rows.iter().map(|row| row.get(0)).collect();
        Ok(tables)
    }

    async fn get_table_info(&self, table_name: &str) -> anyhow::Result<TableInfo> {
        let columns = self.get_columns(table_name).await?;
        let indexes = self.get_indexes(table_name).await?;
        let constraints = self.get_constraints(table_name).await?;
        let row_count = self.get_row_count(table_name).await.ok();

        Ok(TableInfo {
            schema_name: "public".to_string(),
            table_name: table_name.to_string(),
            columns,
            indexes,
            constraints,
            row_count,
        })
    }

    async fn get_rows(
        &self,
        table_name: &str,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> anyhow::Result<Vec<Row>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let query = format!(
            "SELECT * FROM \"{}\" LIMIT {} OFFSET {}",
            table_name.replace('"', "\"\""),
            limit,
            offset
        );

        self.execute_query(&query).await
    }

    async fn execute_query(&self, query: &str) -> anyhow::Result<Vec<Row>> {
        let client = self.client()?;

        let rows = client
            .query(query, &[])
            .await
            .context("Failed to execute query")?;

        let mut result = Vec::new();
        for row in &rows {
            result.push(Self::row_to_map(row));
        }
        Ok(result)
    }

    async fn begin_transaction(&mut self) -> anyhow::Result<()> {
        let client = self.client()?;
        client
            .batch_execute("BEGIN")
            .await
            .context("Failed to begin transaction")?;
        Ok(())
    }

    async fn commit_transaction(&mut self) -> anyhow::Result<()> {
        let client = self.client()?;
        client
            .batch_execute("COMMIT")
            .await
            .context("Failed to commit transaction")?;
        Ok(())
    }

    async fn rollback_transaction(&mut self) -> anyhow::Result<()> {
        let client = self.client()?;
        client
            .batch_execute("ROLLBACK")
            .await
            .context("Failed to rollback transaction")?;
        Ok(())
    }

    fn engine(&self) -> DatabaseEngine {
        DatabaseEngine::PostgreSql
    }

    async fn get_row_count(&self, table_name: &str) -> anyhow::Result<i64> {
        let client = self.client()?;

        let query = format!(
            "SELECT COUNT(*) FROM \"{}\"",
            table_name.replace('"', "\"\"")
        );

        let rows = client
            .query(&query as &str, &[])
            .await
            .context("Failed to query row count")?;

        let row = rows
            .first()
            .ok_or_else(|| anyhow!("No result from COUNT query"))?;
        let count: i64 = row.try_get(0)?;
        Ok(count)
    }
}

/// Private helper methods for schema introspection
impl PostgresConnector {
    async fn get_columns(&self, table_name: &str) -> anyhow::Result<Vec<ColumnInfo>> {
        let client = self.client()?;

        // Get primary key columns first
        let pk_rows = client
            .query(
                "SELECT kcu.column_name \
                 FROM information_schema.table_constraints tc \
                 JOIN information_schema.key_column_usage kcu \
                   ON tc.constraint_name = kcu.constraint_name \
                   AND tc.table_schema = kcu.table_schema \
                 WHERE tc.constraint_type = 'PRIMARY KEY' \
                   AND tc.table_schema = 'public' \
                   AND tc.table_name = $1",
                &[&table_name],
            )
            .await?;

        let pk_columns: Vec<String> = pk_rows.iter().map(|r| r.get(0)).collect();

        // Get column details
        let col_rows = client
            .query(
                "SELECT column_name, data_type, is_nullable, \
                        character_maximum_length, numeric_precision, numeric_scale, \
                        column_default, ordinal_position \
                 FROM information_schema.columns \
                 WHERE table_schema = 'public' AND table_name = $1 \
                 ORDER BY ordinal_position",
                &[&table_name],
            )
            .await?;

        let mut columns = Vec::new();
        for row in &col_rows {
            let name: String = row.get(0);
            let data_type: String = row.get(1);
            let nullable_str: String = row.get(2);
            let max_length: Option<i32> = row.get(3);
            let precision: Option<i32> = row.get(4);
            let scale: Option<i32> = row.get(5);
            let default_value: Option<String> = row.get(6);
            let ordinal: i32 = row.get(7);

            columns.push(ColumnInfo {
                name: name.clone(),
                data_type,
                is_nullable: nullable_str == "YES",
                is_primary_key: pk_columns.contains(&name),
                max_length,
                precision,
                scale,
                default_value,
                ordinal_position: ordinal,
            });
        }

        Ok(columns)
    }

    async fn get_indexes(&self, table_name: &str) -> anyhow::Result<Vec<IndexInfo>> {
        let client = self.client()?;

        let rows = client
            .query(
                "SELECT indexname, indexdef \
                 FROM pg_indexes \
                 WHERE schemaname = 'public' AND tablename = $1",
                &[&table_name],
            )
            .await?;

        let mut indexes = Vec::new();
        for row in &rows {
            let name: String = row.get(0);
            let indexdef: String = row.get(1);

            let is_unique = indexdef.to_uppercase().contains("UNIQUE");
            // PostgreSQL doesn't have "clustered" indexes in the SQL Server sense
            let is_clustered = false;

            // Parse column names from index definition
            // Format: CREATE [UNIQUE] INDEX name ON table USING btree (col1, col2)
            let columns = Self::parse_index_columns(&indexdef);

            let index_type = if indexdef.to_uppercase().contains("USING BTREE") {
                "BTREE".to_string()
            } else if indexdef.to_uppercase().contains("USING HASH") {
                "HASH".to_string()
            } else if indexdef.to_uppercase().contains("USING GIN") {
                "GIN".to_string()
            } else if indexdef.to_uppercase().contains("USING GIST") {
                "GIST".to_string()
            } else {
                "BTREE".to_string() // default
            };

            indexes.push(IndexInfo {
                name,
                columns,
                is_unique,
                is_clustered,
                index_type,
            });
        }

        Ok(indexes)
    }

    /// Parse column names from a PostgreSQL index definition string
    fn parse_index_columns(indexdef: &str) -> Vec<String> {
        // Extract everything between the last pair of parentheses
        if let Some(start) = indexdef.rfind('(') {
            if let Some(end) = indexdef.rfind(')') {
                if start < end {
                    let cols_str = &indexdef[start + 1..end];
                    return cols_str
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect();
                }
            }
        }
        Vec::new()
    }

    async fn get_constraints(&self, table_name: &str) -> anyhow::Result<Vec<ConstraintInfo>> {
        let client = self.client()?;

        let rows = client
            .query(
                "SELECT tc.constraint_name, tc.constraint_type, \
                        kcu.column_name, \
                        ccu.table_name AS referenced_table, \
                        ccu.column_name AS referenced_column \
                 FROM information_schema.table_constraints tc \
                 JOIN information_schema.key_column_usage kcu \
                   ON tc.constraint_name = kcu.constraint_name \
                   AND tc.table_schema = kcu.table_schema \
                 LEFT JOIN information_schema.referential_constraints rc \
                   ON tc.constraint_name = rc.constraint_name \
                 LEFT JOIN information_schema.constraint_column_usage ccu \
                   ON rc.unique_constraint_name = ccu.constraint_name \
                 WHERE tc.table_schema = 'public' AND tc.table_name = $1 \
                 ORDER BY tc.constraint_name, kcu.ordinal_position",
                &[&table_name],
            )
            .await?;

        let mut constraints: std::collections::HashMap<String, ConstraintInfo> =
            std::collections::HashMap::new();

        for row in &rows {
            let name: String = row.get(0);
            let ctype: String = row.get(1);
            let col: String = row.get(2);
            let ref_table: Option<String> = row.get(3);
            let ref_col: Option<String> = row.get(4);

            let constraint_type = match ctype.as_str() {
                "PRIMARY KEY" => ConstraintType::PrimaryKey,
                "FOREIGN KEY" => ConstraintType::ForeignKey,
                "UNIQUE" => ConstraintType::Unique,
                "CHECK" => ConstraintType::Check,
                _ => ConstraintType::Default,
            };

            let entry = constraints
                .entry(name.clone())
                .or_insert_with(|| ConstraintInfo {
                    name,
                    constraint_type,
                    columns: Vec::new(),
                    referenced_table: ref_table,
                    referenced_columns: None,
                });

            if !entry.columns.contains(&col) {
                entry.columns.push(col);
            }

            if let Some(rc) = ref_col {
                let ref_cols = entry.referenced_columns.get_or_insert_with(Vec::new);
                if !ref_cols.contains(&rc) {
                    ref_cols.push(rc);
                }
            }
        }

        Ok(constraints.into_values().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_connector() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::PostgreSql,
            host: Some("localhost".to_string()),
            port: Some(5432),
            database: Some("testdb".to_string()),
            username: Some("postgres".to_string()),
            password: Some("password".to_string()),
            ..Default::default()
        };
        let connector = PostgresConnector::new(config);
        assert_eq!(connector.engine(), DatabaseEngine::PostgreSql);
    }

    #[test]
    fn test_build_connection_string_from_params() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::PostgreSql,
            host: Some("myhost".to_string()),
            port: Some(5433),
            database: Some("mydb".to_string()),
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
            ..Default::default()
        };
        let connector = PostgresConnector::new(config);
        let conn_str = connector.build_connection_string();
        assert!(conn_str.contains("host=myhost"));
        assert!(conn_str.contains("port=5433"));
        assert!(conn_str.contains("dbname=mydb"));
        assert!(conn_str.contains("user=user"));
        assert!(conn_str.contains("password=pass"));
    }

    #[test]
    fn test_build_connection_string_defaults() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::PostgreSql,
            ..Default::default()
        };
        let connector = PostgresConnector::new(config);
        let conn_str = connector.build_connection_string();
        assert!(conn_str.contains("host=localhost"));
        assert!(conn_str.contains("port=5432"));
    }

    #[test]
    fn test_build_connection_string_from_raw() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::PostgreSql,
            connection_string: Some("host=myhost port=5432 dbname=mydb".to_string()),
            ..Default::default()
        };
        let connector = PostgresConnector::new(config);
        let conn_str = connector.build_connection_string();
        assert_eq!(conn_str, "host=myhost port=5432 dbname=mydb");
    }

    #[test]
    fn test_not_connected_by_default() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::PostgreSql,
            ..Default::default()
        };
        let connector = PostgresConnector::new(config);
        assert!(connector.client.is_none());
    }

    #[test]
    fn test_engine_returns_postgresql() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::PostgreSql,
            ..Default::default()
        };
        let connector = PostgresConnector::new(config);
        assert_eq!(connector.engine(), DatabaseEngine::PostgreSql);
    }

    #[test]
    fn test_parse_index_columns() {
        let indexdef = "CREATE INDEX idx_test ON public.test USING btree (col1, col2)";
        let cols = PostgresConnector::parse_index_columns(indexdef);
        assert_eq!(cols, vec!["col1".to_string(), "col2".to_string()]);
    }

    #[test]
    fn test_parse_index_columns_single() {
        let indexdef = "CREATE UNIQUE INDEX idx_pk ON public.test USING btree (id)";
        let cols = PostgresConnector::parse_index_columns(indexdef);
        assert_eq!(cols, vec!["id".to_string()]);
    }
}
