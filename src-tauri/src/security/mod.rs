use serde::{Deserialize, Serialize};

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

/// Connection profile with encrypted credentials reference
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

/// Validate IPC command input
pub fn validate_input(input: &str, max_length: usize) -> Result<(), String> {
    if input.len() > max_length {
        return Err(format!("Input exceeds maximum length of {}", max_length));
    }
    if input.contains('\0') {
        return Err("Input contains null bytes".to_string());
    }
    Ok(())
}
