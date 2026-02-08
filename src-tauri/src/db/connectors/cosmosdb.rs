use super::{ConnectionConfig, DatabaseConnector, DatabaseEngine};
use crate::db::schema::{Row, SchemaInfo, TableInfo};
use async_trait::async_trait;

/// CosmosDB connector - STUB IMPLEMENTATION
/// All database methods have todo!() bodies.
/// The interface is defined to match the DatabaseConnector trait so the
/// project compiles and CosmosDB support can be added later.
pub struct CosmosDbConnector {
    #[allow(dead_code)]
    config: ConnectionConfig,
}

impl CosmosDbConnector {
    pub fn new(config: ConnectionConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl DatabaseConnector for CosmosDbConnector {
    async fn connect(&mut self) -> anyhow::Result<()> {
        todo!("CosmosDB connect implementation")
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        todo!("CosmosDB disconnect implementation")
    }

    async fn is_connected(&self) -> bool {
        false
    }

    async fn get_schema(&self) -> anyhow::Result<SchemaInfo> {
        todo!("CosmosDB get_schema implementation")
    }

    async fn get_tables(&self) -> anyhow::Result<Vec<String>> {
        todo!("CosmosDB get_tables implementation")
    }

    async fn get_table_info(&self, _table_name: &str) -> anyhow::Result<TableInfo> {
        todo!("CosmosDB get_table_info implementation")
    }

    async fn get_rows(
        &self,
        _table_name: &str,
        _limit: Option<u64>,
        _offset: Option<u64>,
    ) -> anyhow::Result<Vec<Row>> {
        todo!("CosmosDB get_rows implementation")
    }

    async fn execute_query(&self, _query: &str) -> anyhow::Result<Vec<Row>> {
        todo!("CosmosDB execute_query implementation")
    }

    async fn begin_transaction(&mut self) -> anyhow::Result<()> {
        todo!("CosmosDB begin_transaction implementation")
    }

    async fn commit_transaction(&mut self) -> anyhow::Result<()> {
        todo!("CosmosDB commit_transaction implementation")
    }

    async fn rollback_transaction(&mut self) -> anyhow::Result<()> {
        todo!("CosmosDB rollback_transaction implementation")
    }

    fn engine(&self) -> DatabaseEngine {
        DatabaseEngine::CosmosDb
    }

    async fn get_row_count(&self, _table_name: &str) -> anyhow::Result<i64> {
        todo!("CosmosDB get_row_count implementation")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_connector() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::CosmosDb,
            connection_string: Some("AccountEndpoint=https://example.documents.azure.com:443/".to_string()),
            ..Default::default()
        };
        let connector = CosmosDbConnector::new(config);
        assert_eq!(connector.engine(), DatabaseEngine::CosmosDb);
    }

    #[tokio::test]
    async fn test_not_connected() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::CosmosDb,
            ..Default::default()
        };
        let connector = CosmosDbConnector::new(config);
        assert!(!connector.is_connected().await);
    }
}
