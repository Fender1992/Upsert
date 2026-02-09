use std::sync::Arc;
use tauri::{Emitter, State};
use tokio::sync::Mutex;

use crate::appdb::{AppDatabase, ContextChunkRow};
use crate::db::registry::ConnectionRegistry;
use crate::ollama::{ChatMessage, OllamaClient, OllamaModel};
use crate::sidecar::SidecarManager;

/// Required models for auto-setup.
const REQUIRED_MODELS: &[&str] = &["tinyllama", "llama3.2:3b", "nomic-embed-text"];
const EMBEDDING_MODEL: &str = "nomic-embed-text";

/// Extended status including which required models are present/missing.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OllamaStatusExtended {
    pub running: bool,
    pub models: Vec<OllamaModel>,
    pub required_models: Vec<RequiredModelStatus>,
    pub all_models_ready: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RequiredModelStatus {
    pub name: String,
    pub present: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub label: String,
    pub content: String,
    pub chunk_type: String,
    pub score: f32,
}

/// Check if Ollama is running and return available models with required model status.
#[tauri::command]
pub async fn check_ollama_status(
    client: State<'_, Arc<Mutex<OllamaClient>>>,
) -> Result<OllamaStatusExtended, String> {
    let c = client.lock().await;
    let base_status = c.check_status().await;

    let required_models: Vec<RequiredModelStatus> = REQUIRED_MODELS
        .iter()
        .map(|name| {
            let present = base_status.models.iter().any(|m| {
                m.name == *name || m.name.starts_with(&format!("{}:", name))
            });
            RequiredModelStatus {
                name: name.to_string(),
                present,
            }
        })
        .collect();

    let all_models_ready = required_models.iter().all(|m| m.present);

    Ok(OllamaStatusExtended {
        running: base_status.running,
        models: base_status.models,
        required_models,
        all_models_ready,
    })
}

/// List available Ollama models.
#[tauri::command]
pub async fn list_ollama_models(
    client: State<'_, Arc<Mutex<OllamaClient>>>,
) -> Result<Vec<OllamaModel>, String> {
    let c = client.lock().await;
    c.list_models().await.map_err(|e| e.to_string())
}

/// Send a chat message and stream the response via Tauri events.
/// Returns the full response text when complete.
#[tauri::command]
pub async fn send_chat_message(
    model: String,
    messages: Vec<ChatMessage>,
    request_id: String,
    app_handle: tauri::AppHandle,
    client: State<'_, Arc<Mutex<OllamaClient>>>,
) -> Result<String, String> {
    let c = client.lock().await;
    c.chat_stream(&model, messages, &app_handle, &request_id)
        .await
        .map_err(|e| e.to_string())
}

/// Pull a model by name — progress is emitted via `model-pull-progress` events.
#[tauri::command]
pub async fn pull_model(
    name: String,
    app_handle: tauri::AppHandle,
    sidecar: State<'_, Arc<Mutex<SidecarManager>>>,
) -> Result<(), String> {
    let mgr = sidecar.lock().await;
    mgr.pull_model(&name, &app_handle)
        .await
        .map_err(|e| e.to_string())
}

/// Index a connection's schema into the RAG context_chunks table.
/// Fetches tables + column info, chunks them, embeds via nomic-embed-text, and stores.
#[tauri::command]
pub async fn index_connection_context(
    connection_id: String,
    connection_name: String,
    engine: String,
    app_handle: tauri::AppHandle,
    registry: State<'_, Arc<Mutex<ConnectionRegistry>>>,
    client: State<'_, Arc<Mutex<OllamaClient>>>,
    db: State<'_, Arc<Mutex<AppDatabase>>>,
) -> Result<usize, String> {
    let app_db = db.lock().await;

    // Clear existing chunks for this connection
    app_db
        .clear_chunks_for_connection(connection_id.clone())
        .await
        .map_err(|e| e.to_string())?;

    // Fetch tables from the connection
    let connector = {
        let reg = registry.lock().await;
        reg.get(&connection_id)
            .ok_or_else(|| format!("Connection {} not found in registry", connection_id))?
    };

    let table_names = {
        let conn = connector.lock().await;
        conn.get_tables().await.map_err(|e| e.to_string())?
    };

    let mut chunks: Vec<ContextChunkRow> = Vec::new();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    // Connection summary chunk
    let summary_content = format!(
        "Database: {} ({})\nTables ({}): {}",
        connection_name,
        engine,
        table_names.len(),
        table_names.join(", ")
    );
    chunks.push(ContextChunkRow {
        id: format!("conn-summary-{}", connection_id),
        connection_id: Some(connection_id.clone()),
        chunk_type: "connection_summary".into(),
        label: format!("{} ({})", connection_name, engine),
        content: summary_content,
        embedding: None,
        model: Some(EMBEDDING_MODEL.into()),
        created_at: now.clone(),
    });

    // Table schema chunks (up to 50)
    let limit = table_names.len().min(50);
    for table_name in &table_names[..limit] {
        let info = {
            let conn = connector.lock().await;
            match conn.get_table_info(table_name).await {
                Ok(info) => info,
                Err(_) => continue,
            }
        };

        let mut content = format!(
            "Table: {}.{}\nRow count: {}\nColumns:\n",
            info.schema_name,
            info.table_name,
            info.row_count.map(|r| r.to_string()).unwrap_or("unknown".into())
        );

        for col in &info.columns {
            let pk = if col.is_primary_key { " PK" } else { "" };
            let nullable = if col.is_nullable { " NULL" } else { " NOT NULL" };
            let len = col
                .max_length
                .map(|l| format!("({})", l))
                .unwrap_or_default();
            content.push_str(&format!(
                "  {}: {}{}{}{}\n",
                col.name, col.data_type, len, nullable, pk
            ));
        }

        if !info.indexes.is_empty() {
            content.push_str("Indexes:\n");
            for idx in &info.indexes {
                let unique = if idx.is_unique { " UNIQUE" } else { "" };
                content.push_str(&format!(
                    "  {}{}: ({})\n",
                    idx.name,
                    unique,
                    idx.columns.join(", ")
                ));
            }
        }

        if !info.constraints.is_empty() {
            content.push_str("Constraints:\n");
            for con in &info.constraints {
                content.push_str(&format!(
                    "  {} [{:?}]: ({})\n",
                    con.name,
                    con.constraint_type,
                    con.columns.join(", ")
                ));
            }
        }

        chunks.push(ContextChunkRow {
            id: format!("table-{}-{}", connection_id, table_name),
            connection_id: Some(connection_id.clone()),
            chunk_type: "table_schema".into(),
            label: format!("{} ({})", table_name, connection_name),
            content,
            embedding: None,
            model: Some(EMBEDDING_MODEL.into()),
            created_at: now.clone(),
        });
    }

    // Embed each chunk
    let total = chunks.len();
    let ollama = client.lock().await;
    for (i, chunk) in chunks.iter_mut().enumerate() {
        match ollama.embed(EMBEDDING_MODEL, &chunk.content).await {
            Ok(emb) => chunk.embedding = Some(emb),
            Err(e) => {
                log::warn!("Failed to embed chunk {}: {}", chunk.id, e);
            }
        }

        let _ = app_handle.emit(
            "indexing-progress",
            serde_json::json!({
                "connectionId": connection_id,
                "current": i + 1,
                "total": total,
            }),
        );
    }
    drop(ollama);

    let chunk_count = chunks.len();
    app_db
        .save_context_chunks(chunks)
        .await
        .map_err(|e| e.to_string())?;

    let _ = app_handle.emit(
        "indexing-complete",
        serde_json::json!({
            "connectionId": connection_id,
            "chunks": chunk_count,
        }),
    );

    Ok(chunk_count)
}

/// Search context chunks by semantic similarity to the query.
#[tauri::command]
pub async fn search_context(
    query: String,
    top_k: Option<usize>,
    client: State<'_, Arc<Mutex<OllamaClient>>>,
    db: State<'_, Arc<Mutex<AppDatabase>>>,
) -> Result<Vec<SearchResult>, String> {
    let k = top_k.unwrap_or(5);

    // Embed the query
    let ollama = client.lock().await;
    let query_embedding = ollama
        .embed(EMBEDDING_MODEL, &query)
        .await
        .map_err(|e| e.to_string())?;
    drop(ollama);

    // Search similar chunks
    let app_db = db.lock().await;
    let results = app_db
        .search_similar(query_embedding, k)
        .await
        .map_err(|e| e.to_string())?;

    Ok(results
        .into_iter()
        .map(|(chunk, score)| SearchResult {
            label: chunk.label,
            content: chunk.content,
            chunk_type: chunk.chunk_type,
            score,
        })
        .collect())
}

/// Index global app context (capabilities description).
#[tauri::command]
pub async fn index_app_context(
    client: State<'_, Arc<Mutex<OllamaClient>>>,
    db: State<'_, Arc<Mutex<AppDatabase>>>,
) -> Result<(), String> {
    let content = "Upsert is a cross-platform desktop database comparison and migration tool. \
        It supports 7 database engines: SQL Server, PostgreSQL, MySQL, SQLite, MongoDB, Oracle, and CosmosDB. \
        Features include: schema comparison (diff tables, columns, indexes, constraints between databases), \
        data comparison (row-level diff with key matching), migration engine (5 modes: insert, update, upsert, \
        sync, delete), ETL transform pipeline (7 rule types), job scheduling with cron and chains, \
        and reporting/exports (Markdown, HTML, CSV, JSON). \
        Type mappings go through canonical types for cross-engine compatibility.";

    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    let ollama = client.lock().await;
    let embedding = ollama
        .embed(EMBEDDING_MODEL, content)
        .await
        .map_err(|e| e.to_string())?;
    drop(ollama);

    let chunk = ContextChunkRow {
        id: "app-info-global".into(),
        connection_id: None,
        chunk_type: "app_info".into(),
        label: "Upsert capabilities".into(),
        content: content.into(),
        embedding: Some(embedding),
        model: Some(EMBEDDING_MODEL.into()),
        created_at: now,
    };

    let app_db = db.lock().await;
    app_db
        .save_context_chunks(vec![chunk])
        .await
        .map_err(|e| e.to_string())
}
