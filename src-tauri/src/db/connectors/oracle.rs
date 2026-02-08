use super::{ConnectionConfig, DatabaseConnector, DatabaseEngine};
use crate::db::schema::{Row, SchemaInfo, TableInfo};
use async_trait::async_trait;

/// Oracle connector - STUB IMPLEMENTATION
/// Oracle Instant Client is not available. All methods have todo!() bodies.
/// The interface is defined to match the DatabaseConnector trait.
pub struct OracleConnector {
    config: ConnectionConfig,
    connected: bool,
}

impl OracleConnector {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            connected: false,
        }
    }
}

#[async_trait]
impl DatabaseConnector for OracleConnector {
    async fn connect(&mut self) -> anyhow::Result<()> {
        todo!("Oracle connect - requires Oracle Instant Client")
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        todo!("Oracle disconnect - requires Oracle Instant Client")
    }

    async fn is_connected(&self) -> bool {
        false
    }

    async fn get_schema(&self) -> anyhow::Result<SchemaInfo> {
        todo!("Oracle get_schema - requires Oracle Instant Client")
    }

    async fn get_tables(&self) -> anyhow::Result<Vec<String>> {
        todo!("Oracle get_tables - requires Oracle Instant Client")
    }

    async fn get_table_info(&self, _table_name: &str) -> anyhow::Result<TableInfo> {
        todo!("Oracle get_table_info - requires Oracle Instant Client")
    }

    async fn get_rows(
        &self,
        _table_name: &str,
        _limit: Option<u64>,
        _offset: Option<u64>,
    ) -> anyhow::Result<Vec<Row>> {
        todo!("Oracle get_rows - requires Oracle Instant Client")
    }

    async fn execute_query(&self, _query: &str) -> anyhow::Result<Vec<Row>> {
        todo!("Oracle execute_query - requires Oracle Instant Client")
    }

    async fn begin_transaction(&mut self) -> anyhow::Result<()> {
        todo!("Oracle begin_transaction - requires Oracle Instant Client")
    }

    async fn commit_transaction(&mut self) -> anyhow::Result<()> {
        todo!("Oracle commit_transaction - requires Oracle Instant Client")
    }

    async fn rollback_transaction(&mut self) -> anyhow::Result<()> {
        todo!("Oracle rollback_transaction - requires Oracle Instant Client")
    }

    fn engine(&self) -> DatabaseEngine {
        DatabaseEngine::Oracle
    }

    async fn get_row_count(&self, _table_name: &str) -> anyhow::Result<i64> {
        todo!("Oracle get_row_count - requires Oracle Instant Client")
    }
}
