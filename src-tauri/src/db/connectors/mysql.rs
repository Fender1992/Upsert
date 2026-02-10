use super::{ConnectionConfig, DatabaseConnector, DatabaseEngine};
use crate::db::schema::{
    ColumnInfo, ConstraintInfo, ConstraintType, IndexInfo, Row, SchemaInfo, TableInfo,
};
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use mysql_async::prelude::*;
use mysql_async::{Conn, Opts, OptsBuilder, Pool};

/// MySQL connector using mysql_async
pub struct MySqlConnector {
    config: ConnectionConfig,
    pool: Option<Pool>,
}

impl MySqlConnector {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            pool: None,
        }
    }

    /// Build MySQL connection options from our config.
    fn build_opts(&self) -> anyhow::Result<Opts> {
        if let Some(ref conn_str) = self.config.connection_string {
            return Opts::from_url(conn_str)
                .map_err(|e| anyhow!("Invalid MySQL connection string: {}", e));
        }

        let mut builder = OptsBuilder::default()
            .ip_or_hostname(
                self.config
                    .host
                    .clone()
                    .unwrap_or_else(|| "localhost".to_string()),
            )
            .tcp_port(self.config.port.unwrap_or(3306));

        if let Some(ref db) = self.config.database {
            builder = builder.db_name(Some(db.clone()));
        }

        if let Some(ref user) = self.config.username {
            builder = builder.user(Some(user.clone()));
        }

        if let Some(ref pass) = self.config.password {
            builder = builder.pass(Some(pass.clone()));
        }

        Ok(builder.into())
    }

    /// Get a connection from the pool.
    async fn get_conn(&self) -> anyhow::Result<Conn> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| anyhow!("Not connected to MySQL"))?;
        pool.get_conn()
            .await
            .context("Failed to get MySQL connection from pool")
    }

    /// Get the current database name from config.
    fn database_name(&self) -> String {
        self.config.database.clone().unwrap_or_default()
    }
}

#[async_trait]
impl DatabaseConnector for MySqlConnector {
    async fn connect(&mut self) -> anyhow::Result<()> {
        let opts = self.build_opts()?;
        let pool = Pool::new(opts);

        // Test the connection by getting a conn and pinging
        let mut conn = pool
            .get_conn()
            .await
            .context("Failed to connect to MySQL")?;
        conn.ping().await.context("MySQL ping failed")?;

        self.pool = Some(pool);
        Ok(())
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        if let Some(pool) = self.pool.take() {
            pool.disconnect().await.context("Failed to disconnect MySQL pool")?;
        }
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        if let Some(ref pool) = self.pool {
            if let Ok(mut conn) = pool.get_conn().await {
                return conn.ping().await.is_ok();
            }
        }
        false
    }

    async fn get_schema(&self) -> anyhow::Result<SchemaInfo> {
        let db_name = self.database_name();
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
        let mut conn = self.get_conn().await?;
        let db = self.database_name();

        let rows: Vec<String> = conn
            .exec(
                "SELECT TABLE_NAME FROM information_schema.TABLES \
                 WHERE TABLE_SCHEMA = ? AND TABLE_TYPE = 'BASE TABLE' \
                 ORDER BY TABLE_NAME",
                (db,),
            )
            .await
            .context("Failed to query MySQL tables")?;

        Ok(rows)
    }

    async fn get_table_info(&self, table_name: &str) -> anyhow::Result<TableInfo> {
        let columns = self.get_columns(table_name).await?;
        let indexes = self.get_indexes(table_name).await?;
        let constraints = self.get_constraints(table_name).await?;
        let row_count = self.get_row_count(table_name).await.ok();

        Ok(TableInfo {
            schema_name: self.database_name(),
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
            "SELECT * FROM `{}` LIMIT {} OFFSET {}",
            table_name.replace('`', "``"),
            limit,
            offset
        );

        self.execute_query(&query).await
    }

    async fn execute_query(&self, query: &str) -> anyhow::Result<Vec<Row>> {
        let mut conn = self.get_conn().await?;

        let result: Vec<mysql_async::Row> = conn
            .query(query)
            .await
            .context("Failed to execute MySQL query")?;

        let mut rows = Vec::new();
        for mysql_row in &result {
            rows.push(mysql_row_to_map(mysql_row));
        }
        Ok(rows)
    }

    async fn begin_transaction(&mut self) -> anyhow::Result<()> {
        let mut conn = self.get_conn().await?;
        conn.query_drop("START TRANSACTION")
            .await
            .context("Failed to begin MySQL transaction")?;
        Ok(())
    }

    async fn commit_transaction(&mut self) -> anyhow::Result<()> {
        let mut conn = self.get_conn().await?;
        conn.query_drop("COMMIT")
            .await
            .context("Failed to commit MySQL transaction")?;
        Ok(())
    }

    async fn rollback_transaction(&mut self) -> anyhow::Result<()> {
        let mut conn = self.get_conn().await?;
        conn.query_drop("ROLLBACK")
            .await
            .context("Failed to rollback MySQL transaction")?;
        Ok(())
    }

    fn engine(&self) -> DatabaseEngine {
        DatabaseEngine::MySql
    }

    async fn get_row_count(&self, table_name: &str) -> anyhow::Result<i64> {
        let mut conn = self.get_conn().await?;
        let query = format!(
            "SELECT COUNT(*) FROM `{}`",
            table_name.replace('`', "``")
        );

        let count: Option<i64> = conn
            .query_first(&query)
            .await
            .context("Failed to get row count")?;

        count.ok_or_else(|| anyhow!("No result from COUNT query"))
    }
}

/// Private helper methods for MySQL schema introspection.
impl MySqlConnector {
    async fn get_columns(&self, table_name: &str) -> anyhow::Result<Vec<ColumnInfo>> {
        let mut conn = self.get_conn().await?;
        let db = self.database_name();

        // Get primary key columns
        let pk_rows: Vec<String> = conn
            .exec(
                "SELECT COLUMN_NAME FROM information_schema.KEY_COLUMN_USAGE \
                 WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ? AND CONSTRAINT_NAME = 'PRIMARY' \
                 ORDER BY ORDINAL_POSITION",
                (&db, table_name),
            )
            .await?;

        // Get column details
        #[allow(clippy::type_complexity)]
        let col_rows: Vec<(String, String, String, Option<i64>, Option<i64>, Option<i64>, Option<String>, i64)> = conn
            .exec(
                "SELECT COLUMN_NAME, COLUMN_TYPE, IS_NULLABLE, \
                        CHARACTER_MAXIMUM_LENGTH, NUMERIC_PRECISION, NUMERIC_SCALE, \
                        COLUMN_DEFAULT, ORDINAL_POSITION \
                 FROM information_schema.COLUMNS \
                 WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ? \
                 ORDER BY ORDINAL_POSITION",
                (&db, table_name),
            )
            .await?;

        let mut columns = Vec::new();
        for (name, data_type, nullable, max_len, precision, scale, default_val, ordinal) in
            &col_rows
        {
            columns.push(ColumnInfo {
                name: name.clone(),
                data_type: data_type.clone(),
                is_nullable: nullable == "YES",
                is_primary_key: pk_rows.contains(name),
                max_length: max_len.map(|v| v as i32),
                precision: precision.map(|v| v as i32),
                scale: scale.map(|v| v as i32),
                default_value: default_val.clone(),
                ordinal_position: *ordinal as i32,
            });
        }

        Ok(columns)
    }

    async fn get_indexes(&self, table_name: &str) -> anyhow::Result<Vec<IndexInfo>> {
        let mut conn = self.get_conn().await?;
        let db = self.database_name();

        let rows: Vec<(String, i32, String, String)> = conn
            .exec(
                "SELECT INDEX_NAME, NON_UNIQUE, COLUMN_NAME, INDEX_TYPE \
                 FROM information_schema.STATISTICS \
                 WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ? \
                 ORDER BY INDEX_NAME, SEQ_IN_INDEX",
                (&db, table_name),
            )
            .await?;

        let mut index_map: std::collections::HashMap<String, IndexInfo> =
            std::collections::HashMap::new();

        for (name, non_unique, column, index_type) in &rows {
            let entry = index_map.entry(name.clone()).or_insert_with(|| IndexInfo {
                name: name.clone(),
                columns: Vec::new(),
                is_unique: *non_unique == 0,
                is_clustered: name == "PRIMARY",
                index_type: index_type.clone(),
            });
            entry.columns.push(column.clone());
        }

        Ok(index_map.into_values().collect())
    }

    async fn get_constraints(&self, table_name: &str) -> anyhow::Result<Vec<ConstraintInfo>> {
        let mut conn = self.get_conn().await?;
        let db = self.database_name();

        #[allow(clippy::type_complexity)]
        let rows: Vec<(String, String, String, Option<String>, Option<String>)> = conn
            .exec(
                "SELECT tc.CONSTRAINT_NAME, tc.CONSTRAINT_TYPE, kcu.COLUMN_NAME, \
                        kcu.REFERENCED_TABLE_NAME, kcu.REFERENCED_COLUMN_NAME \
                 FROM information_schema.TABLE_CONSTRAINTS tc \
                 JOIN information_schema.KEY_COLUMN_USAGE kcu \
                   ON tc.CONSTRAINT_NAME = kcu.CONSTRAINT_NAME \
                   AND tc.TABLE_SCHEMA = kcu.TABLE_SCHEMA \
                   AND tc.TABLE_NAME = kcu.TABLE_NAME \
                 WHERE tc.TABLE_SCHEMA = ? AND tc.TABLE_NAME = ? \
                 ORDER BY tc.CONSTRAINT_NAME, kcu.ORDINAL_POSITION",
                (&db, table_name),
            )
            .await?;

        let mut constraint_map: std::collections::HashMap<String, ConstraintInfo> =
            std::collections::HashMap::new();

        for (name, ctype, col, ref_table, ref_col) in &rows {
            let constraint_type = match ctype.as_str() {
                "PRIMARY KEY" => ConstraintType::PrimaryKey,
                "FOREIGN KEY" => ConstraintType::ForeignKey,
                "UNIQUE" => ConstraintType::Unique,
                "CHECK" => ConstraintType::Check,
                _ => ConstraintType::Default,
            };

            let entry =
                constraint_map
                    .entry(name.clone())
                    .or_insert_with(|| ConstraintInfo {
                        name: name.clone(),
                        constraint_type,
                        columns: Vec::new(),
                        referenced_table: ref_table.clone(),
                        referenced_columns: None,
                    });

            if !entry.columns.contains(col) {
                entry.columns.push(col.clone());
            }

            if let Some(rc) = ref_col {
                let ref_cols = entry.referenced_columns.get_or_insert_with(Vec::new);
                if !ref_cols.contains(rc) {
                    ref_cols.push(rc.clone());
                }
            }
        }

        Ok(constraint_map.into_values().collect())
    }
}

/// Convert a mysql_async::Row to our Row type (HashMap<String, serde_json::Value>).
fn mysql_row_to_map(row: &mysql_async::Row) -> Row {
    let mut map = std::collections::HashMap::new();

    for (i, col) in row.columns_ref().iter().enumerate() {
        let name = col.name_str().to_string();
        let value = mysql_value_to_json(row, i);
        map.insert(name, value);
    }

    map
}

/// Convert a MySQL column value at the given index to serde_json::Value.
fn mysql_value_to_json(row: &mysql_async::Row, idx: usize) -> serde_json::Value {
    use mysql_async::Value;

    match row.as_ref(idx) {
        Some(Value::NULL) | None => serde_json::Value::Null,
        Some(Value::Int(v)) => serde_json::json!(*v),
        Some(Value::UInt(v)) => serde_json::json!(*v),
        Some(Value::Float(v)) => serde_json::json!(*v),
        Some(Value::Double(v)) => serde_json::json!(*v),
        Some(Value::Bytes(b)) => {
            // Try to interpret as UTF-8 string first
            match String::from_utf8(b.clone()) {
                Ok(s) => serde_json::Value::String(s),
                Err(_) => {
                    // Fall back to base64 or hex representation
                    serde_json::Value::String(hex::encode(b))
                }
            }
        }
        Some(Value::Date(y, m, d, h, min, s, _us)) => {
            serde_json::Value::String(format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                y, m, d, h, min, s
            ))
        }
        Some(Value::Time(neg, d, h, min, s, _us)) => {
            let sign = if *neg { "-" } else { "" };
            let total_hours = *d * 24 + (*h as u32);
            serde_json::Value::String(format!(
                "{}{:02}:{:02}:{:02}",
                sign, total_hours, min, s
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_connector() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::MySql,
            host: Some("localhost".to_string()),
            port: Some(3306),
            database: Some("testdb".to_string()),
            username: Some("root".to_string()),
            password: Some("password".to_string()),
            ..Default::default()
        };
        let connector = MySqlConnector::new(config);
        assert_eq!(connector.engine(), DatabaseEngine::MySql);
        assert!(connector.pool.is_none());
    }

    #[test]
    fn test_build_opts_from_params() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::MySql,
            host: Some("myhost".to_string()),
            port: Some(3307),
            database: Some("mydb".to_string()),
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
            ..Default::default()
        };
        let connector = MySqlConnector::new(config);
        let opts = connector.build_opts().unwrap();

        assert_eq!(opts.ip_or_hostname(), "myhost");
        assert_eq!(opts.tcp_port(), 3307);
        assert_eq!(opts.db_name(), Some("mydb"));
        assert_eq!(opts.user(), Some("user"));
        assert_eq!(opts.pass(), Some("pass"));
    }

    #[test]
    fn test_build_opts_defaults() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::MySql,
            ..Default::default()
        };
        let connector = MySqlConnector::new(config);
        let opts = connector.build_opts().unwrap();

        assert_eq!(opts.ip_or_hostname(), "localhost");
        assert_eq!(opts.tcp_port(), 3306);
    }

    #[test]
    fn test_build_opts_from_connection_string() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::MySql,
            connection_string: Some("mysql://user:pass@myhost:3306/mydb".to_string()),
            ..Default::default()
        };
        let connector = MySqlConnector::new(config);
        let opts = connector.build_opts().unwrap();

        assert_eq!(opts.ip_or_hostname(), "myhost");
        assert_eq!(opts.tcp_port(), 3306);
        assert_eq!(opts.db_name(), Some("mydb"));
    }

    #[test]
    fn test_build_opts_invalid_connection_string() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::MySql,
            connection_string: Some("not-a-valid-url".to_string()),
            ..Default::default()
        };
        let connector = MySqlConnector::new(config);
        assert!(connector.build_opts().is_err());
    }

    #[test]
    fn test_database_name() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::MySql,
            database: Some("testdb".to_string()),
            ..Default::default()
        };
        let connector = MySqlConnector::new(config);
        assert_eq!(connector.database_name(), "testdb");
    }

    #[test]
    fn test_database_name_default() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::MySql,
            ..Default::default()
        };
        let connector = MySqlConnector::new(config);
        assert_eq!(connector.database_name(), "");
    }

    #[tokio::test]
    async fn test_not_connected_by_default() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::MySql,
            ..Default::default()
        };
        let connector = MySqlConnector::new(config);
        assert!(!connector.is_connected().await);
    }
}
