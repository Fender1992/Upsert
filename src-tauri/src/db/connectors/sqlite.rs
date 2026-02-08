use super::{ConnectionConfig, DatabaseConnector, DatabaseEngine};
use crate::db::schema::{
    ColumnInfo, ConstraintInfo, ConstraintType, IndexInfo, Row, SchemaInfo, TableInfo,
};
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// SQLite connector using rusqlite.
/// Since rusqlite is synchronous, all operations are wrapped with
/// `tokio::task::spawn_blocking` to avoid blocking the async runtime.
pub struct SqliteConnector {
    config: ConnectionConfig,
    conn: Option<Arc<Mutex<Connection>>>,
}

impl SqliteConnector {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            conn: None,
        }
    }

    /// Get a reference to the underlying connection.
    fn connection(&self) -> anyhow::Result<Arc<Mutex<Connection>>> {
        self.conn
            .clone()
            .ok_or_else(|| anyhow!("Not connected to SQLite"))
    }

    /// Determine the database path from config.
    fn db_path(&self) -> String {
        if let Some(ref path) = self.config.file_path {
            path.clone()
        } else if let Some(ref conn_str) = self.config.connection_string {
            conn_str.clone()
        } else {
            ":memory:".to_string()
        }
    }
}

#[async_trait]
impl DatabaseConnector for SqliteConnector {
    async fn connect(&mut self) -> anyhow::Result<()> {
        let path = self.db_path();

        let conn = tokio::task::spawn_blocking(move || {
            Connection::open(&path).context("Failed to open SQLite database")
        })
        .await
        .context("SQLite spawn_blocking join error")??;

        // Enable WAL mode for better concurrent access
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .context("Failed to set WAL mode")?;

        if self.config.read_only {
            conn.execute_batch("PRAGMA query_only=ON;")
                .context("Failed to set read-only mode")?;
        }

        self.conn = Some(Arc::new(Mutex::new(conn)));
        Ok(())
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        self.conn = None;
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        if let Some(ref conn) = self.conn {
            if let Ok(c) = conn.lock() {
                return c.execute_batch("SELECT 1").is_ok();
            }
        }
        false
    }

    async fn get_schema(&self) -> anyhow::Result<SchemaInfo> {
        let path = self.db_path();
        let db_name = std::path::Path::new(&path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("sqlite")
            .to_string();

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
        let conn = self.connection()?;

        tokio::task::spawn_blocking(move || {
            let c = conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
            let mut stmt = c
                .prepare(
                    "SELECT name FROM sqlite_master \
                     WHERE type = 'table' AND name NOT LIKE 'sqlite_%' \
                     ORDER BY name",
                )
                .context("Failed to prepare tables query")?;

            let tables: Vec<String> = stmt
                .query_map([], |row| row.get(0))
                .context("Failed to query tables")?
                .filter_map(|r| r.ok())
                .collect();

            Ok(tables)
        })
        .await
        .context("spawn_blocking join error")?
    }

    async fn get_table_info(&self, table_name: &str) -> anyhow::Result<TableInfo> {
        let columns = self.get_columns(table_name).await?;
        let indexes = self.get_indexes(table_name).await?;
        let constraints = self.get_constraints(table_name).await?;
        let row_count = self.get_row_count(table_name).await.ok();

        Ok(TableInfo {
            schema_name: "main".to_string(),
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
        let conn = self.connection()?;
        let query = query.to_string();

        tokio::task::spawn_blocking(move || {
            let c = conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
            let mut stmt = c.prepare(&query).context("Failed to prepare query")?;

            let column_names: Vec<String> = stmt
                .column_names()
                .iter()
                .map(|s| s.to_string())
                .collect();

            let rows: Vec<Row> = stmt
                .query_map([], |row| {
                    let mut map = std::collections::HashMap::new();
                    for (i, name) in column_names.iter().enumerate() {
                        let value = sqlite_value_to_json(row, i);
                        map.insert(name.clone(), value);
                    }
                    Ok(map)
                })
                .context("Failed to execute query")?
                .filter_map(|r| r.ok())
                .collect();

            Ok(rows)
        })
        .await
        .context("spawn_blocking join error")?
    }

    async fn begin_transaction(&mut self) -> anyhow::Result<()> {
        let conn = self.connection()?;
        tokio::task::spawn_blocking(move || {
            let c = conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
            c.execute_batch("BEGIN TRANSACTION")
                .context("Failed to begin transaction")
        })
        .await
        .context("spawn_blocking join error")?
    }

    async fn commit_transaction(&mut self) -> anyhow::Result<()> {
        let conn = self.connection()?;
        tokio::task::spawn_blocking(move || {
            let c = conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
            c.execute_batch("COMMIT")
                .context("Failed to commit transaction")
        })
        .await
        .context("spawn_blocking join error")?
    }

    async fn rollback_transaction(&mut self) -> anyhow::Result<()> {
        let conn = self.connection()?;
        tokio::task::spawn_blocking(move || {
            let c = conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
            c.execute_batch("ROLLBACK")
                .context("Failed to rollback transaction")
        })
        .await
        .context("spawn_blocking join error")?
    }

    fn engine(&self) -> DatabaseEngine {
        DatabaseEngine::Sqlite
    }

    async fn get_row_count(&self, table_name: &str) -> anyhow::Result<i64> {
        let conn = self.connection()?;
        let table = table_name.to_string();

        tokio::task::spawn_blocking(move || {
            let c = conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
            let query = format!(
                "SELECT COUNT(*) FROM \"{}\"",
                table.replace('"', "\"\"")
            );
            let count: i64 = c
                .query_row(&query, [], |row| row.get(0))
                .context("Failed to get row count")?;
            Ok(count)
        })
        .await
        .context("spawn_blocking join error")?
    }
}

/// Private helper methods for SQLite schema introspection.
impl SqliteConnector {
    async fn get_columns(&self, table_name: &str) -> anyhow::Result<Vec<ColumnInfo>> {
        let conn = self.connection()?;
        let table = table_name.to_string();

        tokio::task::spawn_blocking(move || {
            let c = conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

            // PRAGMA table_info returns: cid, name, type, notnull, dflt_value, pk
            let mut stmt = c
                .prepare(&format!(
                    "PRAGMA table_info(\"{}\")",
                    table.replace('"', "\"\"")
                ))
                .context("Failed to get table info")?;

            let columns: Vec<ColumnInfo> = stmt
                .query_map([], |row| {
                    let cid: i32 = row.get(0)?;
                    let name: String = row.get(1)?;
                    let data_type: String = row.get(2)?;
                    let notnull: bool = row.get(3)?;
                    let default_value: Option<String> = row.get(4)?;
                    let pk: i32 = row.get(5)?;

                    Ok(ColumnInfo {
                        name,
                        data_type,
                        is_nullable: !notnull,
                        is_primary_key: pk > 0,
                        max_length: None,
                        precision: None,
                        scale: None,
                        default_value,
                        ordinal_position: cid + 1, // PRAGMA uses 0-based cid
                    })
                })
                .context("Failed to query columns")?
                .filter_map(|r| r.ok())
                .collect();

            Ok(columns)
        })
        .await
        .context("spawn_blocking join error")?
    }

    async fn get_indexes(&self, table_name: &str) -> anyhow::Result<Vec<IndexInfo>> {
        let conn = self.connection()?;
        let table = table_name.to_string();

        tokio::task::spawn_blocking(move || {
            let c = conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

            // PRAGMA index_list returns: seq, name, unique, origin, partial
            let mut stmt = c
                .prepare(&format!(
                    "PRAGMA index_list(\"{}\")",
                    table.replace('"', "\"\"")
                ))
                .context("Failed to get index list")?;

            let index_basics: Vec<(String, bool)> = stmt
                .query_map([], |row| {
                    let name: String = row.get(1)?;
                    let unique: bool = row.get(2)?;
                    Ok((name, unique))
                })?
                .filter_map(|r| r.ok())
                .collect();

            let mut indexes = Vec::new();
            for (name, is_unique) in &index_basics {
                // PRAGMA index_info returns: seqno, cid, name
                let mut idx_stmt = c
                    .prepare(&format!(
                        "PRAGMA index_info(\"{}\")",
                        name.replace('"', "\"\"")
                    ))
                    .context("Failed to get index info")?;

                let columns: Vec<String> = idx_stmt
                    .query_map([], |row| {
                        let col_name: String = row.get(2)?;
                        Ok(col_name)
                    })?
                    .filter_map(|r| r.ok())
                    .collect();

                indexes.push(IndexInfo {
                    name: name.clone(),
                    columns,
                    is_unique: *is_unique,
                    is_clustered: false, // SQLite does not have clustered indexes
                    index_type: "BTREE".to_string(),
                });
            }

            Ok(indexes)
        })
        .await
        .context("spawn_blocking join error")?
    }

    async fn get_constraints(&self, table_name: &str) -> anyhow::Result<Vec<ConstraintInfo>> {
        let conn = self.connection()?;
        let table = table_name.to_string();

        tokio::task::spawn_blocking(move || {
            let c = conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
            let mut constraints = Vec::new();

            // Get primary key from table_info
            let mut stmt = c
                .prepare(&format!(
                    "PRAGMA table_info(\"{}\")",
                    table.replace('"', "\"\"")
                ))
                .context("Failed to get table info for constraints")?;

            let pk_columns: Vec<String> = stmt
                .query_map([], |row| {
                    let name: String = row.get(1)?;
                    let pk: i32 = row.get(5)?;
                    Ok((name, pk))
                })?
                .filter_map(|r| r.ok())
                .filter(|(_, pk)| *pk > 0)
                .map(|(name, _)| name)
                .collect();

            if !pk_columns.is_empty() {
                constraints.push(ConstraintInfo {
                    name: format!("{}_pk", table),
                    constraint_type: ConstraintType::PrimaryKey,
                    columns: pk_columns,
                    referenced_table: None,
                    referenced_columns: None,
                });
            }

            // Get foreign keys
            let mut fk_stmt = c
                .prepare(&format!(
                    "PRAGMA foreign_key_list(\"{}\")",
                    table.replace('"', "\"\"")
                ))
                .context("Failed to get foreign keys")?;

            // PRAGMA foreign_key_list returns: id, seq, table, from, to, on_update, on_delete, match
            let fk_rows: Vec<(i32, String, String, String)> = fk_stmt
                .query_map([], |row| {
                    let id: i32 = row.get(0)?;
                    let ref_table: String = row.get(2)?;
                    let from_col: String = row.get(3)?;
                    let to_col: String = row.get(4)?;
                    Ok((id, ref_table, from_col, to_col))
                })?
                .filter_map(|r| r.ok())
                .collect();

            // Group foreign keys by id
            let mut fk_map: std::collections::HashMap<i32, ConstraintInfo> =
                std::collections::HashMap::new();

            for (id, ref_table, from_col, to_col) in &fk_rows {
                let entry = fk_map.entry(*id).or_insert_with(|| ConstraintInfo {
                    name: format!("{}_fk_{}", table, id),
                    constraint_type: ConstraintType::ForeignKey,
                    columns: Vec::new(),
                    referenced_table: Some(ref_table.clone()),
                    referenced_columns: Some(Vec::new()),
                });
                entry.columns.push(from_col.clone());
                if let Some(ref mut ref_cols) = entry.referenced_columns {
                    ref_cols.push(to_col.clone());
                }
            }

            constraints.extend(fk_map.into_values());

            Ok(constraints)
        })
        .await
        .context("spawn_blocking join error")?
    }
}

/// Convert a rusqlite column value to serde_json::Value.
fn sqlite_value_to_json(row: &rusqlite::Row<'_>, idx: usize) -> serde_json::Value {
    // Try types in order: integer, real, text, blob, null
    if let Ok(v) = row.get::<_, i64>(idx) {
        return serde_json::json!(v);
    }
    if let Ok(v) = row.get::<_, f64>(idx) {
        return serde_json::json!(v);
    }
    if let Ok(v) = row.get::<_, String>(idx) {
        return serde_json::Value::String(v);
    }
    if let Ok(v) = row.get::<_, Vec<u8>>(idx) {
        return serde_json::Value::String(hex::encode(v));
    }
    serde_json::Value::Null
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_connector() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::Sqlite,
            file_path: Some("/tmp/test.db".to_string()),
            ..Default::default()
        };
        let connector = SqliteConnector::new(config);
        assert_eq!(connector.engine(), DatabaseEngine::Sqlite);
        assert!(connector.conn.is_none());
    }

    #[test]
    fn test_db_path_from_file_path() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::Sqlite,
            file_path: Some("/data/mydb.sqlite".to_string()),
            ..Default::default()
        };
        let connector = SqliteConnector::new(config);
        assert_eq!(connector.db_path(), "/data/mydb.sqlite");
    }

    #[test]
    fn test_db_path_from_connection_string() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::Sqlite,
            connection_string: Some("/data/mydb.db".to_string()),
            ..Default::default()
        };
        let connector = SqliteConnector::new(config);
        assert_eq!(connector.db_path(), "/data/mydb.db");
    }

    #[test]
    fn test_db_path_defaults_to_memory() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::Sqlite,
            ..Default::default()
        };
        let connector = SqliteConnector::new(config);
        assert_eq!(connector.db_path(), ":memory:");
    }

    #[test]
    fn test_file_path_takes_priority() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::Sqlite,
            file_path: Some("/data/primary.db".to_string()),
            connection_string: Some("/data/fallback.db".to_string()),
            ..Default::default()
        };
        let connector = SqliteConnector::new(config);
        assert_eq!(connector.db_path(), "/data/primary.db");
    }

    #[tokio::test]
    async fn test_connect_in_memory() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::Sqlite,
            read_only: false,
            ..Default::default()
        };
        let mut connector = SqliteConnector::new(config);
        connector.connect().await.unwrap();
        assert!(connector.is_connected().await);
    }

    #[tokio::test]
    async fn test_disconnect() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::Sqlite,
            read_only: false,
            ..Default::default()
        };
        let mut connector = SqliteConnector::new(config);
        connector.connect().await.unwrap();
        connector.disconnect().await.unwrap();
        assert!(!connector.is_connected().await);
    }

    #[tokio::test]
    async fn test_create_table_and_query() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::Sqlite,
            read_only: false,
            ..Default::default()
        };
        let mut connector = SqliteConnector::new(config);
        connector.connect().await.unwrap();

        // Create a table
        connector
            .execute_query("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT, value REAL)")
            .await
            .unwrap();

        // Insert rows
        connector
            .execute_query("INSERT INTO test VALUES (1, 'hello', 1.5)")
            .await
            .unwrap();
        connector
            .execute_query("INSERT INTO test VALUES (2, 'world', 2.5)")
            .await
            .unwrap();

        // Query rows
        let rows = connector.get_rows("test", None, None).await.unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name"], serde_json::json!("hello"));
        assert_eq!(rows[1]["value"], serde_json::json!(2.5));
    }

    #[tokio::test]
    async fn test_get_tables() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::Sqlite,
            read_only: false,
            ..Default::default()
        };
        let mut connector = SqliteConnector::new(config);
        connector.connect().await.unwrap();

        connector
            .execute_query("CREATE TABLE alpha (id INTEGER PRIMARY KEY)")
            .await
            .unwrap();
        connector
            .execute_query("CREATE TABLE beta (id INTEGER PRIMARY KEY)")
            .await
            .unwrap();

        let tables = connector.get_tables().await.unwrap();
        assert_eq!(tables, vec!["alpha", "beta"]);
    }

    #[tokio::test]
    async fn test_get_table_info() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::Sqlite,
            read_only: false,
            ..Default::default()
        };
        let mut connector = SqliteConnector::new(config);
        connector.connect().await.unwrap();

        connector
            .execute_query(
                "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, email TEXT)",
            )
            .await
            .unwrap();

        let info = connector.get_table_info("users").await.unwrap();
        assert_eq!(info.table_name, "users");
        assert_eq!(info.columns.len(), 3);

        let id_col = &info.columns[0];
        assert_eq!(id_col.name, "id");
        assert!(id_col.is_primary_key);
        assert_eq!(id_col.data_type, "INTEGER");

        let name_col = &info.columns[1];
        assert_eq!(name_col.name, "name");
        assert!(!name_col.is_nullable);
    }

    #[tokio::test]
    async fn test_get_row_count() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::Sqlite,
            read_only: false,
            ..Default::default()
        };
        let mut connector = SqliteConnector::new(config);
        connector.connect().await.unwrap();

        connector
            .execute_query("CREATE TABLE items (id INTEGER PRIMARY KEY)")
            .await
            .unwrap();
        connector
            .execute_query("INSERT INTO items VALUES (1)")
            .await
            .unwrap();
        connector
            .execute_query("INSERT INTO items VALUES (2)")
            .await
            .unwrap();
        connector
            .execute_query("INSERT INTO items VALUES (3)")
            .await
            .unwrap();

        let count = connector.get_row_count("items").await.unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_get_rows_with_limit_offset() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::Sqlite,
            read_only: false,
            ..Default::default()
        };
        let mut connector = SqliteConnector::new(config);
        connector.connect().await.unwrap();

        connector
            .execute_query("CREATE TABLE nums (val INTEGER)")
            .await
            .unwrap();
        for i in 1..=10 {
            connector
                .execute_query(&format!("INSERT INTO nums VALUES ({})", i))
                .await
                .unwrap();
        }

        let rows = connector.get_rows("nums", Some(3), Some(2)).await.unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0]["val"], serde_json::json!(3));
        assert_eq!(rows[2]["val"], serde_json::json!(5));
    }

    #[tokio::test]
    async fn test_transaction_commit() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::Sqlite,
            read_only: false,
            ..Default::default()
        };
        let mut connector = SqliteConnector::new(config);
        connector.connect().await.unwrap();

        connector
            .execute_query("CREATE TABLE txn_test (id INTEGER)")
            .await
            .unwrap();

        connector.begin_transaction().await.unwrap();
        connector
            .execute_query("INSERT INTO txn_test VALUES (1)")
            .await
            .unwrap();
        connector.commit_transaction().await.unwrap();

        let count = connector.get_row_count("txn_test").await.unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_transaction_rollback() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::Sqlite,
            read_only: false,
            ..Default::default()
        };
        let mut connector = SqliteConnector::new(config);
        connector.connect().await.unwrap();

        connector
            .execute_query("CREATE TABLE txn_test2 (id INTEGER)")
            .await
            .unwrap();

        connector.begin_transaction().await.unwrap();
        connector
            .execute_query("INSERT INTO txn_test2 VALUES (1)")
            .await
            .unwrap();
        connector.rollback_transaction().await.unwrap();

        let count = connector.get_row_count("txn_test2").await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_schema_introspection_with_foreign_key() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::Sqlite,
            read_only: false,
            ..Default::default()
        };
        let mut connector = SqliteConnector::new(config);
        connector.connect().await.unwrap();

        // Enable foreign keys
        connector
            .execute_query("PRAGMA foreign_keys = ON")
            .await
            .unwrap();

        connector
            .execute_query("CREATE TABLE parent (id INTEGER PRIMARY KEY, name TEXT)")
            .await
            .unwrap();
        connector
            .execute_query(
                "CREATE TABLE child (id INTEGER PRIMARY KEY, parent_id INTEGER, \
                 FOREIGN KEY(parent_id) REFERENCES parent(id))",
            )
            .await
            .unwrap();

        let info = connector.get_table_info("child").await.unwrap();

        // Should have a primary key constraint
        let pk = info
            .constraints
            .iter()
            .find(|c| c.constraint_type == ConstraintType::PrimaryKey);
        assert!(pk.is_some());

        // Should have a foreign key constraint
        let fk = info
            .constraints
            .iter()
            .find(|c| c.constraint_type == ConstraintType::ForeignKey);
        assert!(fk.is_some());
        let fk = fk.unwrap();
        assert_eq!(fk.referenced_table.as_deref(), Some("parent"));
        assert_eq!(fk.columns, vec!["parent_id"]);
    }

    #[tokio::test]
    async fn test_get_indexes() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::Sqlite,
            read_only: false,
            ..Default::default()
        };
        let mut connector = SqliteConnector::new(config);
        connector.connect().await.unwrap();

        connector
            .execute_query("CREATE TABLE indexed (id INTEGER PRIMARY KEY, name TEXT, email TEXT)")
            .await
            .unwrap();
        connector
            .execute_query("CREATE INDEX idx_name ON indexed(name)")
            .await
            .unwrap();
        connector
            .execute_query("CREATE UNIQUE INDEX idx_email ON indexed(email)")
            .await
            .unwrap();

        let info = connector.get_table_info("indexed").await.unwrap();

        let name_idx = info.indexes.iter().find(|i| i.name == "idx_name");
        assert!(name_idx.is_some());
        assert!(!name_idx.unwrap().is_unique);

        let email_idx = info.indexes.iter().find(|i| i.name == "idx_email");
        assert!(email_idx.is_some());
        assert!(email_idx.unwrap().is_unique);
    }
}
