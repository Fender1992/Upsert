pub mod commands;
pub mod db;
pub mod jobs;
pub mod security;

use std::sync::Arc;
use tokio::sync::Mutex;

use db::registry::{ConnectionRegistry, MigrationState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_stronghold::Builder::new(|password| {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(password);
            hasher.finalize().to_vec()
        }).build())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .manage(Arc::new(Mutex::new(ConnectionRegistry::new())))
        .manage(Arc::new(Mutex::new(MigrationState::new())))
        .invoke_handler(tauri::generate_handler![
            commands::connection::test_connection,
            commands::connection::connect_database,
            commands::connection::disconnect_database,
            commands::schema::get_tables,
            commands::schema::get_table_info,
            commands::schema::get_row_count,
            commands::migration::dry_run,
            commands::migration::execute_migration,
            commands::migration::cancel_migration,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
