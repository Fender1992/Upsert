use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

use crate::db::registry::ConnectionRegistry;
use crate::db::schema::TableInfo;

/// Return the list of table names for a connection.
#[tauri::command]
pub async fn get_tables(
    connection_id: String,
    registry: State<'_, Arc<Mutex<ConnectionRegistry>>>,
) -> Result<Vec<String>, String> {
    let reg = registry.lock().await;
    let conn = reg
        .get(&connection_id)
        .ok_or_else(|| format!("Connection '{}' not found", connection_id))?;
    let guard = conn.lock().await;
    guard.get_tables().await.map_err(|e| e.to_string())
}

/// Return detailed table info (columns, indexes, constraints).
#[tauri::command]
pub async fn get_table_info(
    connection_id: String,
    table_name: String,
    registry: State<'_, Arc<Mutex<ConnectionRegistry>>>,
) -> Result<TableInfo, String> {
    let reg = registry.lock().await;
    let conn = reg
        .get(&connection_id)
        .ok_or_else(|| format!("Connection '{}' not found", connection_id))?;
    let guard = conn.lock().await;
    guard
        .get_table_info(&table_name)
        .await
        .map_err(|e| e.to_string())
}

/// Return the row count for a table.
#[tauri::command]
pub async fn get_row_count(
    connection_id: String,
    table_name: String,
    registry: State<'_, Arc<Mutex<ConnectionRegistry>>>,
) -> Result<i64, String> {
    let reg = registry.lock().await;
    let conn = reg
        .get(&connection_id)
        .ok_or_else(|| format!("Connection '{}' not found", connection_id))?;
    let guard = conn.lock().await;
    guard
        .get_row_count(&table_name)
        .await
        .map_err(|e| e.to_string())
}
