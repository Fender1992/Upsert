use std::collections::HashMap;
use std::sync::Arc;

use tauri::State;
use tokio::sync::Mutex;

use crate::appdb::{AppDatabase, ChatMessageRow, ConnectionProfileRow};

type AppDbState = Arc<Mutex<AppDatabase>>;

// ── Connection Profiles ─────────────────────────────────────────────────

#[tauri::command]
pub async fn save_connection_profile(
    state: State<'_, AppDbState>,
    profile: ConnectionProfileRow,
) -> Result<(), String> {
    let db = state.lock().await;
    db.save_connection(profile)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_connection_profiles(
    state: State<'_, AppDbState>,
) -> Result<Vec<ConnectionProfileRow>, String> {
    let db = state.lock().await;
    db.load_connections().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_connection_profile(
    state: State<'_, AppDbState>,
    id: String,
) -> Result<(), String> {
    let db = state.lock().await;
    db.delete_connection(id).await.map_err(|e| e.to_string())
}

// ── Settings ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_setting(
    state: State<'_, AppDbState>,
    key: String,
) -> Result<Option<String>, String> {
    let db = state.lock().await;
    db.get_setting(key).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_setting(
    state: State<'_, AppDbState>,
    key: String,
    value: String,
) -> Result<(), String> {
    let db = state.lock().await;
    db.set_setting(key, value).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_all_settings(
    state: State<'_, AppDbState>,
) -> Result<HashMap<String, String>, String> {
    let db = state.lock().await;
    db.get_all_settings().await.map_err(|e| e.to_string())
}

// ── Migration History ───────────────────────────────────────────────────

#[tauri::command]
pub async fn get_migration_history(
    state: State<'_, AppDbState>,
) -> Result<Vec<crate::appdb::MigrationHistoryRow>, String> {
    let db = state.lock().await;
    db.load_migrations().await.map_err(|e| e.to_string())
}

// ── Chat Messages ───────────────────────────────────────────────────────

#[tauri::command]
pub async fn save_chat_message(
    state: State<'_, AppDbState>,
    message: ChatMessageRow,
) -> Result<(), String> {
    let db = state.lock().await;
    db.save_chat_message(message)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load_chat_messages(
    state: State<'_, AppDbState>,
) -> Result<Vec<ChatMessageRow>, String> {
    let db = state.lock().await;
    db.load_chat_messages().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_chat_messages(
    state: State<'_, AppDbState>,
) -> Result<(), String> {
    let db = state.lock().await;
    db.clear_chat_messages().await.map_err(|e| e.to_string())
}
