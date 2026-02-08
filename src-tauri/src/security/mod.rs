pub mod audit;
pub mod credentials;
pub mod profiles;
pub mod validation;

use serde::{Deserialize, Serialize};

// Re-export key types and functions for convenient access
pub use audit::{AuditError, AuditFilter, AuditLogger};
pub use credentials::{
    build_connection_config, credential_key_for_profile, CredentialEntry, CredentialError,
    CredentialStore, InMemoryCredentialStore,
};
pub use profiles::{ProfileError, ProfileStore};
pub use validation::{
    sanitize_for_display, validate_connection_string, validate_input, validate_table_name,
    ValidationError,
};

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: String,
    pub user: String,
    pub action: AuditAction,
    pub source_connection: Option<String>,
    pub target_connection: Option<String>,
    pub affected_rows: Option<i64>,
    pub details: Option<String>,
}

/// Types of auditable actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditAction {
    ConnectionCreated,
    ConnectionModified,
    ConnectionDeleted,
    ConnectionTested,
    SchemaCompared,
    DataCompared,
    MigrationStarted,
    MigrationCompleted,
    MigrationFailed,
    MigrationCancelled,
    JobCreated,
    JobExecuted,
    SettingsChanged,
}

/// Connection profile with encrypted credentials reference.
/// Note: This struct intentionally has NO password field.
/// Passwords are stored in Stronghold and referenced via `credential_key`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionProfile {
    pub id: String,
    pub name: String,
    pub engine: String,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub database: Option<String>,
    pub username: Option<String>,
    pub credential_key: Option<String>,
    pub file_path: Option<String>,
    pub read_only: bool,
    pub created_at: String,
    pub updated_at: String,
}
