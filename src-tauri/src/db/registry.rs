use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::connectors::{
    cosmosdb::CosmosDbConnector,
    mongodb_connector::MongoDbConnector,
    mysql::MySqlConnector,
    oracle::OracleConnector,
    postgres::PostgresConnector,
    sqlite::SqliteConnector,
    sqlserver::SqlServerConnector,
    ConnectionConfig, DatabaseConnector, DatabaseEngine,
};
use crate::db::migrator::CancellationToken;

pub type SharedConnector = Arc<Mutex<Box<dyn DatabaseConnector>>>;

/// Holds live database connections keyed by a user-chosen ID.
pub struct ConnectionRegistry {
    connections: HashMap<String, SharedConnector>,
}

impl ConnectionRegistry {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
        }
    }

    /// Create a connector for the given engine, connect it, and store it.
    pub async fn connect(&mut self, id: String, config: ConnectionConfig) -> anyhow::Result<()> {
        let mut connector: Box<dyn DatabaseConnector> = match config.engine {
            DatabaseEngine::SqlServer => Box::new(SqlServerConnector::new(config)),
            DatabaseEngine::PostgreSql => Box::new(PostgresConnector::new(config)),
            DatabaseEngine::MySql => Box::new(MySqlConnector::new(config)),
            DatabaseEngine::Sqlite => Box::new(SqliteConnector::new(config)),
            DatabaseEngine::MongoDb => Box::new(MongoDbConnector::new(config)),
            DatabaseEngine::Oracle => Box::new(OracleConnector::new(config)),
            DatabaseEngine::CosmosDb => Box::new(CosmosDbConnector::new(config)),
        };
        connector.connect().await?;
        self.connections.insert(id, Arc::new(Mutex::new(connector)));
        Ok(())
    }

    /// Get a shared reference to a live connector.
    pub fn get(&self, id: &str) -> Option<SharedConnector> {
        self.connections.get(id).cloned()
    }

    /// Disconnect and remove a connector.
    pub async fn disconnect(&mut self, id: &str) -> anyhow::Result<()> {
        if let Some(conn) = self.connections.remove(id) {
            let mut guard = conn.lock().await;
            guard.disconnect().await?;
        }
        Ok(())
    }
}

/// Holds active migration cancellation tokens.
pub struct MigrationState {
    tokens: HashMap<String, CancellationToken>,
}

impl MigrationState {
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: String, token: CancellationToken) {
        self.tokens.insert(id, token);
    }

    pub fn cancel(&self, id: &str) -> bool {
        if let Some(token) = self.tokens.get(id) {
            token.cancel();
            true
        } else {
            false
        }
    }

    pub fn remove(&mut self, id: &str) {
        self.tokens.remove(id);
    }
}
