pub mod cosmosdb;
pub mod mongodb_connector;
pub mod mysql;
pub mod oracle;
pub mod postgres;
pub mod sqlite;
pub mod sqlserver;

use crate::db::schema::{Row, SchemaInfo, TableInfo};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Supported database engines
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DatabaseEngine {
    SqlServer,
    PostgreSql,
    MySql,
    Sqlite,
    Oracle,
    MongoDb,
    CosmosDb,
}

impl std::fmt::Display for DatabaseEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseEngine::SqlServer => write!(f, "SQL Server"),
            DatabaseEngine::PostgreSql => write!(f, "PostgreSQL"),
            DatabaseEngine::MySql => write!(f, "MySQL"),
            DatabaseEngine::Sqlite => write!(f, "SQLite"),
            DatabaseEngine::Oracle => write!(f, "Oracle"),
            DatabaseEngine::MongoDb => write!(f, "MongoDB"),
            DatabaseEngine::CosmosDb => write!(f, "CosmosDB"),
        }
    }
}

/// Connection configuration for a database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub engine: DatabaseEngine,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub database: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub connection_string: Option<String>,
    pub file_path: Option<String>,
    pub read_only: bool,
    pub connection_timeout_secs: u64,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            engine: DatabaseEngine::Sqlite,
            host: None,
            port: None,
            database: None,
            username: None,
            password: None,
            connection_string: None,
            file_path: None,
            read_only: true,
            connection_timeout_secs: 30,
        }
    }
}

/// The core trait that all database connectors must implement
#[async_trait]
pub trait DatabaseConnector: Send + Sync {
    /// Connect to the database
    async fn connect(&mut self) -> anyhow::Result<()>;

    /// Disconnect from the database
    async fn disconnect(&mut self) -> anyhow::Result<()>;

    /// Check if the connection is active
    async fn is_connected(&self) -> bool;

    /// Get the full database schema
    async fn get_schema(&self) -> anyhow::Result<SchemaInfo>;

    /// Get a list of table names
    async fn get_tables(&self) -> anyhow::Result<Vec<String>>;

    /// Get detailed info for a specific table
    async fn get_table_info(&self, table_name: &str) -> anyhow::Result<TableInfo>;

    /// Get rows from a table with optional limit and offset
    async fn get_rows(
        &self,
        table_name: &str,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> anyhow::Result<Vec<Row>>;

    /// Execute a raw query and return results
    async fn execute_query(&self, query: &str) -> anyhow::Result<Vec<Row>>;

    /// Begin a transaction
    async fn begin_transaction(&mut self) -> anyhow::Result<()>;

    /// Commit a transaction
    async fn commit_transaction(&mut self) -> anyhow::Result<()>;

    /// Rollback a transaction
    async fn rollback_transaction(&mut self) -> anyhow::Result<()>;

    /// Get the database engine type
    fn engine(&self) -> DatabaseEngine;

    /// Get the row count for a table
    async fn get_row_count(&self, table_name: &str) -> anyhow::Result<i64>;
}
