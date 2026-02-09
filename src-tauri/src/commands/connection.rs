use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

use crate::db::connectors::{ConnectionConfig, DatabaseEngine};
use crate::db::registry::ConnectionRegistry;

/// DTO that the frontend sends (camelCase fields).
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionConfigDto {
    pub engine: DatabaseEngine,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub database: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub connection_string: Option<String>,
    pub file_path: Option<String>,
    pub read_only: bool,
}

impl From<ConnectionConfigDto> for ConnectionConfig {
    fn from(dto: ConnectionConfigDto) -> Self {
        ConnectionConfig {
            engine: dto.engine,
            host: dto.host,
            port: dto.port,
            database: dto.database,
            username: dto.username,
            password: dto.password,
            connection_string: dto.connection_string,
            file_path: dto.file_path,
            read_only: dto.read_only,
            connection_timeout_secs: 30,
        }
    }
}

/// Test a database connection without persisting it.
#[tauri::command]
pub async fn test_connection(config: ConnectionConfigDto) -> Result<bool, String> {
    use crate::db::connectors::*;

    let cfg: ConnectionConfig = config.into();
    let mut connector: Box<dyn DatabaseConnector> = match cfg.engine {
        DatabaseEngine::SqlServer => Box::new(sqlserver::SqlServerConnector::new(cfg.clone())),
        DatabaseEngine::PostgreSql => Box::new(postgres::PostgresConnector::new(cfg.clone())),
        DatabaseEngine::MySql => Box::new(mysql::MySqlConnector::new(cfg.clone())),
        DatabaseEngine::Sqlite => Box::new(sqlite::SqliteConnector::new(cfg.clone())),
        DatabaseEngine::MongoDb => Box::new(mongodb_connector::MongoDbConnector::new(cfg.clone())),
        DatabaseEngine::Oracle => Box::new(oracle::OracleConnector::new(cfg.clone())),
        DatabaseEngine::CosmosDb => Box::new(cosmosdb::CosmosDbConnector::new(cfg.clone())),
    };

    connector.connect().await.map_err(|e| e.to_string())?;
    connector.disconnect().await.map_err(|e| e.to_string())?;
    Ok(true)
}

/// Connect and register a database in the connection registry.
#[tauri::command]
pub async fn connect_database(
    id: String,
    config: ConnectionConfigDto,
    registry: State<'_, Arc<Mutex<ConnectionRegistry>>>,
) -> Result<(), String> {
    let cfg: ConnectionConfig = config.into();
    let mut reg = registry.lock().await;
    reg.connect(id, cfg).await.map_err(|e| e.to_string())
}

/// Disconnect a database and remove it from the registry.
#[tauri::command]
pub async fn disconnect_database(
    id: String,
    registry: State<'_, Arc<Mutex<ConnectionRegistry>>>,
) -> Result<(), String> {
    let mut reg = registry.lock().await;
    reg.disconnect(&id).await.map_err(|e| e.to_string())
}
