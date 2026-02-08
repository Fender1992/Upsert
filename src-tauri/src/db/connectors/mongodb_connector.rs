use super::{ConnectionConfig, DatabaseConnector, DatabaseEngine};
use crate::db::schema::{ColumnInfo, Row, SchemaInfo, TableInfo};
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use mongodb::bson::{doc, Bson, Document};
use mongodb::{Client, Database};

/// MongoDB connector using the official mongodb driver.
///
/// MongoDB is schema-less, so schema introspection works by sampling
/// documents from each collection to infer field names and types.
pub struct MongoDbConnector {
    config: ConnectionConfig,
    client: Option<Client>,
    database: Option<Database>,
}

impl MongoDbConnector {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            client: None,
            database: None,
        }
    }

    /// Build a MongoDB connection URI from config.
    fn build_uri(&self) -> String {
        if let Some(ref conn_str) = self.config.connection_string {
            return conn_str.clone();
        }

        let host = self
            .config
            .host
            .as_deref()
            .unwrap_or("localhost");
        let port = self.config.port.unwrap_or(27017);

        if let (Some(ref user), Some(ref pass)) = (&self.config.username, &self.config.password) {
            format!("mongodb://{}:{}@{}:{}", user, pass, host, port)
        } else {
            format!("mongodb://{}:{}", host, port)
        }
    }

    /// Get the database name from config.
    fn database_name(&self) -> String {
        self.config
            .database
            .clone()
            .unwrap_or_else(|| "test".to_string())
    }

    /// Get a reference to the connected database, or return an error.
    fn db(&self) -> anyhow::Result<&Database> {
        self.database
            .as_ref()
            .ok_or_else(|| anyhow!("Not connected to MongoDB"))
    }

    /// Infer the BSON type as a string for schema purposes.
    fn bson_type_name(value: &Bson) -> &'static str {
        match value {
            Bson::Double(_) => "double",
            Bson::String(_) => "string",
            Bson::Document(_) => "object",
            Bson::Array(_) => "array",
            Bson::Binary(_) => "binData",
            Bson::ObjectId(_) => "objectId",
            Bson::Boolean(_) => "bool",
            Bson::DateTime(_) => "date",
            Bson::Null => "null",
            Bson::Int32(_) => "int",
            Bson::Int64(_) => "long",
            Bson::Timestamp(_) => "timestamp",
            Bson::Decimal128(_) => "decimal",
            _ => "unknown",
        }
    }
}

#[async_trait]
impl DatabaseConnector for MongoDbConnector {
    async fn connect(&mut self) -> anyhow::Result<()> {
        let uri = self.build_uri();

        let client = Client::with_uri_str(&uri)
            .await
            .context("Failed to create MongoDB client")?;

        // Ping to verify the connection
        let db_name = self.database_name();
        let db = client.database(&db_name);
        db.run_command(doc! { "ping": 1 })
            .await
            .context("MongoDB ping failed - check connection settings")?;

        self.database = Some(db);
        self.client = Some(client);
        Ok(())
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        self.database = None;
        self.client = None;
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        if let Some(ref db) = self.database {
            db.run_command(doc! { "ping": 1 }).await.is_ok()
        } else {
            false
        }
    }

    async fn get_schema(&self) -> anyhow::Result<SchemaInfo> {
        let db_name = self.database_name();
        let tables = self.get_tables().await?;
        let mut table_infos = Vec::new();

        for collection_name in &tables {
            match self.get_table_info(collection_name).await {
                Ok(info) => table_infos.push(info),
                Err(e) => {
                    log::warn!(
                        "Failed to infer schema for collection {}: {}",
                        collection_name,
                        e
                    );
                }
            }
        }

        Ok(SchemaInfo {
            database_name: db_name,
            tables: table_infos,
        })
    }

    async fn get_tables(&self) -> anyhow::Result<Vec<String>> {
        let db = self.db()?;
        let mut names = db
            .list_collection_names()
            .await
            .context("Failed to list MongoDB collections")?;
        names.sort();
        Ok(names)
    }

    async fn get_table_info(&self, table_name: &str) -> anyhow::Result<TableInfo> {
        let db = self.db()?;
        let collection = db.collection::<Document>(table_name);

        // Sample up to 100 documents to infer schema
        use futures_util::TryStreamExt;
        let mut cursor = collection
            .find(doc! {})
            .limit(100)
            .await
            .context("Failed to query collection for schema inference")?;

        let mut field_types: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        while let Some(doc) = cursor.try_next().await? {
            for (key, value) in doc.iter() {
                field_types
                    .entry(key.clone())
                    .or_insert_with(|| Self::bson_type_name(value).to_string());
            }
        }

        // Sort fields by name for consistent ordering
        let mut field_list: Vec<(String, String)> = field_types.into_iter().collect();
        field_list.sort_by(|a, b| a.0.cmp(&b.0));

        let columns: Vec<ColumnInfo> = field_list
            .iter()
            .enumerate()
            .map(|(i, (name, dtype))| ColumnInfo {
                name: name.clone(),
                data_type: dtype.clone(),
                is_nullable: true, // MongoDB fields are always nullable
                is_primary_key: name == "_id",
                max_length: None,
                precision: None,
                scale: None,
                default_value: None,
                ordinal_position: (i + 1) as i32,
            })
            .collect();

        let row_count = self.get_row_count(table_name).await.ok();

        Ok(TableInfo {
            schema_name: self.database_name(),
            table_name: table_name.to_string(),
            columns,
            indexes: Vec::new(), // Could be enhanced later with list_indexes
            constraints: Vec::new(),
            row_count,
        })
    }

    async fn get_rows(
        &self,
        table_name: &str,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> anyhow::Result<Vec<Row>> {
        let db = self.db()?;
        let collection = db.collection::<Document>(table_name);

        let limit_val = limit.unwrap_or(100) as i64;
        let skip_val = offset.unwrap_or(0);

        use futures_util::TryStreamExt;
        let mut cursor = collection
            .find(doc! {})
            .skip(skip_val)
            .limit(limit_val)
            .await
            .context("Failed to query MongoDB collection")?;

        let mut rows = Vec::new();
        while let Some(doc) = cursor.try_next().await? {
            rows.push(bson_doc_to_row(&doc));
        }

        Ok(rows)
    }

    async fn execute_query(&self, query: &str) -> anyhow::Result<Vec<Row>> {
        let db = self.db()?;

        // Interpret the query as a JSON command document
        let command: Document = serde_json::from_str(query)
            .map_err(|e| anyhow!("Query must be a valid JSON document: {}", e))?;

        let result = db
            .run_command(command)
            .await
            .context("Failed to execute MongoDB command")?;

        // Try to extract results from common command response formats
        let mut rows = Vec::new();

        // Check for cursor-based response (find, aggregate)
        if let Some(Bson::Document(cursor_doc)) = result.get("cursor") {
            if let Some(Bson::Array(batch)) = cursor_doc.get("firstBatch") {
                for item in batch {
                    if let Bson::Document(doc) = item {
                        rows.push(bson_doc_to_row(doc));
                    }
                }
            }
        } else {
            // Return the whole result as a single row
            rows.push(bson_doc_to_row(&result));
        }

        Ok(rows)
    }

    async fn begin_transaction(&mut self) -> anyhow::Result<()> {
        // MongoDB transactions require replica sets.
        // For now, this is a no-op that logs a warning.
        log::warn!("MongoDB transactions require a replica set. Transaction not started.");
        Ok(())
    }

    async fn commit_transaction(&mut self) -> anyhow::Result<()> {
        log::warn!("MongoDB transactions require a replica set. Commit is a no-op.");
        Ok(())
    }

    async fn rollback_transaction(&mut self) -> anyhow::Result<()> {
        log::warn!("MongoDB transactions require a replica set. Rollback is a no-op.");
        Ok(())
    }

    fn engine(&self) -> DatabaseEngine {
        DatabaseEngine::MongoDb
    }

    async fn get_row_count(&self, table_name: &str) -> anyhow::Result<i64> {
        let db = self.db()?;
        let collection = db.collection::<Document>(table_name);

        let count = collection
            .count_documents(doc! {})
            .await
            .context("Failed to count documents")?;

        Ok(count as i64)
    }
}

/// Convert a BSON Document to our Row type (HashMap<String, serde_json::Value>).
fn bson_doc_to_row(doc: &Document) -> Row {
    let mut map = std::collections::HashMap::new();
    for (key, value) in doc.iter() {
        map.insert(key.clone(), bson_to_json(value));
    }
    map
}

/// Convert a BSON value to serde_json::Value.
fn bson_to_json(bson: &Bson) -> serde_json::Value {
    match bson {
        Bson::Double(v) => serde_json::json!(*v),
        Bson::String(v) => serde_json::Value::String(v.clone()),
        Bson::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(bson_to_json).collect())
        }
        Bson::Document(doc) => {
            let map: serde_json::Map<String, serde_json::Value> = doc
                .iter()
                .map(|(k, v)| (k.clone(), bson_to_json(v)))
                .collect();
            serde_json::Value::Object(map)
        }
        Bson::Boolean(v) => serde_json::Value::Bool(*v),
        Bson::Null => serde_json::Value::Null,
        Bson::Int32(v) => serde_json::json!(*v),
        Bson::Int64(v) => serde_json::json!(*v),
        Bson::ObjectId(oid) => serde_json::Value::String(oid.to_hex()),
        Bson::DateTime(dt) => serde_json::Value::String(
            dt.try_to_rfc3339_string()
                .unwrap_or_else(|_| format!("{:?}", dt)),
        ),
        Bson::Decimal128(d) => serde_json::Value::String(d.to_string()),
        Bson::Timestamp(ts) => serde_json::json!({
            "t": ts.time,
            "i": ts.increment
        }),
        Bson::Binary(bin) => serde_json::Value::String(hex::encode(&bin.bytes)),
        _ => serde_json::Value::String(format!("{:?}", bson)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_connector() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::MongoDb,
            host: Some("localhost".to_string()),
            port: Some(27017),
            database: Some("testdb".to_string()),
            ..Default::default()
        };
        let connector = MongoDbConnector::new(config);
        assert_eq!(connector.engine(), DatabaseEngine::MongoDb);
        assert!(connector.client.is_none());
    }

    #[test]
    fn test_build_uri_from_params() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::MongoDb,
            host: Some("myhost".to_string()),
            port: Some(27018),
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
            ..Default::default()
        };
        let connector = MongoDbConnector::new(config);
        assert_eq!(connector.build_uri(), "mongodb://user:pass@myhost:27018");
    }

    #[test]
    fn test_build_uri_no_auth() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::MongoDb,
            host: Some("myhost".to_string()),
            port: Some(27017),
            ..Default::default()
        };
        let connector = MongoDbConnector::new(config);
        assert_eq!(connector.build_uri(), "mongodb://myhost:27017");
    }

    #[test]
    fn test_build_uri_defaults() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::MongoDb,
            ..Default::default()
        };
        let connector = MongoDbConnector::new(config);
        assert_eq!(connector.build_uri(), "mongodb://localhost:27017");
    }

    #[test]
    fn test_build_uri_from_connection_string() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::MongoDb,
            connection_string: Some("mongodb://custom:27019/mydb".to_string()),
            host: Some("ignored".to_string()),
            ..Default::default()
        };
        let connector = MongoDbConnector::new(config);
        assert_eq!(connector.build_uri(), "mongodb://custom:27019/mydb");
    }

    #[test]
    fn test_database_name_default() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::MongoDb,
            ..Default::default()
        };
        let connector = MongoDbConnector::new(config);
        assert_eq!(connector.database_name(), "test");
    }

    #[test]
    fn test_database_name_from_config() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::MongoDb,
            database: Some("mydb".to_string()),
            ..Default::default()
        };
        let connector = MongoDbConnector::new(config);
        assert_eq!(connector.database_name(), "mydb");
    }

    #[test]
    fn test_bson_type_names() {
        assert_eq!(MongoDbConnector::bson_type_name(&Bson::Double(1.0)), "double");
        assert_eq!(
            MongoDbConnector::bson_type_name(&Bson::String("hi".to_string())),
            "string"
        );
        assert_eq!(MongoDbConnector::bson_type_name(&Bson::Boolean(true)), "bool");
        assert_eq!(MongoDbConnector::bson_type_name(&Bson::Int32(1)), "int");
        assert_eq!(MongoDbConnector::bson_type_name(&Bson::Int64(1)), "long");
        assert_eq!(MongoDbConnector::bson_type_name(&Bson::Null), "null");
    }

    #[test]
    fn test_bson_doc_to_row() {
        let doc = doc! {
            "name": "test",
            "value": 42,
            "active": true,
            "score": 3.14,
            "missing": Bson::Null,
        };

        let row = bson_doc_to_row(&doc);
        assert_eq!(row["name"], serde_json::json!("test"));
        assert_eq!(row["value"], serde_json::json!(42));
        assert_eq!(row["active"], serde_json::json!(true));
        assert_eq!(row["score"], serde_json::json!(3.14));
        assert_eq!(row["missing"], serde_json::Value::Null);
    }

    #[test]
    fn test_bson_nested_doc_to_json() {
        let doc = doc! {
            "nested": {
                "inner": "value"
            },
            "arr": [1, 2, 3]
        };

        let row = bson_doc_to_row(&doc);
        assert_eq!(row["nested"]["inner"], serde_json::json!("value"));
        assert_eq!(row["arr"], serde_json::json!([1, 2, 3]));
    }

    #[tokio::test]
    async fn test_not_connected_by_default() {
        let config = ConnectionConfig {
            engine: DatabaseEngine::MongoDb,
            ..Default::default()
        };
        let connector = MongoDbConnector::new(config);
        assert!(!connector.is_connected().await);
    }
}
