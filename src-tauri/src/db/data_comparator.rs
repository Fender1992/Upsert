use serde::{Deserialize, Serialize};

use super::schema::Row;

/// Strategy for matching rows between source and target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchStrategy {
    PrimaryKey,
    CompositeKey(Vec<String>),
    CustomExpression(String),
    Fuzzy { threshold: f64 },
}

/// Configuration for data comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataCompareConfig {
    pub match_strategy: MatchStrategy,
    pub ignore_columns: Vec<String>,
    pub normalize_whitespace: bool,
    pub case_insensitive: bool,
    pub numeric_tolerance: Option<f64>,
    pub null_equals_empty: bool,
    pub use_hash_mode: bool,
    pub batch_size: usize,
}

impl Default for DataCompareConfig {
    fn default() -> Self {
        Self {
            match_strategy: MatchStrategy::PrimaryKey,
            ignore_columns: Vec::new(),
            normalize_whitespace: false,
            case_insensitive: false,
            numeric_tolerance: None,
            null_equals_empty: false,
            use_hash_mode: true,
            batch_size: 1000,
        }
    }
}

/// Result of comparing data between two tables
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataDiffResult {
    pub source_table: String,
    pub target_table: String,
    pub matched_rows: usize,
    pub inserted_rows: Vec<Row>,
    pub updated_rows: Vec<RowDiff>,
    pub deleted_rows: Vec<Row>,
    pub error_rows: Vec<RowError>,
}

/// A row that differs between source and target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowDiff {
    pub source_row: Row,
    pub target_row: Row,
    pub changed_columns: Vec<String>,
}

/// A row that caused an error during comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowError {
    pub row: Row,
    pub error: String,
}

/// Compare data between two sets of rows
pub fn compare_data(
    _source_rows: &[Row],
    _target_rows: &[Row],
    _config: &DataCompareConfig,
) -> DataDiffResult {
    todo!("Data comparison implementation")
}
