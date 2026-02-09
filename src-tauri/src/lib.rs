pub mod appdb;
pub mod commands;
pub mod db;
pub mod jobs;
pub mod ollama;
pub mod security;
pub mod sidecar;

use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

use appdb::AppDatabase;
use db::registry::{ConnectionRegistry, MigrationState};
use ollama::OllamaClient;
use sidecar::SidecarManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let ollama_client = Arc::new(Mutex::new(OllamaClient::new()));
    let sidecar_manager = Arc::new(Mutex::new(SidecarManager::new(ollama_client.clone())));

    let sidecar_for_exit = sidecar_manager.clone();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_stronghold::Builder::new(|password| {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(password);
            hasher.finalize().to_vec()
        }).build())
        .setup(move |app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Initialize embedded app database
            let app_data_dir = app.path().app_data_dir()
                .expect("Failed to resolve app data directory");
            let app_db = AppDatabase::init(app_data_dir)
                .expect("Failed to initialize app database");
            app.manage(Arc::new(Mutex::new(app_db)));

            // Spawn sidecar + model pull in background
            let app_handle = app.handle().clone();
            let sidecar_setup = sidecar_manager.clone();
            tauri::async_runtime::spawn(async move {
                let mut mgr = sidecar_setup.lock().await;
                match mgr.start(&app_handle).await {
                    Ok(()) => {
                        log::info!("Ollama sidecar started successfully");
                        drop(mgr); // release lock before ensure_models
                        let mgr = sidecar_setup.lock().await;
                        if let Err(e) = mgr.ensure_models(&app_handle).await {
                            log::error!("Failed to ensure models: {}", e);
                        }
                    }
                    Err(e) => {
                        log::warn!("Ollama sidecar failed to start: {}. Chat will check for external Ollama.", e);
                    }
                }
            });

            Ok(())
        })
        .manage(Arc::new(Mutex::new(ConnectionRegistry::new())))
        .manage(Arc::new(Mutex::new(MigrationState::new())))
        .manage(ollama_client)
        .manage(sidecar_for_exit.clone())
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
            commands::chat::check_ollama_status,
            commands::chat::list_ollama_models,
            commands::chat::send_chat_message,
            commands::chat::pull_model,
            commands::chat::index_connection_context,
            commands::chat::search_context,
            commands::chat::index_app_context,
            commands::appdb::save_connection_profile,
            commands::appdb::get_connection_profiles,
            commands::appdb::delete_connection_profile,
            commands::appdb::get_setting,
            commands::appdb::set_setting,
            commands::appdb::get_all_settings,
            commands::appdb::get_migration_history,
            commands::appdb::save_chat_message,
            commands::appdb::load_chat_messages,
            commands::appdb::clear_chat_messages,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(move |_app, event| {
        if let tauri::RunEvent::Exit = event {
            let sidecar = sidecar_for_exit.clone();
            tauri::async_runtime::block_on(async {
                let mut mgr = sidecar.lock().await;
                mgr.stop();
            });
        }
    });
}
