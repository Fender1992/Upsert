use super::{ConnectionConfig, DatabaseConnector, DatabaseEngine};
use crate::db::schema::{Row, SchemaInfo, TableInfo};
use async_trait::async_trait;

/// SQL Server connector using tiberius
pub struct SqlServerConnector {
    config: ConnectionConfig,
    connected: bool,
}

impl SqlServerConnector {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            connected: false,
        }
    }
}

#[async_trait]
impl DatabaseConnector for SqlServerConnector {
    async fn connect(&mut self) -> anyhow::Result<()> {
        todo!("SQL Server connect implementation")
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        self.connected = false;
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        self.connected
    }

    async fn get_schema(&self) -> anyhow::Result<SchemaInfo> {
        todo!("SQL Server get_schema implementation")
    }

    async fn get_tables(&self) -> anyhow::Result<Vec<String>> {
        todo!("SQL Server get_tables implementation")
    }

    async fn get_table_info(&self, _table_name: &str) -> anyhow::Result<TableInfo> {
        todo!("SQL Server get_table_info implementation")
    }

    async fn get_rows(
        &self,
        _table_name: &str,
        _limit: Option<u64>,
        _offset: Option<u64>,
    ) -> anyhow::Result<Vec<Row>> {
        todo!("SQL Server get_rows implementation")
    }

    async fn execute_query(&self, _query: &str) -> anyhow::Result<Vec<Row>> {
        todo!("SQL Server execute_query implementation")
    }

    async fn begin_transaction(&mut self) -> anyhow::Result<()> {
        todo!("SQL Server begin_transaction implementation")
    }

    async fn commit_transaction(&mut self) -> anyhow::Result<()> {
        todo!("SQL Server commit_transaction implementation")
    }

    async fn rollback_transaction(&mut self) -> anyhow::Result<()> {
        todo!("SQL Server rollback_transaction implementation")
    }

    fn engine(&self) -> DatabaseEngine {
        DatabaseEngine::SqlServer
    }

    async fn get_row_count(&self, _table_name: &str) -> anyhow::Result<i64> {
        todo!("SQL Server get_row_count implementation")
    }
}
