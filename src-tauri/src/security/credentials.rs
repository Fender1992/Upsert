use crate::db::connectors::{ConnectionConfig, DatabaseEngine};
use crate::security::ConnectionProfile;
use serde::{Deserialize, Serialize};

/// Represents a credential entry stored in the secure vault.
/// The actual password is only held in Stronghold at runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialEntry {
    pub key: String,
    pub profile_id: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Errors from credential operations.
#[derive(Debug)]
pub enum CredentialError {
    NotFound(String),
    StoreError(String),
    SerializationError(String),
}

impl std::fmt::Display for CredentialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CredentialError::NotFound(key) => write!(f, "Credential not found: {}", key),
            CredentialError::StoreError(msg) => write!(f, "Credential store error: {}", msg),
            CredentialError::SerializationError(msg) => {
                write!(f, "Credential serialization error: {}", msg)
            }
        }
    }
}

impl std::error::Error for CredentialError {}

/// Trait defining the interface for credential storage.
///
/// At runtime, this will be implemented using Tauri's Stronghold plugin
/// which requires an AppHandle. The trait allows testing with mock
/// implementations.
pub trait CredentialStore {
    /// Store a credential (password) under the given key.
    fn store(&self, key: &str, value: &str) -> Result<(), CredentialError>;

    /// Retrieve a credential by key.
    fn retrieve(&self, key: &str) -> Result<String, CredentialError>;

    /// Delete a credential by key.
    fn delete(&self, key: &str) -> Result<(), CredentialError>;

    /// List all credential keys.
    fn list_keys(&self) -> Result<Vec<String>, CredentialError>;
}

/// In-memory credential store for testing purposes.
/// Production code will use Stronghold via AppHandle.
pub struct InMemoryCredentialStore {
    entries: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

impl InMemoryCredentialStore {
    pub fn new() -> Self {
        Self {
            entries: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for InMemoryCredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialStore for InMemoryCredentialStore {
    fn store(&self, key: &str, value: &str) -> Result<(), CredentialError> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|e| CredentialError::StoreError(e.to_string()))?;
        entries.insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn retrieve(&self, key: &str) -> Result<String, CredentialError> {
        let entries = self
            .entries
            .lock()
            .map_err(|e| CredentialError::StoreError(e.to_string()))?;
        entries
            .get(key)
            .cloned()
            .ok_or_else(|| CredentialError::NotFound(key.to_string()))
    }

    fn delete(&self, key: &str) -> Result<(), CredentialError> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|e| CredentialError::StoreError(e.to_string()))?;
        entries.remove(key);
        Ok(())
    }

    fn list_keys(&self) -> Result<Vec<String>, CredentialError> {
        let entries = self
            .entries
            .lock()
            .map_err(|e| CredentialError::StoreError(e.to_string()))?;
        Ok(entries.keys().cloned().collect())
    }
}

/// Generate a credential key for a connection profile.
pub fn credential_key_for_profile(profile_id: &str) -> String {
    format!("upsert_cred_{}", profile_id)
}

/// Parse a database engine string into a DatabaseEngine enum.
fn parse_engine(engine: &str) -> Result<DatabaseEngine, CredentialError> {
    match engine.to_lowercase().as_str() {
        "sqlserver" | "sql server" | "mssql" => Ok(DatabaseEngine::SqlServer),
        "postgresql" | "postgres" => Ok(DatabaseEngine::PostgreSql),
        "mysql" => Ok(DatabaseEngine::MySql),
        "sqlite" => Ok(DatabaseEngine::Sqlite),
        "oracle" => Ok(DatabaseEngine::Oracle),
        "mongodb" | "mongo" => Ok(DatabaseEngine::MongoDb),
        "cosmosdb" | "cosmos" => Ok(DatabaseEngine::CosmosDb),
        _ => Err(CredentialError::StoreError(format!(
            "Unknown database engine: {}",
            engine
        ))),
    }
}

/// Build a ConnectionConfig from a profile and a credential store.
/// The password is fetched from the credential store using the profile's credential_key.
pub fn build_connection_config(
    profile: &ConnectionProfile,
    store: &dyn CredentialStore,
) -> Result<ConnectionConfig, CredentialError> {
    let engine = parse_engine(&profile.engine)?;

    let password = match &profile.credential_key {
        Some(key) => Some(store.retrieve(key)?),
        None => None,
    };

    Ok(ConnectionConfig {
        engine,
        host: profile.host.clone(),
        port: profile.port,
        database: profile.database.clone(),
        username: profile.username.clone(),
        password,
        connection_string: None,
        file_path: profile.file_path.clone(),
        read_only: profile.read_only,
        connection_timeout_secs: 30,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_profile() -> ConnectionProfile {
        ConnectionProfile {
            id: "test-id".to_string(),
            name: "Test Connection".to_string(),
            engine: "postgresql".to_string(),
            host: Some("localhost".to_string()),
            port: Some(5432),
            database: Some("testdb".to_string()),
            username: Some("admin".to_string()),
            credential_key: Some("upsert_cred_test-id".to_string()),
            file_path: None,
            read_only: false,
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_in_memory_store_crud() {
        let store = InMemoryCredentialStore::new();

        // Store
        store.store("key1", "password123").unwrap();
        store.store("key2", "secret456").unwrap();

        // Retrieve
        assert_eq!(store.retrieve("key1").unwrap(), "password123");
        assert_eq!(store.retrieve("key2").unwrap(), "secret456");

        // List
        let mut keys = store.list_keys().unwrap();
        keys.sort();
        assert_eq!(keys, vec!["key1", "key2"]);

        // Delete
        store.delete("key1").unwrap();
        assert!(store.retrieve("key1").is_err());

        // List after delete
        let keys = store.list_keys().unwrap();
        assert_eq!(keys.len(), 1);
    }

    #[test]
    fn test_retrieve_nonexistent_key() {
        let store = InMemoryCredentialStore::new();
        let result = store.retrieve("nonexistent");
        assert!(result.is_err());
        match result {
            Err(CredentialError::NotFound(key)) => assert_eq!(key, "nonexistent"),
            other => panic!("Expected NotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_credential_key_for_profile() {
        assert_eq!(
            credential_key_for_profile("abc-123"),
            "upsert_cred_abc-123"
        );
    }

    #[test]
    fn test_build_connection_config() {
        let store = InMemoryCredentialStore::new();
        store.store("upsert_cred_test-id", "mypassword").unwrap();

        let profile = make_profile();
        let config = build_connection_config(&profile, &store).unwrap();

        assert_eq!(config.engine, DatabaseEngine::PostgreSql);
        assert_eq!(config.host.as_deref(), Some("localhost"));
        assert_eq!(config.port, Some(5432));
        assert_eq!(config.database.as_deref(), Some("testdb"));
        assert_eq!(config.username.as_deref(), Some("admin"));
        assert_eq!(config.password.as_deref(), Some("mypassword"));
        assert!(!config.read_only);
    }

    #[test]
    fn test_build_connection_config_no_credential() {
        let store = InMemoryCredentialStore::new();
        let mut profile = make_profile();
        profile.credential_key = None;

        let config = build_connection_config(&profile, &store).unwrap();
        assert!(config.password.is_none());
    }

    #[test]
    fn test_build_connection_config_missing_credential() {
        let store = InMemoryCredentialStore::new();
        let profile = make_profile();

        // Don't store the credential - should fail
        let result = build_connection_config(&profile, &store);
        assert!(result.is_err());
    }

    #[test]
    fn test_profile_serialization_no_password() {
        let profile = make_profile();
        let json = serde_json::to_string(&profile).unwrap();

        // The JSON should contain credential_key but never a "password" field
        assert!(json.contains("credential_key"));
        assert!(!json.contains("\"password\""));

        // Verify round-trip
        let deserialized: ConnectionProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, profile.id);
        assert_eq!(deserialized.credential_key, profile.credential_key);
    }

    #[test]
    fn test_parse_engine_variants() {
        assert_eq!(parse_engine("postgresql").unwrap(), DatabaseEngine::PostgreSql);
        assert_eq!(parse_engine("postgres").unwrap(), DatabaseEngine::PostgreSql);
        assert_eq!(parse_engine("sqlserver").unwrap(), DatabaseEngine::SqlServer);
        assert_eq!(parse_engine("SQL Server").unwrap(), DatabaseEngine::SqlServer);
        assert_eq!(parse_engine("mssql").unwrap(), DatabaseEngine::SqlServer);
        assert_eq!(parse_engine("mysql").unwrap(), DatabaseEngine::MySql);
        assert_eq!(parse_engine("sqlite").unwrap(), DatabaseEngine::Sqlite);
        assert_eq!(parse_engine("oracle").unwrap(), DatabaseEngine::Oracle);
        assert_eq!(parse_engine("mongodb").unwrap(), DatabaseEngine::MongoDb);
        assert_eq!(parse_engine("cosmosdb").unwrap(), DatabaseEngine::CosmosDb);
        assert!(parse_engine("unknown").is_err());
    }

    #[test]
    fn test_overwrite_credential() {
        let store = InMemoryCredentialStore::new();
        store.store("key1", "first").unwrap();
        store.store("key1", "second").unwrap();
        assert_eq!(store.retrieve("key1").unwrap(), "second");
    }

    #[test]
    fn test_delete_nonexistent_key_is_ok() {
        let store = InMemoryCredentialStore::new();
        // Deleting a key that doesn't exist should not error
        store.delete("nonexistent").unwrap();
    }
}
