use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use super::data_comparator::{compare_data, DataCompareConfig, DataDiffResult, MatchStrategy};
use super::schema::Row;

// ---------------------------------------------------------------------------
// Enums & configuration types
// ---------------------------------------------------------------------------

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
    /// Column name used for NewestWins conflict resolution (e.g. "updated_at")
    #[serde(default = "default_timestamp_column")]
    pub timestamp_column: Option<String>,
    /// Key columns used to match rows between source and target.
    /// When empty, falls back to the DataCompareConfig's match strategy.
    #[serde(default)]
    pub key_columns: Vec<String>,
}

fn default_timestamp_column() -> Option<String> {
    None
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
            timestamp_column: None,
            key_columns: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

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

impl MigrationResult {
    fn new() -> Self {
        Self {
            rows_inserted: 0,
            rows_updated: 0,
            rows_deleted: 0,
            rows_skipped: 0,
            errors: Vec::new(),
            duration_ms: 0,
            status: MigrationStatus::Pending,
        }
    }
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

// ---------------------------------------------------------------------------
// Migration plan
// ---------------------------------------------------------------------------

/// Describes what a migration will do before it is executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    /// Rows that will be inserted into the target
    pub rows_to_insert: Vec<Row>,
    /// Rows that will be updated in the target (source version of each row)
    pub rows_to_update: Vec<Row>,
    /// Rows that will be deleted from the target
    pub rows_to_delete: Vec<Row>,
    /// Rows flagged for manual review (when conflict resolution = ManualReview)
    pub rows_to_review: Vec<Row>,
    /// Total source row count
    pub source_row_count: usize,
    /// Total target row count
    pub target_row_count: usize,
    /// Number of batches that will be processed
    pub batch_count: usize,
    /// The migration mode being used
    pub mode: MigrationMode,
    /// Whether this is a dry-run plan
    pub dry_run: bool,
}

// ---------------------------------------------------------------------------
// Cancellation token  (lightweight, no extra crate features needed)
// ---------------------------------------------------------------------------

/// A simple cancellation token backed by an `AtomicBool`.
#[derive(Debug, Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Core public API
// ---------------------------------------------------------------------------

/// Build a comparison config from the migration config's key columns.
fn build_compare_config(config: &MigrationConfig) -> DataCompareConfig {
    let match_strategy = if config.key_columns.is_empty() {
        MatchStrategy::PrimaryKey
    } else {
        MatchStrategy::CompositeKey(config.key_columns.clone())
    };
    DataCompareConfig {
        match_strategy,
        ..Default::default()
    }
}

/// Produce a `DataDiffResult` that describes differences between source and target rows.
fn diff_rows(
    source_rows: &[Row],
    target_rows: &[Row],
    config: &MigrationConfig,
) -> DataDiffResult {
    let compare_config = build_compare_config(config);
    compare_data(source_rows, target_rows, &compare_config)
}

/// Create a migration plan without executing anything.
///
/// The plan describes which rows would be inserted, updated, deleted, or
/// flagged for review based on the chosen migration mode and conflict
/// resolution strategy.
pub fn plan_migration(
    source_rows: &[Row],
    target_rows: &[Row],
    config: &MigrationConfig,
) -> MigrationPlan {
    let diff = diff_rows(source_rows, target_rows, config);
    build_plan_from_diff(&diff, source_rows, target_rows, config)
}

/// Execute a migration according to the provided configuration.
///
/// When `config.dry_run` is `true` the function builds a plan and returns
/// a `MigrationResult` with zero counts and `Completed` status without
/// mutating the output vectors.
///
/// The function writes into `target_rows_out` which should be initialised
/// to the current target data.  After the call it will contain the
/// post-migration state of the target.
///
/// `cancel` is an optional cancellation token; when it fires, the
/// migration stops after the current batch and reports `Cancelled`.
pub fn execute_migration(
    source_rows: &[Row],
    target_rows: &[Row],
    config: &MigrationConfig,
    cancel: Option<&CancellationToken>,
) -> (MigrationResult, Vec<Row>) {
    let start = Instant::now();
    let mut result = MigrationResult::new();
    result.status = MigrationStatus::Running;

    // --- SchemaOnly: nothing to do data-wise ---
    if config.mode == MigrationMode::SchemaOnly {
        result.status = MigrationStatus::Completed;
        result.duration_ms = start.elapsed().as_millis() as u64;
        return (result, target_rows.to_vec());
    }

    // --- Compute diff ---
    let diff = diff_rows(source_rows, target_rows, config);
    let plan = build_plan_from_diff(&diff, source_rows, target_rows, config);

    // --- Dry-run: just report the plan without touching data ---
    if config.dry_run {
        result.rows_inserted = plan.rows_to_insert.len();
        result.rows_updated = plan.rows_to_update.len();
        result.rows_deleted = plan.rows_to_delete.len();
        result.rows_skipped = plan.rows_to_review.len();
        result.status = MigrationStatus::Completed;
        result.duration_ms = start.elapsed().as_millis() as u64;
        return (result, target_rows.to_vec());
    }

    // --- Build a mutable copy of target ---
    let mut output = target_rows.to_vec();
    let key_columns = effective_key_columns(config, source_rows);

    // Collect all operations into a flat list so we can batch them.
    let ops = build_operations(&plan, &key_columns);
    let batches = batch_operations(&ops, config.batch_size);

    for (batch_idx, batch) in batches.iter().enumerate() {
        // Check cancellation before each batch
        if let Some(token) = cancel {
            if token.is_cancelled() {
                result.status = MigrationStatus::Cancelled;
                result.duration_ms = start.elapsed().as_millis() as u64;
                return (result, output);
            }
        }

        // Retry loop per batch
        let mut attempts = 0u32;
        let max_attempts = config.retry_count.max(1);
        loop {
            attempts += 1;
            match apply_batch(batch, &mut output, &key_columns) {
                Ok(counts) => {
                    result.rows_inserted += counts.inserted;
                    result.rows_updated += counts.updated;
                    result.rows_deleted += counts.deleted;
                    result.rows_skipped += counts.skipped;
                    break;
                }
                Err(msg) => {
                    if attempts >= max_attempts {
                        result.errors.push(MigrationError {
                            batch_index: batch_idx,
                            row_index: None,
                            message: msg,
                            is_retryable: false,
                        });
                        break;
                    }
                    // Exponential back-off (only meaningful in real async
                    // scenarios; here we just record the attempt).
                    let _backoff =
                        config.retry_backoff_ms * 2u64.pow(attempts - 1);
                    // In a real implementation we would sleep here.
                    // For the data-layer we just retry immediately.
                }
            }
        }
    }

    // Review rows count as skipped
    result.rows_skipped += plan.rows_to_review.len();

    // Determine final status
    if result.errors.is_empty() {
        result.status = MigrationStatus::Completed;
    } else if config.auto_rollback {
        // Roll back: return original target unchanged
        result.status = MigrationStatus::RolledBack;
        result.duration_ms = start.elapsed().as_millis() as u64;
        return (result, target_rows.to_vec());
    } else {
        result.status = MigrationStatus::Failed;
    }

    result.duration_ms = start.elapsed().as_millis() as u64;
    (result, output)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Determine the effective key columns for matching rows.
fn effective_key_columns(config: &MigrationConfig, sample_rows: &[Row]) -> Vec<String> {
    if !config.key_columns.is_empty() {
        return config.key_columns.clone();
    }
    // Default heuristic: use "id" if present, else all columns.
    if let Some(first) = sample_rows.first() {
        if first.contains_key("id") {
            return vec!["id".to_string()];
        }
        let mut cols: Vec<String> = first.keys().cloned().collect();
        cols.sort();
        return cols;
    }
    vec![]
}

/// Build a `MigrationPlan` from a diff result, applying mode and conflict
/// resolution filters.
fn build_plan_from_diff(
    diff: &DataDiffResult,
    source_rows: &[Row],
    target_rows: &[Row],
    config: &MigrationConfig,
) -> MigrationPlan {
    let mut rows_to_insert: Vec<Row> = Vec::new();
    let mut rows_to_update: Vec<Row> = Vec::new();
    let mut rows_to_delete: Vec<Row> = Vec::new();
    let mut rows_to_review: Vec<Row> = Vec::new();

    match config.mode {
        MigrationMode::SchemaOnly => {
            // No data operations
        }
        MigrationMode::AppendOnly => {
            // Insert only; never update or delete
            rows_to_insert = diff.inserted_rows.clone();
        }
        MigrationMode::Merge => {
            // Insert new + update existing; never delete
            rows_to_insert = diff.inserted_rows.clone();
            resolve_updates(
                &diff,
                config,
                &mut rows_to_update,
                &mut rows_to_review,
            );
        }
        MigrationMode::Upsert => {
            // Insert new + update existing; no deletes
            rows_to_insert = diff.inserted_rows.clone();
            resolve_updates(
                &diff,
                config,
                &mut rows_to_update,
                &mut rows_to_review,
            );
        }
        MigrationMode::Mirror => {
            // Insert + update + delete to make target match source exactly
            rows_to_insert = diff.inserted_rows.clone();
            resolve_updates(
                &diff,
                config,
                &mut rows_to_update,
                &mut rows_to_review,
            );
            rows_to_delete = diff.deleted_rows.clone();
        }
    }

    let total_ops = rows_to_insert.len() + rows_to_update.len() + rows_to_delete.len();
    let batch_size = if config.batch_size == 0 {
        1
    } else {
        config.batch_size
    };
    let batch_count = if total_ops == 0 {
        0
    } else {
        (total_ops + batch_size - 1) / batch_size
    };

    MigrationPlan {
        rows_to_insert,
        rows_to_update,
        rows_to_delete,
        rows_to_review,
        source_row_count: source_rows.len(),
        target_row_count: target_rows.len(),
        batch_count,
        mode: config.mode.clone(),
        dry_run: config.dry_run,
    }
}

/// Apply the conflict resolution strategy to the updated rows from the diff
/// and push results into the appropriate output vectors.
fn resolve_updates(
    diff: &DataDiffResult,
    config: &MigrationConfig,
    rows_to_update: &mut Vec<Row>,
    rows_to_review: &mut Vec<Row>,
) {
    for row_diff in &diff.updated_rows {
        match &config.conflict_resolution {
            ConflictResolution::SourceWins => {
                rows_to_update.push(row_diff.source_row.clone());
            }
            ConflictResolution::TargetWins => {
                // Target already has the data; skip this row.
            }
            ConflictResolution::NewestWins => {
                let ts_col = config
                    .timestamp_column
                    .as_deref()
                    .unwrap_or("updated_at");
                let use_source = match (
                    row_diff.source_row.get(ts_col),
                    row_diff.target_row.get(ts_col),
                ) {
                    (Some(src_ts), Some(tgt_ts)) => {
                        // Compare as strings (ISO-8601 timestamps sort lexically)
                        let src_s = value_to_string(src_ts);
                        let tgt_s = value_to_string(tgt_ts);
                        src_s >= tgt_s
                    }
                    // If one side lacks the timestamp, default to source wins
                    _ => true,
                };
                if use_source {
                    rows_to_update.push(row_diff.source_row.clone());
                }
            }
            ConflictResolution::ManualReview => {
                rows_to_review.push(row_diff.source_row.clone());
            }
            ConflictResolution::CustomRules(rules) => {
                // Apply custom rules: for each rule string "column:source" or
                // "column:target" merge the winning value. Default to source.
                let merged = apply_custom_rules(
                    &row_diff.source_row,
                    &row_diff.target_row,
                    rules,
                );
                rows_to_update.push(merged);
            }
        }
    }
}

/// Merge two rows using custom per-column rules.
/// Rule format: "column_name:source" or "column_name:target".
/// Columns not mentioned default to the source value.
fn apply_custom_rules(source: &Row, target: &Row, rules: &[String]) -> Row {
    let mut merged = source.clone();
    let rule_map: HashMap<&str, &str> = rules
        .iter()
        .filter_map(|r| {
            let parts: Vec<&str> = r.splitn(2, ':').collect();
            if parts.len() == 2 {
                Some((parts[0], parts[1]))
            } else {
                None
            }
        })
        .collect();

    for (col, winner) in &rule_map {
        if *winner == "target" {
            if let Some(val) = target.get(*col) {
                merged.insert(col.to_string(), val.clone());
            }
        }
        // "source" is the default, no action needed
    }
    merged
}

/// Convert a JSON value to a string for timestamp comparison.
fn value_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Operations & batching
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum Operation {
    Insert(Row),
    Update(Row),
    Delete(Row),
}

/// Flatten the plan into a sequence of operations.
fn build_operations(plan: &MigrationPlan, _key_columns: &[String]) -> Vec<Operation> {
    let mut ops = Vec::new();
    for r in &plan.rows_to_insert {
        ops.push(Operation::Insert(r.clone()));
    }
    for r in &plan.rows_to_update {
        ops.push(Operation::Update(r.clone()));
    }
    for r in &plan.rows_to_delete {
        ops.push(Operation::Delete(r.clone()));
    }
    ops
}

/// Split a list of operations into fixed-size batches.
fn batch_operations(ops: &[Operation], batch_size: usize) -> Vec<Vec<Operation>> {
    let bs = if batch_size == 0 { 1 } else { batch_size };
    ops.chunks(bs).map(|c| c.to_vec()).collect()
}

/// Counts returned after applying a single batch.
struct BatchCounts {
    inserted: usize,
    updated: usize,
    deleted: usize,
    skipped: usize,
}

/// Apply a batch of operations to the mutable output vec.
fn apply_batch(
    ops: &[Operation],
    output: &mut Vec<Row>,
    key_columns: &[String],
) -> Result<BatchCounts, String> {
    let mut counts = BatchCounts {
        inserted: 0,
        updated: 0,
        deleted: 0,
        skipped: 0,
    };

    for op in ops {
        match op {
            Operation::Insert(row) => {
                output.push(row.clone());
                counts.inserted += 1;
            }
            Operation::Update(row) => {
                let key = build_row_key(row, key_columns);
                let mut found = false;
                for existing in output.iter_mut() {
                    if build_row_key(existing, key_columns) == key {
                        *existing = row.clone();
                        found = true;
                        break;
                    }
                }
                if found {
                    counts.updated += 1;
                } else {
                    counts.skipped += 1;
                }
            }
            Operation::Delete(row) => {
                let key = build_row_key(row, key_columns);
                let before = output.len();
                output.retain(|r| build_row_key(r, key_columns) != key);
                if output.len() < before {
                    counts.deleted += 1;
                } else {
                    counts.skipped += 1;
                }
            }
        }
    }

    Ok(counts)
}

/// Build a composite key string from a row.
fn build_row_key(row: &Row, key_columns: &[String]) -> String {
    key_columns
        .iter()
        .map(|col| {
            row.get(col)
                .map(|v| match v {
                    serde_json::Value::Null => "NULL".to_string(),
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    other => other.to_string(),
                })
                .unwrap_or_else(|| "NULL".to_string())
        })
        .collect::<Vec<_>>()
        .join("|")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Helper to create a row from key-value pairs.
    fn row(pairs: &[(&str, serde_json::Value)]) -> Row {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    fn default_config() -> MigrationConfig {
        MigrationConfig::default()
    }

    // -----------------------------------------------------------------------
    // 1. Plan - basic sanity
    // -----------------------------------------------------------------------
    #[test]
    fn test_plan_upsert_mode() {
        let source = vec![
            row(&[("id", json!(1)), ("name", json!("Alice"))]),
            row(&[("id", json!(2)), ("name", json!("Bob_new"))]),
            row(&[("id", json!(3)), ("name", json!("Charlie"))]),
        ];
        let target = vec![
            row(&[("id", json!(1)), ("name", json!("Alice"))]),
            row(&[("id", json!(2)), ("name", json!("Bob"))]),
        ];

        let config = default_config(); // Upsert mode
        let plan = plan_migration(&source, &target, &config);

        assert_eq!(plan.rows_to_insert.len(), 1); // id=3
        assert_eq!(plan.rows_to_update.len(), 1); // id=2
        assert!(plan.rows_to_delete.is_empty()); // upsert never deletes
        assert_eq!(plan.source_row_count, 3);
        assert_eq!(plan.target_row_count, 2);
        assert!(plan.batch_count > 0);
    }

    // -----------------------------------------------------------------------
    // 2. Mirror mode - includes deletes
    // -----------------------------------------------------------------------
    #[test]
    fn test_plan_mirror_mode() {
        let source = vec![
            row(&[("id", json!(1)), ("name", json!("Alice"))]),
        ];
        let target = vec![
            row(&[("id", json!(1)), ("name", json!("Alice"))]),
            row(&[("id", json!(2)), ("name", json!("Bob"))]),
        ];

        let config = MigrationConfig {
            mode: MigrationMode::Mirror,
            ..default_config()
        };
        let plan = plan_migration(&source, &target, &config);

        assert!(plan.rows_to_insert.is_empty());
        assert!(plan.rows_to_update.is_empty());
        assert_eq!(plan.rows_to_delete.len(), 1); // id=2 deleted
    }

    // -----------------------------------------------------------------------
    // 3. AppendOnly mode - no updates, no deletes
    // -----------------------------------------------------------------------
    #[test]
    fn test_plan_append_only_mode() {
        let source = vec![
            row(&[("id", json!(1)), ("name", json!("Alice_updated"))]),
            row(&[("id", json!(2)), ("name", json!("Bob"))]),
        ];
        let target = vec![
            row(&[("id", json!(1)), ("name", json!("Alice"))]),
        ];

        let config = MigrationConfig {
            mode: MigrationMode::AppendOnly,
            ..default_config()
        };
        let plan = plan_migration(&source, &target, &config);

        assert_eq!(plan.rows_to_insert.len(), 1); // id=2
        assert!(plan.rows_to_update.is_empty()); // no updates
        assert!(plan.rows_to_delete.is_empty()); // no deletes
    }

    // -----------------------------------------------------------------------
    // 4. Merge mode - inserts + updates, no deletes
    // -----------------------------------------------------------------------
    #[test]
    fn test_plan_merge_mode() {
        let source = vec![
            row(&[("id", json!(1)), ("name", json!("Alice_v2"))]),
            row(&[("id", json!(3)), ("name", json!("Charlie"))]),
        ];
        let target = vec![
            row(&[("id", json!(1)), ("name", json!("Alice"))]),
            row(&[("id", json!(2)), ("name", json!("Bob"))]),
        ];

        let config = MigrationConfig {
            mode: MigrationMode::Merge,
            ..default_config()
        };
        let plan = plan_migration(&source, &target, &config);

        assert_eq!(plan.rows_to_insert.len(), 1); // id=3
        assert_eq!(plan.rows_to_update.len(), 1); // id=1
        assert!(plan.rows_to_delete.is_empty()); // merge never deletes
    }

    // -----------------------------------------------------------------------
    // 5. SchemaOnly mode - nothing
    // -----------------------------------------------------------------------
    #[test]
    fn test_plan_schema_only_mode() {
        let source = vec![row(&[("id", json!(1))])];
        let target = vec![];

        let config = MigrationConfig {
            mode: MigrationMode::SchemaOnly,
            ..default_config()
        };
        let plan = plan_migration(&source, &target, &config);

        assert!(plan.rows_to_insert.is_empty());
        assert!(plan.rows_to_update.is_empty());
        assert!(plan.rows_to_delete.is_empty());
        assert_eq!(plan.batch_count, 0);
    }

    // -----------------------------------------------------------------------
    // 6. Execute upsert
    // -----------------------------------------------------------------------
    #[test]
    fn test_execute_upsert() {
        let source = vec![
            row(&[("id", json!(1)), ("val", json!("a_new"))]),
            row(&[("id", json!(2)), ("val", json!("b"))]),
            row(&[("id", json!(3)), ("val", json!("c"))]),
        ];
        let target = vec![
            row(&[("id", json!(1)), ("val", json!("a"))]),
            row(&[("id", json!(2)), ("val", json!("b"))]),
        ];

        let config = default_config();
        let (result, output) = execute_migration(&source, &target, &config, None);

        assert_eq!(result.status, MigrationStatus::Completed);
        assert_eq!(result.rows_inserted, 1); // id=3
        assert_eq!(result.rows_updated, 1); // id=1
        assert_eq!(result.rows_deleted, 0);
        assert_eq!(output.len(), 3);
    }

    // -----------------------------------------------------------------------
    // 7. Execute mirror
    // -----------------------------------------------------------------------
    #[test]
    fn test_execute_mirror() {
        let source = vec![
            row(&[("id", json!(1)), ("val", json!("a"))]),
        ];
        let target = vec![
            row(&[("id", json!(1)), ("val", json!("a"))]),
            row(&[("id", json!(2)), ("val", json!("b"))]),
            row(&[("id", json!(3)), ("val", json!("c"))]),
        ];

        let config = MigrationConfig {
            mode: MigrationMode::Mirror,
            ..default_config()
        };
        let (result, output) = execute_migration(&source, &target, &config, None);

        assert_eq!(result.status, MigrationStatus::Completed);
        assert_eq!(result.rows_deleted, 2); // id=2,3
        assert_eq!(output.len(), 1);
        assert_eq!(output[0].get("id"), Some(&json!(1)));
    }

    // -----------------------------------------------------------------------
    // 8. Execute append-only
    // -----------------------------------------------------------------------
    #[test]
    fn test_execute_append_only() {
        let source = vec![
            row(&[("id", json!(1)), ("val", json!("CHANGED"))]),
            row(&[("id", json!(2)), ("val", json!("new"))]),
        ];
        let target = vec![
            row(&[("id", json!(1)), ("val", json!("original"))]),
        ];

        let config = MigrationConfig {
            mode: MigrationMode::AppendOnly,
            ..default_config()
        };
        let (result, output) = execute_migration(&source, &target, &config, None);

        assert_eq!(result.rows_inserted, 1);
        assert_eq!(result.rows_updated, 0);
        assert_eq!(result.rows_deleted, 0);
        // Original row untouched
        assert_eq!(output[0].get("val"), Some(&json!("original")));
        assert_eq!(output.len(), 2);
    }

    // -----------------------------------------------------------------------
    // 9. Dry run
    // -----------------------------------------------------------------------
    #[test]
    fn test_dry_run_does_not_modify_data() {
        let source = vec![
            row(&[("id", json!(1)), ("val", json!("new"))]),
            row(&[("id", json!(2)), ("val", json!("b"))]),
        ];
        let target = vec![
            row(&[("id", json!(1)), ("val", json!("old"))]),
        ];

        let config = MigrationConfig {
            dry_run: true,
            ..default_config()
        };
        let (result, output) = execute_migration(&source, &target, &config, None);

        assert_eq!(result.status, MigrationStatus::Completed);
        assert_eq!(result.rows_inserted, 1);
        assert_eq!(result.rows_updated, 1);
        // Output is unchanged (dry run)
        assert_eq!(output.len(), 1);
        assert_eq!(output[0].get("val"), Some(&json!("old")));
    }

    // -----------------------------------------------------------------------
    // 10. Cancellation
    // -----------------------------------------------------------------------
    #[test]
    fn test_cancellation_stops_migration() {
        // Create enough rows that we get multiple batches
        let source: Vec<Row> = (0..100)
            .map(|i| row(&[("id", json!(i)), ("val", json!("x"))]))
            .collect();
        let target: Vec<Row> = vec![];

        let token = CancellationToken::new();
        // Cancel immediately
        token.cancel();

        let config = MigrationConfig {
            batch_size: 10,
            ..default_config()
        };
        let (result, _output) = execute_migration(&source, &target, &config, Some(&token));

        assert_eq!(result.status, MigrationStatus::Cancelled);
        // Nothing should have been processed because cancellation was checked
        // before the first batch
        assert_eq!(result.rows_inserted, 0);
    }

    // -----------------------------------------------------------------------
    // 11. Batching - small batch size
    // -----------------------------------------------------------------------
    #[test]
    fn test_batching_processes_all_rows() {
        let source: Vec<Row> = (0..25)
            .map(|i| row(&[("id", json!(i)), ("val", json!("s"))]))
            .collect();
        let target: Vec<Row> = vec![];

        let config = MigrationConfig {
            batch_size: 7,
            ..default_config()
        };
        let (result, output) = execute_migration(&source, &target, &config, None);

        assert_eq!(result.status, MigrationStatus::Completed);
        assert_eq!(result.rows_inserted, 25);
        assert_eq!(output.len(), 25);
    }

    // -----------------------------------------------------------------------
    // 12. Conflict resolution: TargetWins
    // -----------------------------------------------------------------------
    #[test]
    fn test_conflict_resolution_target_wins() {
        let source = vec![
            row(&[("id", json!(1)), ("val", json!("source_val"))]),
        ];
        let target = vec![
            row(&[("id", json!(1)), ("val", json!("target_val"))]),
        ];

        let config = MigrationConfig {
            conflict_resolution: ConflictResolution::TargetWins,
            ..default_config()
        };
        let (result, output) = execute_migration(&source, &target, &config, None);

        assert_eq!(result.rows_updated, 0);
        assert_eq!(output[0].get("val"), Some(&json!("target_val")));
    }

    // -----------------------------------------------------------------------
    // 13. Conflict resolution: NewestWins
    // -----------------------------------------------------------------------
    #[test]
    fn test_conflict_resolution_newest_wins_source_newer() {
        let source = vec![
            row(&[
                ("id", json!(1)),
                ("val", json!("source")),
                ("updated_at", json!("2025-06-01")),
            ]),
        ];
        let target = vec![
            row(&[
                ("id", json!(1)),
                ("val", json!("target")),
                ("updated_at", json!("2024-01-01")),
            ]),
        ];

        let config = MigrationConfig {
            conflict_resolution: ConflictResolution::NewestWins,
            ..default_config()
        };
        let (result, output) = execute_migration(&source, &target, &config, None);

        assert_eq!(result.rows_updated, 1);
        assert_eq!(output[0].get("val"), Some(&json!("source")));
    }

    #[test]
    fn test_conflict_resolution_newest_wins_target_newer() {
        let source = vec![
            row(&[
                ("id", json!(1)),
                ("val", json!("source")),
                ("updated_at", json!("2023-01-01")),
            ]),
        ];
        let target = vec![
            row(&[
                ("id", json!(1)),
                ("val", json!("target")),
                ("updated_at", json!("2025-12-31")),
            ]),
        ];

        let config = MigrationConfig {
            conflict_resolution: ConflictResolution::NewestWins,
            ..default_config()
        };
        let (result, output) = execute_migration(&source, &target, &config, None);

        assert_eq!(result.rows_updated, 0);
        assert_eq!(output[0].get("val"), Some(&json!("target")));
    }

    // -----------------------------------------------------------------------
    // 14. Conflict resolution: ManualReview
    // -----------------------------------------------------------------------
    #[test]
    fn test_conflict_resolution_manual_review() {
        let source = vec![
            row(&[("id", json!(1)), ("val", json!("src"))]),
        ];
        let target = vec![
            row(&[("id", json!(1)), ("val", json!("tgt"))]),
        ];

        let config = MigrationConfig {
            conflict_resolution: ConflictResolution::ManualReview,
            ..default_config()
        };
        let plan = plan_migration(&source, &target, &config);

        assert!(plan.rows_to_update.is_empty());
        assert_eq!(plan.rows_to_review.len(), 1);

        let (result, output) = execute_migration(&source, &target, &config, None);
        assert_eq!(result.rows_skipped, 1);
        // Target stays unchanged
        assert_eq!(output[0].get("val"), Some(&json!("tgt")));
    }

    // -----------------------------------------------------------------------
    // 15. Conflict resolution: CustomRules
    // -----------------------------------------------------------------------
    #[test]
    fn test_conflict_resolution_custom_rules() {
        let source = vec![
            row(&[
                ("id", json!(1)),
                ("name", json!("src_name")),
                ("email", json!("src@example.com")),
            ]),
        ];
        let target = vec![
            row(&[
                ("id", json!(1)),
                ("name", json!("tgt_name")),
                ("email", json!("tgt@example.com")),
            ]),
        ];

        // name from target, email from source (default)
        let config = MigrationConfig {
            conflict_resolution: ConflictResolution::CustomRules(vec![
                "name:target".to_string(),
                "email:source".to_string(),
            ]),
            ..default_config()
        };
        let (result, output) = execute_migration(&source, &target, &config, None);

        assert_eq!(result.rows_updated, 1);
        assert_eq!(output[0].get("name"), Some(&json!("tgt_name")));
        assert_eq!(output[0].get("email"), Some(&json!("src@example.com")));
    }

    // -----------------------------------------------------------------------
    // 16. Empty source and target
    // -----------------------------------------------------------------------
    #[test]
    fn test_empty_source_and_target() {
        let source: Vec<Row> = vec![];
        let target: Vec<Row> = vec![];

        let config = default_config();
        let (result, output) = execute_migration(&source, &target, &config, None);

        assert_eq!(result.status, MigrationStatus::Completed);
        assert_eq!(result.rows_inserted, 0);
        assert_eq!(result.rows_updated, 0);
        assert_eq!(result.rows_deleted, 0);
        assert!(output.is_empty());
    }

    // -----------------------------------------------------------------------
    // 17. SchemaOnly execute returns immediately
    // -----------------------------------------------------------------------
    #[test]
    fn test_execute_schema_only() {
        let source = vec![row(&[("id", json!(1))])];
        let target = vec![];

        let config = MigrationConfig {
            mode: MigrationMode::SchemaOnly,
            ..default_config()
        };
        let (result, output) = execute_migration(&source, &target, &config, None);

        assert_eq!(result.status, MigrationStatus::Completed);
        assert_eq!(result.rows_inserted, 0);
        assert!(output.is_empty());
    }

    // -----------------------------------------------------------------------
    // 18. Composite key columns
    // -----------------------------------------------------------------------
    #[test]
    fn test_composite_key_migration() {
        let source = vec![
            row(&[("tenant", json!("a")), ("id", json!(1)), ("val", json!("new"))]),
            row(&[("tenant", json!("b")), ("id", json!(1)), ("val", json!("b1"))]),
        ];
        let target = vec![
            row(&[("tenant", json!("a")), ("id", json!(1)), ("val", json!("old"))]),
        ];

        let config = MigrationConfig {
            key_columns: vec!["tenant".to_string(), "id".to_string()],
            ..default_config()
        };
        let (result, output) = execute_migration(&source, &target, &config, None);

        assert_eq!(result.rows_inserted, 1);
        assert_eq!(result.rows_updated, 1);
        assert_eq!(output.len(), 2);
    }

    // -----------------------------------------------------------------------
    // 19. Batch count in plan
    // -----------------------------------------------------------------------
    #[test]
    fn test_plan_batch_count() {
        let source: Vec<Row> = (0..15)
            .map(|i| row(&[("id", json!(i)), ("v", json!("x"))]))
            .collect();
        let target: Vec<Row> = vec![];

        let config = MigrationConfig {
            batch_size: 4,
            ..default_config()
        };
        let plan = plan_migration(&source, &target, &config);

        // 15 inserts / 4 per batch = 4 batches (3 full + 1 partial)
        assert_eq!(plan.batch_count, 4);
        assert_eq!(plan.rows_to_insert.len(), 15);
    }

    // -----------------------------------------------------------------------
    // 20. CancellationToken default is not cancelled
    // -----------------------------------------------------------------------
    #[test]
    fn test_cancellation_token_default() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
        token.cancel();
        assert!(token.is_cancelled());
    }

    // -----------------------------------------------------------------------
    // 21. Large migration with all identical rows
    // -----------------------------------------------------------------------
    #[test]
    fn test_no_changes_needed() {
        let rows: Vec<Row> = (0..50)
            .map(|i| row(&[("id", json!(i)), ("v", json!("same"))]))
            .collect();
        let source = rows.clone();
        let target = rows;

        let config = default_config();
        let (result, output) = execute_migration(&source, &target, &config, None);

        assert_eq!(result.status, MigrationStatus::Completed);
        assert_eq!(result.rows_inserted, 0);
        assert_eq!(result.rows_updated, 0);
        assert_eq!(result.rows_deleted, 0);
        assert_eq!(output.len(), 50);
    }
}
