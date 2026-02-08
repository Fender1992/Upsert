use super::{ConnectionConfig, DatabaseConnector, DatabaseEngine};
use crate::db::schema::{
    ColumnInfo, ConstraintInfo, ConstraintType, IndexInfo, Row, SchemaInfo, TableInfo,
};
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use tiberius::{AuthMethod, Client, Config, EncryptionLevel};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

/// SQL Server connector using tiberius
pub struct SqlServerConnector {
    config: ConnectionConfig,
    client: Mutex<Option<Client<Compat<TcpStream>>>>,
}

impl SqlServerConnector {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            client: Mutex::new(None),
        }
    }

    /// Build a tiberius Config from our ConnectionConfig and return (Config, host, port)
    fn build_tiberius_config(&self) -> anyhow::Result<(Config, String, u16)> {
        if let Some(ref conn_str) = self.config.connection_string {
            let config = Config::from_ado_string(conn_str)
                .context("Failed to parse SQL Server connection string")?;
            // Parse host/port from the connection string for TCP connection
            let host = self.config.host.clone().unwrap_or_else(|| "localhost".to_string());
            let port = self.config.port.unwrap_or(1433);
            return Ok((config, host, port));
        }

        let mut tib_config = Config::new();

        let host = self.config.host.clone().unwrap_or_else(|| "localhost".to_string());
        let port = self.config.port.unwrap_or(1433);

        tib_config.host(&host);
        tib_config.port(port);

        if let Some(ref db) = self.config.database {
            tib_config.database(db);
        }

        match (&self.config.username, &self.config.password) {
            (Some(user), Some(pass)) => {
                tib_config.authentication(AuthMethod::sql_server(user, pass));
            }
            _ => {
                // Windows authentication - use AAD token or no auth
                tib_config.authentication(AuthMethod::None);
            }
        }

        // Trust server certificate by default for development
        tib_config.trust_cert();
        tib_config.encryption(EncryptionLevel::Required);

        Ok((tib_config, host, port))
    }

    /// Convert a tiberius Row into our Row type (HashMap<String, Value>)
    fn row_to_map(row: &tiberius::Row) -> Row {
        let mut map = std::collections::HashMap::new();
        for col in row.columns() {
            let name = col.name().to_string();
            let value = Self::column_to_json(row, col);
            map.insert(name, value);
        }
        map
    }

    /// Convert a single tiberius column value to serde_json::Value
    fn column_to_json(row: &tiberius::Row, col: &tiberius::Column) -> serde_json::Value {
        use tiberius::ColumnType;

        match col.column_type() {
            ColumnType::Null => serde_json::Value::Null,
            ColumnType::Bit | ColumnType::Bitn => match row.try_get::<bool, _>(col.name()) {
                Ok(Some(v)) => serde_json::Value::Bool(v),
                _ => serde_json::Value::Null,
            },
            ColumnType::Int1 => match row.try_get::<u8, _>(col.name()) {
                Ok(Some(v)) => serde_json::json!(v),
                _ => serde_json::Value::Null,
            },
            ColumnType::Int2 => match row.try_get::<i16, _>(col.name()) {
                Ok(Some(v)) => serde_json::json!(v),
                _ => serde_json::Value::Null,
            },
            ColumnType::Int4 => match row.try_get::<i32, _>(col.name()) {
                Ok(Some(v)) => serde_json::json!(v),
                _ => serde_json::Value::Null,
            },
            ColumnType::Int8 => match row.try_get::<i64, _>(col.name()) {
                Ok(Some(v)) => serde_json::json!(v),
                _ => serde_json::Value::Null,
            },
            ColumnType::Intn => {
                if let Ok(Some(v)) = row.try_get::<i64, _>(col.name()) {
                    serde_json::json!(v)
                } else if let Ok(Some(v)) = row.try_get::<i32, _>(col.name()) {
                    serde_json::json!(v)
                } else if let Ok(Some(v)) = row.try_get::<i16, _>(col.name()) {
                    serde_json::json!(v)
                } else {
                    serde_json::Value::Null
                }
            }
            ColumnType::Float4 => match row.try_get::<f32, _>(col.name()) {
                Ok(Some(v)) => serde_json::json!(v),
                _ => serde_json::Value::Null,
            },
            ColumnType::Float8 => match row.try_get::<f64, _>(col.name()) {
                Ok(Some(v)) => serde_json::json!(v),
                _ => serde_json::Value::Null,
            },
            ColumnType::Floatn => {
                if let Ok(Some(v)) = row.try_get::<f64, _>(col.name()) {
                    serde_json::json!(v)
                } else if let Ok(Some(v)) = row.try_get::<f32, _>(col.name()) {
                    serde_json::json!(v)
                } else {
                    serde_json::Value::Null
                }
            }
            ColumnType::Numericn | ColumnType::Decimaln => {
                match row.try_get::<f64, _>(col.name()) {
                    Ok(Some(v)) => serde_json::json!(v),
                    _ => match row.try_get::<&str, _>(col.name()) {
                        Ok(Some(v)) => serde_json::Value::String(v.to_string()),
                        _ => serde_json::Value::Null,
                    },
                }
            }
            _ => {
                // For all string/text/date/binary/guid types, try as string first
                match row.try_get::<&str, _>(col.name()) {
                    Ok(Some(v)) => serde_json::Value::String(v.to_string()),
                    _ => serde_json::Value::Null,
                }
            }
        }
    }
}

#[async_trait]
impl DatabaseConnector for SqlServerConnector {
    async fn connect(&mut self) -> anyhow::Result<()> {
        let (tib_config, host, port) = self.build_tiberius_config()?;
        let addr = format!("{}:{}", host, port);

        let tcp = TcpStream::connect(&addr)
            .await
            .context(format!("Failed to connect to SQL Server at {}", addr))?;
        tcp.set_nodelay(true)?;

        let client = Client::connect(tib_config, tcp.compat_write())
            .await
            .context("TDS connection/authentication failed")?;

        *self.client.lock().await = Some(client);
        Ok(())
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        *self.client.lock().await = None;
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        self.client.lock().await.is_some()
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
        let query = "SELECT TABLE_SCHEMA, TABLE_NAME FROM INFORMATION_SCHEMA.TABLES \
                     WHERE TABLE_TYPE = 'BASE TABLE' \
                     ORDER BY TABLE_SCHEMA, TABLE_NAME";

        let mut guard = self.client.lock().await;
        let client = guard
            .as_mut()
            .ok_or_else(|| anyhow!("Not connected to SQL Server"))?;

        let stream = client
            .simple_query(query)
            .await
            .context("Failed to query tables")?;
        let rows = stream
            .into_first_result()
            .await
            .context("Failed to read table results")?;

        let mut tables = Vec::new();
        for row in &rows {
            let schema: &str = row.try_get(0)?.unwrap_or("dbo");
            let name: &str = row.try_get(1)?.unwrap_or("");
            if schema == "dbo" {
                tables.push(name.to_string());
            } else {
                tables.push(format!("{}.{}", schema, name));
            }
        }
        Ok(tables)
    }

    async fn get_table_info(&self, table_name: &str) -> anyhow::Result<TableInfo> {
        let (schema_name, bare_table) = if table_name.contains('.') {
            let parts: Vec<&str> = table_name.splitn(2, '.').collect();
            (parts[0].to_string(), parts[1].to_string())
        } else {
            ("dbo".to_string(), table_name.to_string())
        };

        let columns = self.get_columns(&schema_name, &bare_table).await?;
        let indexes = self.get_indexes(&schema_name, &bare_table).await?;
        let constraints = self.get_constraints(&schema_name, &bare_table).await?;
        let row_count = self.get_row_count(table_name).await.ok();

        Ok(TableInfo {
            schema_name,
            table_name: bare_table,
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
            "SELECT * FROM [{}] ORDER BY (SELECT NULL) OFFSET {} ROWS FETCH NEXT {} ROWS ONLY",
            table_name.replace(']', "]]"),
            offset,
            limit
        );

        self.execute_query(&query).await
    }

    async fn execute_query(&self, query: &str) -> anyhow::Result<Vec<Row>> {
        let mut guard = self.client.lock().await;
        let client = guard
            .as_mut()
            .ok_or_else(|| anyhow!("Not connected to SQL Server"))?;

        let stream = client
            .simple_query(query)
            .await
            .context("Failed to execute query")?;
        let rows = stream
            .into_first_result()
            .await
            .context("Failed to read query results")?;

        let mut result = Vec::new();
        for row in &rows {
            result.push(Self::row_to_map(row));
        }
        Ok(result)
    }

    async fn begin_transaction(&mut self) -> anyhow::Result<()> {
        let mut guard = self.client.lock().await;
        let client = guard
            .as_mut()
            .ok_or_else(|| anyhow!("Not connected to SQL Server"))?;
        client
            .simple_query("BEGIN TRANSACTION")
            .await
            .context("Failed to begin transaction")?
            .into_results()
            .await?;
        Ok(())
    }

    async fn commit_transaction(&mut self) -> anyhow::Result<()> {
        let mut guard = self.client.lock().await;
        let client = guard
            .as_mut()
            .ok_or_else(|| anyhow!("Not connected to SQL Server"))?;
        client
            .simple_query("COMMIT TRANSACTION")
            .await
            .context("Failed to commit transaction")?
            .into_results()
            .await?;
        Ok(())
    }

    async fn rollback_transaction(&mut self) -> anyhow::Result<()> {
        let mut guard = self.client.lock().await;
        let client = guard
            .as_mut()
            .ok_or_else(|| anyhow!("Not connected to SQL Server"))?;
        client
            .simple_query("ROLLBACK TRANSACTION")
            .await
            .context("Failed to rollback transaction")?
            .into_results()
            .await?;
        Ok(())
    }

    fn engine(&self) -> DatabaseEngine {
        DatabaseEngine::SqlServer
    }

    async fn get_row_count(&self, table_name: &str) -> anyhow::Result<i64> {
        let query = format!(
            "SELECT COUNT(*) AS cnt FROM [{}]",
            table_name.replace(']', "]]")
        );

        let mut guard = self.client.lock().await;
        let client = guard
            .as_mut()
            .ok_or_else(|| anyhow!("Not connected to SQL Server"))?;

        let stream = client
            .simple_query(&query)
            .await
            .context("Failed to query row count")?;
        let rows = stream
            .into_first_result()
            .await
            .context("Failed to read row count")?;

        let row = rows
            .first()
            .ok_or_else(|| anyhow!("No result from COUNT query"))?;
        let count: i32 = row.try_get(0)?.unwrap_or(0);
        Ok(count as i64)
    }
}

/// Private helper methods for schema introspection
impl SqlServerConnector {
    async fn get_columns(
        &self,
        schema_name: &str,
        table_name: &str,
    ) -> anyhow::Result<Vec<ColumnInfo>> {
        let mut guard = self.client.lock().await;
        let client = guard
            .as_mut()
            .ok_or_else(|| anyhow!("Not connected to SQL Server"))?;

        // Get primary key columns first
        let pk_query = format!(
            "SELECT kcu.COLUMN_NAME \
             FROM INFORMATION_SCHEMA.TABLE_CONSTRAINTS tc \
             JOIN INFORMATION_SCHEMA.KEY_COLUMN_USAGE kcu \
               ON tc.CONSTRAINT_NAME = kcu.CONSTRAINT_NAME \
               AND tc.TABLE_SCHEMA = kcu.TABLE_SCHEMA \
             WHERE tc.CONSTRAINT_TYPE = 'PRIMARY KEY' \
               AND tc.TABLE_SCHEMA = '{}' \
               AND tc.TABLE_NAME = '{}'",
            schema_name.replace('\'', "''"),
            table_name.replace('\'', "''")
        );

        let pk_stream = client.simple_query(&pk_query).await?;
        let pk_rows = pk_stream.into_first_result().await?;
        let pk_columns: Vec<String> = pk_rows
            .iter()
            .filter_map(|r| r.try_get::<&str, _>(0).ok().flatten().map(|s| s.to_string()))
            .collect();

        // Get column info
        let col_query = format!(
            "SELECT COLUMN_NAME, DATA_TYPE, IS_NULLABLE, \
                    CHARACTER_MAXIMUM_LENGTH, NUMERIC_PRECISION, NUMERIC_SCALE, \
                    COLUMN_DEFAULT, ORDINAL_POSITION \
             FROM INFORMATION_SCHEMA.COLUMNS \
             WHERE TABLE_SCHEMA = '{}' AND TABLE_NAME = '{}' \
             ORDER BY ORDINAL_POSITION",
            schema_name.replace('\'', "''"),
            table_name.replace('\'', "''")
        );

        let col_stream = client.simple_query(&col_query).await?;
        let col_rows = col_stream.into_first_result().await?;

        let mut columns = Vec::new();
        for row in &col_rows {
            let name: &str = row.try_get(0)?.unwrap_or("");
            let data_type: &str = row.try_get(1)?.unwrap_or("");
            let nullable_str: &str = row.try_get(2)?.unwrap_or("YES");
            let max_length: Option<i32> = row.try_get(3)?;
            let precision: Option<u8> = row.try_get(4)?;
            let scale: Option<i32> = row.try_get(5)?;
            let default_value: Option<&str> = row.try_get(6)?;
            let ordinal: i32 = row.try_get(7)?.unwrap_or(0);

            columns.push(ColumnInfo {
                name: name.to_string(),
                data_type: data_type.to_string(),
                is_nullable: nullable_str == "YES",
                is_primary_key: pk_columns.contains(&name.to_string()),
                max_length,
                precision: precision.map(|v| v as i32),
                scale,
                default_value: default_value.map(|s| s.to_string()),
                ordinal_position: ordinal,
            });
        }

        Ok(columns)
    }

    async fn get_indexes(
        &self,
        schema_name: &str,
        table_name: &str,
    ) -> anyhow::Result<Vec<IndexInfo>> {
        let mut guard = self.client.lock().await;
        let client = guard
            .as_mut()
            .ok_or_else(|| anyhow!("Not connected to SQL Server"))?;

        let query = format!(
            "SELECT i.name AS index_name, \
                    COL_NAME(ic.object_id, ic.column_id) AS column_name, \
                    i.is_unique, \
                    i.type_desc \
             FROM sys.indexes i \
             JOIN sys.index_columns ic ON i.object_id = ic.object_id AND i.index_id = ic.index_id \
             JOIN sys.tables t ON i.object_id = t.object_id \
             JOIN sys.schemas s ON t.schema_id = s.schema_id \
             WHERE s.name = '{}' AND t.name = '{}' AND i.name IS NOT NULL \
             ORDER BY i.name, ic.key_ordinal",
            schema_name.replace('\'', "''"),
            table_name.replace('\'', "''")
        );

        let stream = client.simple_query(&query).await?;
        let rows = stream.into_first_result().await?;

        let mut indexes: std::collections::HashMap<String, IndexInfo> =
            std::collections::HashMap::new();

        for row in &rows {
            let idx_name: &str = row.try_get(0)?.unwrap_or("");
            let col_name: &str = row.try_get(1)?.unwrap_or("");
            let is_unique: bool = row.try_get(2)?.unwrap_or(false);
            let type_desc: &str = row.try_get(3)?.unwrap_or("");

            let entry = indexes.entry(idx_name.to_string()).or_insert_with(|| IndexInfo {
                name: idx_name.to_string(),
                columns: Vec::new(),
                is_unique,
                is_clustered: type_desc.contains("CLUSTERED")
                    && !type_desc.contains("NONCLUSTERED"),
                index_type: type_desc.to_string(),
            });
            entry.columns.push(col_name.to_string());
        }

        Ok(indexes.into_values().collect())
    }

    async fn get_constraints(
        &self,
        schema_name: &str,
        table_name: &str,
    ) -> anyhow::Result<Vec<ConstraintInfo>> {
        let mut guard = self.client.lock().await;
        let client = guard
            .as_mut()
            .ok_or_else(|| anyhow!("Not connected to SQL Server"))?;

        let query = format!(
            "SELECT tc.CONSTRAINT_NAME, tc.CONSTRAINT_TYPE, \
                    kcu.COLUMN_NAME, \
                    ccu.TABLE_NAME AS referenced_table, \
                    ccu.COLUMN_NAME AS referenced_column \
             FROM INFORMATION_SCHEMA.TABLE_CONSTRAINTS tc \
             JOIN INFORMATION_SCHEMA.KEY_COLUMN_USAGE kcu \
               ON tc.CONSTRAINT_NAME = kcu.CONSTRAINT_NAME \
               AND tc.TABLE_SCHEMA = kcu.TABLE_SCHEMA \
             LEFT JOIN INFORMATION_SCHEMA.REFERENTIAL_CONSTRAINTS rc \
               ON tc.CONSTRAINT_NAME = rc.CONSTRAINT_NAME \
             LEFT JOIN INFORMATION_SCHEMA.CONSTRAINT_COLUMN_USAGE ccu \
               ON rc.UNIQUE_CONSTRAINT_NAME = ccu.CONSTRAINT_NAME \
             WHERE tc.TABLE_SCHEMA = '{}' AND tc.TABLE_NAME = '{}' \
             ORDER BY tc.CONSTRAINT_NAME, kcu.ORDINAL_POSITION",
            schema_name.replace('\'', "''"),
            table_name.replace('\'', "''")
        );

        let stream = client.simple_query(&query).await?;
        let rows = stream.into_first_result().await?;

        let mut constraints: std::collections::HashMap<String, ConstraintInfo> =
            std::collections::HashMap::new();

        for row in &rows {
            let name: &str = row.try_get(0)?.unwrap_or("");
            let ctype: &str = row.try_get(1)?.unwrap_or("");
            let col: &str = row.try_get(2)?.unwrap_or("");
            let ref_table: Option<&str> = row.try_get(3)?;
            let ref_col: Option<&str> = row.try_get(4)?;

            let constraint_type = match ctype {
                "PRIMARY KEY" => ConstraintType::PrimaryKey,
                "FOREIGN KEY" => ConstraintType::ForeignKey,
                "UNIQUE" => ConstraintType::Unique,
                "CHECK" => ConstraintType::Check,
                _ => ConstraintType::Default,
            };

            let entry = constraints
                .entry(name.to_string())
                .or_insert_with(|| ConstraintInfo {
                    name: name.to_string(),
                    constraint_type,
                    columns: Vec::new(),
                    referenced_table: ref_table.map(|s| s.to_string()),
                    referenced_columns: None,
                });

            if !entry.columns.contains(&col.to_string()) {
                entry.columns.push(col.to_string());
            }

            if let Some(rc) = ref_col {
                let ref_cols = entry.referenced_columns.get_or_insert_with(Vec::new);
                if !ref_cols.contains(&rc.to_string()) {
                    ref_cols.push(rc.to_string());
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
            engine: DatabaseEngine::SqlServer,
            host: Some("localhost".to_string()),
            port: Some(1433),
            database: Some("testdb".to_string()),
            username: Some("sa".to_string()),
            password: Some("password".to_string()),
            ..Default::default()
        };
        let connector = SqlServerConnector::new(config);
        assert_eq!(connector.engine(), DatabaseEngine::SqlServer);
    }

    #[test]
    fn test_build_config_from_params() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::SqlServer,
            host: Some("myserver".to_string()),
            port: Some(1434),
            database: Some("mydb".to_string()),
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
            ..Default::default()
        };
        let connector = SqlServerConnector::new(config);
        let (_tib_config, host, port) = connector.build_tiberius_config().unwrap();
        assert_eq!(host, "myserver");
        assert_eq!(port, 1434);
    }

    #[test]
    fn test_build_config_defaults() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::SqlServer,
            ..Default::default()
        };
        let connector = SqlServerConnector::new(config);
        let (_tib_config, host, port) = connector.build_tiberius_config().unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 1433);
    }

    #[test]
    fn test_build_config_from_connection_string() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::SqlServer,
            host: Some("myserver".to_string()),
            connection_string: Some(
                "Server=tcp:myserver,1433;Database=mydb;User Id=sa;Password=pass;".to_string(),
            ),
            ..Default::default()
        };
        let connector = SqlServerConnector::new(config);
        let (_tib_config, host, _port) = connector.build_tiberius_config().unwrap();
        assert_eq!(host, "myserver");
    }

    #[tokio::test]
    async fn test_not_connected_by_default() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::SqlServer,
            ..Default::default()
        };
        let connector = SqlServerConnector::new(config);
        assert!(!connector.is_connected().await);
    }

    #[test]
    fn test_engine_returns_sqlserver() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::SqlServer,
            ..Default::default()
        };
        let connector = SqlServerConnector::new(config);
        assert_eq!(connector.engine(), DatabaseEngine::SqlServer);
    }
}
