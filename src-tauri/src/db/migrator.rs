use serde::{Deserialize, Serialize};

/// Migration mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MigrationMode {
    Upsert,
    Mirror,
    AppendOnly,
    Merge,
    SchemaOnly,
}

/// Conflict resolution strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConflictResolution {
    SourceWins,
    TargetWins,
    NewestWins,
    ManualReview,
    CustomRules(Vec<String>),
}

/// Configuration for a migration job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationConfig {
    pub mode: MigrationMode,
    pub conflict_resolution: ConflictResolution,
    pub batch_size: usize,
    pub transaction_mode: TransactionMode,
    pub retry_count: u32,
    pub retry_backoff_ms: u64,
    pub auto_rollback: bool,
    pub backup_before_migrate: bool,
    pub dry_run: bool,
}

/// Transaction scope for migration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionMode {
    PerBatch,
    WholeMigration,
    None,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            mode: MigrationMode::Upsert,
            conflict_resolution: ConflictResolution::SourceWins,
            batch_size: 1000,
            transaction_mode: TransactionMode::PerBatch,
            retry_count: 3,
            retry_backoff_ms: 1000,
            auto_rollback: true,
            backup_before_migrate: true,
            dry_run: false,
        }
    }
}

/// Result of a migration operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResult {
    pub rows_inserted: usize,
    pub rows_updated: usize,
    pub rows_deleted: usize,
    pub rows_skipped: usize,
    pub errors: Vec<MigrationError>,
    pub duration_ms: u64,
    pub status: MigrationStatus,
}

/// Status of a migration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MigrationStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    RolledBack,
}

/// A migration error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationError {
    pub batch_index: usize,
    pub row_index: Option<usize>,
    pub message: String,
    pub is_retryable: bool,
}
