use anyhow::{anyhow, Context};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const SCHEMA_SQL: &str = include_str!("../appdb/schema.sql");

/// Embedded app database for persistent state.
pub struct AppDatabase {
    conn: Arc<Mutex<Connection>>,
}

// ── DTOs ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProfileRow {
    pub id: String,
    pub name: String,
    pub engine: String,
    pub host: Option<String>,
    pub port: Option<i64>,
    pub database_name: Option<String>,
    pub username: Option<String>,
    pub file_path: Option<String>,
    pub read_only: bool,
    pub credential_key: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationHistoryRow {
    pub id: String,
    pub source_connection_id: Option<String>,
    pub target_connection_id: Option<String>,
    pub mode: String,
    pub status: String,
    pub config_json: Option<String>,
    pub result_json: Option<String>,
    pub error: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub rows_inserted: i64,
    pub rows_updated: i64,
    pub rows_deleted: i64,
    pub rows_skipped: i64,
    pub error_count: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessageRow {
    pub id: String,
    pub role: String,
    pub content: String,
    pub model: Option<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEntryRow {
    pub id: String,
    pub timestamp: String,
    pub user_name: Option<String>,
    pub action: String,
    pub source_connection: Option<String>,
    pub target_connection: Option<String>,
    pub affected_rows: Option<i64>,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextChunkRow {
    pub id: String,
    pub connection_id: Option<String>,
    pub chunk_type: String,
    pub label: String,
    pub content: String,
    #[serde(skip)]
    pub embedding: Option<Vec<f32>>,
    pub model: Option<String>,
    pub created_at: String,
}

/// Cosine similarity between two vectors. Returns 0.0 if either has zero magnitude.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0_f32;
    let mut mag_a = 0.0_f32;
    let mut mag_b = 0.0_f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        mag_a += a[i] * a[i];
        mag_b += b[i] * b[i];
    }
    let denom = mag_a.sqrt() * mag_b.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

fn embedding_to_blob(embedding: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(embedding.len() * 4);
    for &val in embedding {
        bytes.extend_from_slice(&val.to_le_bytes());
    }
    bytes
}

fn blob_to_embedding(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

// ── Implementation ──────────────────────────────────────────────────────

impl AppDatabase {
    /// Initialize the app database at the given data directory.
    /// Creates the DB file if it doesn't exist, then runs schema DDL.
    pub fn init(app_data_dir: PathBuf) -> anyhow::Result<Self> {
        std::fs::create_dir_all(&app_data_dir)
            .context("Failed to create app data directory")?;

        let db_path = app_data_dir.join("upsert.db");

        let conn = Connection::open(&db_path)
            .context("Failed to open app database")?;

        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .context("Failed to set WAL mode")?;

        conn.execute_batch(SCHEMA_SQL)
            .context("Failed to apply schema")?;

        log::info!("App database initialized at {:?}", db_path);

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    // ── Connections ─────────────────────────────────────────────────────

    pub async fn save_connection(&self, profile: ConnectionProfileRow) -> anyhow::Result<()> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let c = conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
            c.execute(
                "INSERT OR REPLACE INTO connections \
                 (id, name, engine, host, port, database_name, username, file_path, read_only, credential_key, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                rusqlite::params![
                    profile.id,
                    profile.name,
                    profile.engine,
                    profile.host,
                    profile.port,
                    profile.database_name,
                    profile.username,
                    profile.file_path,
                    profile.read_only as i32,
                    profile.credential_key,
                    profile.created_at,
                    profile.updated_at,
                ],
            ).context("Failed to save connection")?;
            Ok(())
        })
        .await
        .context("spawn_blocking join error")?
    }

    pub async fn load_connections(&self) -> anyhow::Result<Vec<ConnectionProfileRow>> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let c = conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
            let mut stmt = c
                .prepare(
                    "SELECT id, name, engine, host, port, database_name, username, \
                     file_path, read_only, credential_key, created_at, updated_at \
                     FROM connections ORDER BY name",
                )
                .context("Failed to prepare connections query")?;

            let rows = stmt
                .query_map([], |row| {
                    let read_only_int: i32 = row.get(8)?;
                    Ok(ConnectionProfileRow {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        engine: row.get(2)?,
                        host: row.get(3)?,
                        port: row.get(4)?,
                        database_name: row.get(5)?,
                        username: row.get(6)?,
                        file_path: row.get(7)?,
                        read_only: read_only_int != 0,
                        credential_key: row.get(9)?,
                        created_at: row.get(10)?,
                        updated_at: row.get(11)?,
                    })
                })
                .context("Failed to query connections")?
                .filter_map(|r| r.ok())
                .collect();

            Ok(rows)
        })
        .await
        .context("spawn_blocking join error")?
    }

    pub async fn delete_connection(&self, id: String) -> anyhow::Result<()> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let c = conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
            c.execute("DELETE FROM connections WHERE id = ?1", rusqlite::params![id])
                .context("Failed to delete connection")?;
            Ok(())
        })
        .await
        .context("spawn_blocking join error")?
    }

    // ── Settings ────────────────────────────────────────────────────────

    pub async fn get_setting(&self, key: String) -> anyhow::Result<Option<String>> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let c = conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
            let result = c.query_row(
                "SELECT value FROM settings WHERE key = ?1",
                rusqlite::params![key],
                |row| row.get(0),
            );
            match result {
                Ok(val) => Ok(Some(val)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(anyhow!("Failed to get setting: {}", e)),
            }
        })
        .await
        .context("spawn_blocking join error")?
    }

    pub async fn set_setting(&self, key: String, value: String) -> anyhow::Result<()> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let c = conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
            c.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                rusqlite::params![key, value],
            )
            .context("Failed to set setting")?;
            Ok(())
        })
        .await
        .context("spawn_blocking join error")?
    }

    pub async fn get_all_settings(&self) -> anyhow::Result<HashMap<String, String>> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let c = conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
            let mut stmt = c
                .prepare("SELECT key, value FROM settings")
                .context("Failed to prepare settings query")?;

            let map: HashMap<String, String> = stmt
                .query_map([], |row| {
                    let key: String = row.get(0)?;
                    let value: String = row.get(1)?;
                    Ok((key, value))
                })
                .context("Failed to query settings")?
                .filter_map(|r| r.ok())
                .collect();

            Ok(map)
        })
        .await
        .context("spawn_blocking join error")?
    }

    // ── Migration History ───────────────────────────────────────────────

    pub async fn save_migration(&self, entry: MigrationHistoryRow) -> anyhow::Result<()> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let c = conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
            c.execute(
                "INSERT OR REPLACE INTO migration_history \
                 (id, source_connection_id, target_connection_id, mode, status, \
                  config_json, result_json, error, started_at, completed_at, \
                  rows_inserted, rows_updated, rows_deleted, rows_skipped, error_count, duration_ms) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                rusqlite::params![
                    entry.id,
                    entry.source_connection_id,
                    entry.target_connection_id,
                    entry.mode,
                    entry.status,
                    entry.config_json,
                    entry.result_json,
                    entry.error,
                    entry.started_at,
                    entry.completed_at,
                    entry.rows_inserted,
                    entry.rows_updated,
                    entry.rows_deleted,
                    entry.rows_skipped,
                    entry.error_count,
                    entry.duration_ms,
                ],
            ).context("Failed to save migration")?;
            Ok(())
        })
        .await
        .context("spawn_blocking join error")?
    }

    pub async fn load_migrations(&self) -> anyhow::Result<Vec<MigrationHistoryRow>> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let c = conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
            let mut stmt = c
                .prepare(
                    "SELECT id, source_connection_id, target_connection_id, mode, status, \
                     config_json, result_json, error, started_at, completed_at, \
                     rows_inserted, rows_updated, rows_deleted, rows_skipped, error_count, duration_ms \
                     FROM migration_history ORDER BY started_at DESC",
                )
                .context("Failed to prepare migration query")?;

            let rows = stmt
                .query_map([], |row| {
                    Ok(MigrationHistoryRow {
                        id: row.get(0)?,
                        source_connection_id: row.get(1)?,
                        target_connection_id: row.get(2)?,
                        mode: row.get(3)?,
                        status: row.get(4)?,
                        config_json: row.get(5)?,
                        result_json: row.get(6)?,
                        error: row.get(7)?,
                        started_at: row.get(8)?,
                        completed_at: row.get(9)?,
                        rows_inserted: row.get(10)?,
                        rows_updated: row.get(11)?,
                        rows_deleted: row.get(12)?,
                        rows_skipped: row.get(13)?,
                        error_count: row.get(14)?,
                        duration_ms: row.get(15)?,
                    })
                })
                .context("Failed to query migrations")?
                .filter_map(|r| r.ok())
                .collect();

            Ok(rows)
        })
        .await
        .context("spawn_blocking join error")?
    }

    // ── Chat Messages ───────────────────────────────────────────────────

    pub async fn save_chat_message(&self, msg: ChatMessageRow) -> anyhow::Result<()> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let c = conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
            c.execute(
                "INSERT OR REPLACE INTO chat_messages (id, role, content, model, timestamp) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![msg.id, msg.role, msg.content, msg.model, msg.timestamp],
            )
            .context("Failed to save chat message")?;
            Ok(())
        })
        .await
        .context("spawn_blocking join error")?
    }

    pub async fn load_chat_messages(&self) -> anyhow::Result<Vec<ChatMessageRow>> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let c = conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
            let mut stmt = c
                .prepare(
                    "SELECT id, role, content, model, timestamp \
                     FROM chat_messages ORDER BY timestamp ASC",
                )
                .context("Failed to prepare chat query")?;

            let rows = stmt
                .query_map([], |row| {
                    Ok(ChatMessageRow {
                        id: row.get(0)?,
                        role: row.get(1)?,
                        content: row.get(2)?,
                        model: row.get(3)?,
                        timestamp: row.get(4)?,
                    })
                })
                .context("Failed to query chat messages")?
                .filter_map(|r| r.ok())
                .collect();

            Ok(rows)
        })
        .await
        .context("spawn_blocking join error")?
    }

    pub async fn clear_chat_messages(&self) -> anyhow::Result<()> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let c = conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
            c.execute("DELETE FROM chat_messages", [])
                .context("Failed to clear chat messages")?;
            Ok(())
        })
        .await
        .context("spawn_blocking join error")?
    }

    // ── Audit Log ───────────────────────────────────────────────────────

    pub async fn log_audit(&self, entry: AuditEntryRow) -> anyhow::Result<()> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let c = conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
            c.execute(
                "INSERT INTO audit_log \
                 (id, timestamp, user_name, action, source_connection, target_connection, affected_rows, details) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    entry.id,
                    entry.timestamp,
                    entry.user_name,
                    entry.action,
                    entry.source_connection,
                    entry.target_connection,
                    entry.affected_rows,
                    entry.details,
                ],
            ).context("Failed to log audit entry")?;
            Ok(())
        })
        .await
        .context("spawn_blocking join error")?
    }

    // ── Context Chunks (RAG) ───────────────────────────────────────────

    pub async fn save_context_chunks(&self, chunks: Vec<ContextChunkRow>) -> anyhow::Result<()> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let c = conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
            let tx = c.unchecked_transaction()
                .context("Failed to begin transaction")?;
            {
                let mut stmt = tx.prepare(
                    "INSERT OR REPLACE INTO context_chunks \
                     (id, connection_id, chunk_type, label, content, embedding, model, created_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                ).context("Failed to prepare insert")?;
                for chunk in &chunks {
                    let blob = chunk.embedding.as_ref().map(|e| embedding_to_blob(e));
                    stmt.execute(rusqlite::params![
                        chunk.id,
                        chunk.connection_id,
                        chunk.chunk_type,
                        chunk.label,
                        chunk.content,
                        blob,
                        chunk.model,
                        chunk.created_at,
                    ]).context("Failed to insert chunk")?;
                }
            }
            tx.commit().context("Failed to commit transaction")?;
            Ok(())
        })
        .await
        .context("spawn_blocking join error")?
    }

    pub async fn clear_chunks_for_connection(&self, connection_id: String) -> anyhow::Result<()> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let c = conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
            c.execute(
                "DELETE FROM context_chunks WHERE connection_id = ?1",
                rusqlite::params![connection_id],
            ).context("Failed to clear chunks")?;
            Ok(())
        })
        .await
        .context("spawn_blocking join error")?
    }

    pub async fn get_all_embedded_chunks(&self) -> anyhow::Result<Vec<ContextChunkRow>> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let c = conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
            let mut stmt = c.prepare(
                "SELECT id, connection_id, chunk_type, label, content, embedding, model, created_at \
                 FROM context_chunks WHERE embedding IS NOT NULL",
            ).context("Failed to prepare query")?;

            let rows = stmt
                .query_map([], |row| {
                    let blob: Option<Vec<u8>> = row.get(5)?;
                    let embedding = blob.map(|b| blob_to_embedding(&b));
                    Ok(ContextChunkRow {
                        id: row.get(0)?,
                        connection_id: row.get(1)?,
                        chunk_type: row.get(2)?,
                        label: row.get(3)?,
                        content: row.get(4)?,
                        embedding,
                        model: row.get(6)?,
                        created_at: row.get(7)?,
                    })
                })
                .context("Failed to query chunks")?
                .filter_map(|r| r.ok())
                .collect();

            Ok(rows)
        })
        .await
        .context("spawn_blocking join error")?
    }

    pub async fn search_similar(
        &self,
        query_embedding: Vec<f32>,
        top_k: usize,
    ) -> anyhow::Result<Vec<(ContextChunkRow, f32)>> {
        let chunks = self.get_all_embedded_chunks().await?;
        let mut scored: Vec<(ContextChunkRow, f32)> = chunks
            .into_iter()
            .filter_map(|chunk| {
                let emb = chunk.embedding.as_ref()?;
                let score = cosine_similarity(&query_embedding, emb);
                Some((chunk, score))
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        Ok(scored)
    }

    // ── Audit Log ───────────────────────────────────────────────────────

    pub async fn get_audit_entries(&self) -> anyhow::Result<Vec<AuditEntryRow>> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let c = conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
            let mut stmt = c
                .prepare(
                    "SELECT id, timestamp, user_name, action, source_connection, \
                     target_connection, affected_rows, details \
                     FROM audit_log ORDER BY timestamp DESC",
                )
                .context("Failed to prepare audit query")?;

            let rows = stmt
                .query_map([], |row| {
                    Ok(AuditEntryRow {
                        id: row.get(0)?,
                        timestamp: row.get(1)?,
                        user_name: row.get(2)?,
                        action: row.get(3)?,
                        source_connection: row.get(4)?,
                        target_connection: row.get(5)?,
                        affected_rows: row.get(6)?,
                        details: row.get(7)?,
                    })
                })
                .context("Failed to query audit log")?
                .filter_map(|r| r.ok())
                .collect();

            Ok(rows)
        })
        .await
        .context("spawn_blocking join error")?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> AppDatabase {
        let dir = tempfile::tempdir().unwrap();
        AppDatabase::init(dir.keep()).unwrap()
    }

    #[tokio::test]
    async fn test_settings_roundtrip() {
        let db = temp_db();
        db.set_setting("theme".into(), "dark".into()).await.unwrap();
        let val = db.get_setting("theme".into()).await.unwrap();
        assert_eq!(val, Some("dark".to_string()));
    }

    #[tokio::test]
    async fn test_settings_get_missing() {
        let db = temp_db();
        let val = db.get_setting("nonexistent".into()).await.unwrap();
        assert_eq!(val, None);
    }

    #[tokio::test]
    async fn test_settings_get_all() {
        let db = temp_db();
        db.set_setting("a".into(), "1".into()).await.unwrap();
        db.set_setting("b".into(), "2".into()).await.unwrap();
        let all = db.get_all_settings().await.unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all.get("a").unwrap(), "1");
        assert_eq!(all.get("b").unwrap(), "2");
    }

    #[tokio::test]
    async fn test_connection_save_load_delete() {
        let db = temp_db();
        let profile = ConnectionProfileRow {
            id: "c1".into(),
            name: "Test DB".into(),
            engine: "PostgreSql".into(),
            host: Some("localhost".into()),
            port: Some(5432),
            database_name: Some("mydb".into()),
            username: Some("user".into()),
            file_path: None,
            read_only: true,
            credential_key: None,
            created_at: "2025-01-01T00:00:00".into(),
            updated_at: "2025-01-01T00:00:00".into(),
        };
        db.save_connection(profile).await.unwrap();

        let conns = db.load_connections().await.unwrap();
        assert_eq!(conns.len(), 1);
        assert_eq!(conns[0].name, "Test DB");
        assert!(conns[0].read_only);

        db.delete_connection("c1".into()).await.unwrap();
        let conns = db.load_connections().await.unwrap();
        assert_eq!(conns.len(), 0);
    }

    #[tokio::test]
    async fn test_chat_messages_roundtrip() {
        let db = temp_db();
        db.save_chat_message(ChatMessageRow {
            id: "m1".into(),
            role: "user".into(),
            content: "Hello".into(),
            model: None,
            timestamp: 1000,
        })
        .await
        .unwrap();

        db.save_chat_message(ChatMessageRow {
            id: "m2".into(),
            role: "assistant".into(),
            content: "Hi!".into(),
            model: Some("llama3".into()),
            timestamp: 1001,
        })
        .await
        .unwrap();

        let msgs = db.load_chat_messages().await.unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].content, "Hello");
        assert_eq!(msgs[1].model, Some("llama3".to_string()));

        db.clear_chat_messages().await.unwrap();
        let msgs = db.load_chat_messages().await.unwrap();
        assert_eq!(msgs.len(), 0);
    }

    #[tokio::test]
    async fn test_migration_history() {
        let db = temp_db();
        let entry = MigrationHistoryRow {
            id: "mig1".into(),
            source_connection_id: Some("c1".into()),
            target_connection_id: Some("c2".into()),
            mode: "insert".into(),
            status: "completed".into(),
            config_json: None,
            result_json: None,
            error: None,
            started_at: "2025-01-01T00:00:00".into(),
            completed_at: Some("2025-01-01T00:01:00".into()),
            rows_inserted: 100,
            rows_updated: 0,
            rows_deleted: 0,
            rows_skipped: 5,
            error_count: 0,
            duration_ms: 60000,
        };
        db.save_migration(entry).await.unwrap();

        let migs = db.load_migrations().await.unwrap();
        assert_eq!(migs.len(), 1);
        assert_eq!(migs[0].rows_inserted, 100);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &b).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        assert!((cosine_similarity(&a, &b) + 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_embedding_roundtrip() {
        let orig = vec![1.0_f32, -2.5, 3.14, 0.0, f32::MAX];
        let blob = embedding_to_blob(&orig);
        let restored = blob_to_embedding(&blob);
        assert_eq!(orig, restored);
    }

    #[tokio::test]
    async fn test_context_chunks_roundtrip() {
        let db = temp_db();
        let chunks = vec![
            ContextChunkRow {
                id: "ch1".into(),
                connection_id: Some("conn1".into()),
                chunk_type: "table_schema".into(),
                label: "users table".into(),
                content: "CREATE TABLE users (id INT, name TEXT)".into(),
                embedding: Some(vec![1.0, 0.0, 0.0]),
                model: Some("nomic-embed-text".into()),
                created_at: "2025-01-01T00:00:00".into(),
            },
            ContextChunkRow {
                id: "ch2".into(),
                connection_id: Some("conn1".into()),
                chunk_type: "connection_summary".into(),
                label: "My PostgreSQL".into(),
                content: "PostgreSQL on localhost, 5 tables".into(),
                embedding: Some(vec![0.0, 1.0, 0.0]),
                model: Some("nomic-embed-text".into()),
                created_at: "2025-01-01T00:00:00".into(),
            },
        ];
        db.save_context_chunks(chunks).await.unwrap();

        let loaded = db.get_all_embedded_chunks().await.unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].label, "users table");
        assert_eq!(loaded[0].embedding.as_ref().unwrap(), &[1.0, 0.0, 0.0]);
    }

    #[tokio::test]
    async fn test_clear_chunks_for_connection() {
        let db = temp_db();
        let chunks = vec![
            ContextChunkRow {
                id: "ch1".into(),
                connection_id: Some("conn1".into()),
                chunk_type: "table_schema".into(),
                label: "t1".into(),
                content: "table 1".into(),
                embedding: Some(vec![1.0, 0.0]),
                model: None,
                created_at: "2025-01-01T00:00:00".into(),
            },
            ContextChunkRow {
                id: "ch2".into(),
                connection_id: Some("conn2".into()),
                chunk_type: "table_schema".into(),
                label: "t2".into(),
                content: "table 2".into(),
                embedding: Some(vec![0.0, 1.0]),
                model: None,
                created_at: "2025-01-01T00:00:00".into(),
            },
        ];
        db.save_context_chunks(chunks).await.unwrap();

        db.clear_chunks_for_connection("conn1".into()).await.unwrap();
        let remaining = db.get_all_embedded_chunks().await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, "ch2");
    }

    #[tokio::test]
    async fn test_search_similar_ordering() {
        let db = temp_db();
        let chunks = vec![
            ContextChunkRow {
                id: "ch1".into(),
                connection_id: None,
                chunk_type: "test".into(),
                label: "close match".into(),
                content: "close".into(),
                embedding: Some(vec![0.9, 0.1, 0.0]),
                model: None,
                created_at: "2025-01-01T00:00:00".into(),
            },
            ContextChunkRow {
                id: "ch2".into(),
                connection_id: None,
                chunk_type: "test".into(),
                label: "far match".into(),
                content: "far".into(),
                embedding: Some(vec![0.0, 0.0, 1.0]),
                model: None,
                created_at: "2025-01-01T00:00:00".into(),
            },
            ContextChunkRow {
                id: "ch3".into(),
                connection_id: None,
                chunk_type: "test".into(),
                label: "medium match".into(),
                content: "medium".into(),
                embedding: Some(vec![0.5, 0.5, 0.0]),
                model: None,
                created_at: "2025-01-01T00:00:00".into(),
            },
        ];
        db.save_context_chunks(chunks).await.unwrap();

        let query = vec![1.0, 0.0, 0.0];
        let results = db.search_similar(query, 2).await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0.id, "ch1"); // closest
        assert_eq!(results[1].0.id, "ch3"); // second closest
        assert!(results[0].1 > results[1].1); // scores descending
    }

    #[tokio::test]
    async fn test_audit_log() {
        let db = temp_db();
        let entry = AuditEntryRow {
            id: "a1".into(),
            timestamp: "2025-01-01T00:00:00".into(),
            user_name: Some("admin".into()),
            action: "migration_executed".into(),
            source_connection: Some("c1".into()),
            target_connection: Some("c2".into()),
            affected_rows: Some(100),
            details: Some("Test migration".into()),
        };
        db.log_audit(entry).await.unwrap();

        let entries = db.get_audit_entries().await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].action, "migration_executed");
    }
}
