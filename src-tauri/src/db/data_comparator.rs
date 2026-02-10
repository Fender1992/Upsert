use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

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
    source_rows: &[Row],
    target_rows: &[Row],
    config: &DataCompareConfig,
) -> DataDiffResult {
    let key_columns = resolve_key_columns(source_rows, &config.match_strategy);

    match &config.match_strategy {
        MatchStrategy::Fuzzy { threshold } => {
            compare_fuzzy(source_rows, target_rows, config, *threshold)
        }
        MatchStrategy::CustomExpression(expr) => {
            compare_custom_expression(source_rows, target_rows, config, expr)
        }
        _ => {
            // PrimaryKey and CompositeKey both use key-based matching
            if config.use_hash_mode {
                compare_with_hash(source_rows, target_rows, config, &key_columns)
            } else {
                compare_direct(source_rows, target_rows, config, &key_columns)
            }
        }
    }
}

/// Resolve key columns based on the match strategy.
/// For PrimaryKey, we look for columns named "id" or columns with "pk" or "primary" hints.
/// In production, this would come from the schema's is_primary_key field.
fn resolve_key_columns(rows: &[Row], strategy: &MatchStrategy) -> Vec<String> {
    match strategy {
        MatchStrategy::PrimaryKey => {
            // Use "id" as default PK column if present, otherwise use all columns
            if let Some(first_row) = rows.first() {
                if first_row.contains_key("id") {
                    return vec!["id".to_string()];
                }
                // Fall back to all columns as a composite key
                let mut cols: Vec<String> = first_row.keys().cloned().collect();
                cols.sort();
                cols
            } else {
                vec![]
            }
        }
        MatchStrategy::CompositeKey(cols) => cols.clone(),
        _ => vec![],
    }
}

/// Build a key string from a row using the specified key columns
fn build_row_key(row: &Row, key_columns: &[String]) -> String {
    key_columns
        .iter()
        .map(|col| {
            row.get(col)
                .map(value_to_key_string)
                .unwrap_or_else(|| "NULL".to_string())
        })
        .collect::<Vec<_>>()
        .join("|")
}

/// Convert a JSON value to a stable string representation for key matching
fn value_to_key_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}

/// Compute SHA-256 hash of a row's non-key, non-ignored columns for fast comparison
fn hash_row(row: &Row, key_columns: &[String], config: &DataCompareConfig) -> String {
    let mut hasher = Sha256::new();

    // Sort column names for deterministic hashing
    let mut columns: Vec<&String> = row
        .keys()
        .filter(|k| !key_columns.contains(k) && !config.ignore_columns.contains(k))
        .collect();
    columns.sort();

    for col in columns {
        if let Some(value) = row.get(col) {
            hasher.update(col.as_bytes());
            hasher.update(b"=");
            let val_str = normalize_value_for_comparison(value, config);
            hasher.update(val_str.as_bytes());
            hasher.update(b";");
        }
    }

    hex::encode(hasher.finalize())
}

/// Normalize a value for comparison, applying configured rules
fn normalize_value_for_comparison(value: &serde_json::Value, config: &DataCompareConfig) -> String {
    match value {
        serde_json::Value::Null => {
            if config.null_equals_empty {
                String::new()
            } else {
                "NULL".to_string()
            }
        }
        serde_json::Value::String(s) => {
            let mut result = s.clone();
            if config.normalize_whitespace {
                result = result.split_whitespace().collect::<Vec<_>>().join(" ");
            }
            if config.case_insensitive {
                result = result.to_lowercase();
            }
            if config.null_equals_empty && result.is_empty() {
                return String::new();
            }
            result
        }
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}

/// Compare two values taking into account ignore rules
fn values_equal(
    source: &serde_json::Value,
    target: &serde_json::Value,
    config: &DataCompareConfig,
) -> bool {
    // Handle null_equals_empty
    if config.null_equals_empty {
        let src_is_empty = matches!(source, serde_json::Value::Null)
            || matches!(source, serde_json::Value::String(s) if s.is_empty());
        let tgt_is_empty = matches!(target, serde_json::Value::Null)
            || matches!(target, serde_json::Value::String(s) if s.is_empty());
        if src_is_empty && tgt_is_empty {
            return true;
        }
    }

    // Handle numeric tolerance
    if let Some(tolerance) = config.numeric_tolerance {
        if let (Some(src_num), Some(tgt_num)) = (as_f64(source), as_f64(target)) {
            return (src_num - tgt_num).abs() <= tolerance;
        }
    }

    // Compare normalized string representations
    let src_norm = normalize_value_for_comparison(source, config);
    let tgt_norm = normalize_value_for_comparison(target, config);
    src_norm == tgt_norm
}

/// Try to extract an f64 from a JSON value
fn as_f64(value: &serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    }
}

/// Hash-based comparison: hash each row, compare hashes, then diff only mismatches
fn compare_with_hash(
    source_rows: &[Row],
    target_rows: &[Row],
    config: &DataCompareConfig,
    key_columns: &[String],
) -> DataDiffResult {
    // Index source rows by key
    let mut source_map: HashMap<String, &Row> = HashMap::new();
    let mut source_hashes: HashMap<String, String> = HashMap::new();
    for row in source_rows {
        let key = build_row_key(row, key_columns);
        source_hashes.insert(key.clone(), hash_row(row, key_columns, config));
        source_map.insert(key, row);
    }

    // Index target rows by key
    let mut target_map: HashMap<String, &Row> = HashMap::new();
    let mut target_hashes: HashMap<String, String> = HashMap::new();
    for row in target_rows {
        let key = build_row_key(row, key_columns);
        target_hashes.insert(key.clone(), hash_row(row, key_columns, config));
        target_map.insert(key, row);
    }

    let mut matched_rows = 0;
    let mut inserted_rows = Vec::new();
    let mut updated_rows = Vec::new();
    let mut deleted_rows = Vec::new();

    // Source rows not in target => inserted (in source but missing from target)
    for (key, src_row) in &source_map {
        if !target_map.contains_key(key) {
            inserted_rows.push((*src_row).clone());
        }
    }

    // Target rows not in source => deleted (in target but missing from source)
    for (key, tgt_row) in &target_map {
        if !source_map.contains_key(key) {
            deleted_rows.push((*tgt_row).clone());
        }
    }

    // Matched rows: compare hashes first, then detail differences
    for (key, src_row) in &source_map {
        if let Some(tgt_row) = target_map.get(key) {
            let src_hash = source_hashes.get(key).unwrap();
            let tgt_hash = target_hashes.get(key).unwrap();

            if src_hash == tgt_hash {
                matched_rows += 1;
            } else {
                // Hashes differ: do column-by-column comparison
                let changed = find_changed_columns(src_row, tgt_row, key_columns, config);
                if changed.is_empty() {
                    matched_rows += 1;
                } else {
                    updated_rows.push(RowDiff {
                        source_row: (*src_row).clone(),
                        target_row: (*tgt_row).clone(),
                        changed_columns: changed,
                    });
                }
            }
        }
    }

    DataDiffResult {
        source_table: String::new(),
        target_table: String::new(),
        matched_rows,
        inserted_rows,
        updated_rows,
        deleted_rows,
        error_rows: Vec::new(),
    }
}

/// Direct (non-hash) comparison: compare every matched row column-by-column
fn compare_direct(
    source_rows: &[Row],
    target_rows: &[Row],
    config: &DataCompareConfig,
    key_columns: &[String],
) -> DataDiffResult {
    let mut source_map: HashMap<String, &Row> = HashMap::new();
    for row in source_rows {
        let key = build_row_key(row, key_columns);
        source_map.insert(key, row);
    }

    let mut target_map: HashMap<String, &Row> = HashMap::new();
    for row in target_rows {
        let key = build_row_key(row, key_columns);
        target_map.insert(key, row);
    }

    let mut matched_rows = 0;
    let mut inserted_rows = Vec::new();
    let mut updated_rows = Vec::new();
    let mut deleted_rows = Vec::new();

    for (key, src_row) in &source_map {
        if !target_map.contains_key(key) {
            inserted_rows.push((*src_row).clone());
        }
    }

    for (key, tgt_row) in &target_map {
        if !source_map.contains_key(key) {
            deleted_rows.push((*tgt_row).clone());
        }
    }

    for (key, src_row) in &source_map {
        if let Some(tgt_row) = target_map.get(key) {
            let changed = find_changed_columns(src_row, tgt_row, key_columns, config);
            if changed.is_empty() {
                matched_rows += 1;
            } else {
                updated_rows.push(RowDiff {
                    source_row: (*src_row).clone(),
                    target_row: (*tgt_row).clone(),
                    changed_columns: changed,
                });
            }
        }
    }

    DataDiffResult {
        source_table: String::new(),
        target_table: String::new(),
        matched_rows,
        inserted_rows,
        updated_rows,
        deleted_rows,
        error_rows: Vec::new(),
    }
}

/// Fuzzy matching: for each source row, find the best matching target row by similarity
fn compare_fuzzy(
    source_rows: &[Row],
    target_rows: &[Row],
    config: &DataCompareConfig,
    threshold: f64,
) -> DataDiffResult {
    let mut matched_rows = 0;
    let mut inserted_rows = Vec::new();
    let mut updated_rows = Vec::new();
    let mut target_matched = vec![false; target_rows.len()];

    for src_row in source_rows {
        let mut best_idx = None;
        let mut best_score = 0.0f64;

        for (i, tgt_row) in target_rows.iter().enumerate() {
            if target_matched[i] {
                continue;
            }
            let score = row_similarity(src_row, tgt_row, config);
            if score > best_score {
                best_score = score;
                best_idx = Some(i);
            }
        }

        if best_score >= threshold {
            if let Some(idx) = best_idx {
                target_matched[idx] = true;
                let tgt_row = &target_rows[idx];
                let changed = find_changed_columns_all(src_row, tgt_row, config);
                if changed.is_empty() {
                    matched_rows += 1;
                } else {
                    updated_rows.push(RowDiff {
                        source_row: src_row.clone(),
                        target_row: tgt_row.clone(),
                        changed_columns: changed,
                    });
                }
            }
        } else {
            inserted_rows.push(src_row.clone());
        }
    }

    // Unmatched target rows are deleted
    let deleted_rows: Vec<Row> = target_rows
        .iter()
        .enumerate()
        .filter(|(i, _)| !target_matched[*i])
        .map(|(_, r)| r.clone())
        .collect();

    DataDiffResult {
        source_table: String::new(),
        target_table: String::new(),
        matched_rows,
        inserted_rows,
        updated_rows,
        deleted_rows,
        error_rows: Vec::new(),
    }
}

/// Compute row similarity as a ratio (0.0 to 1.0) of matching columns
fn row_similarity(source: &Row, target: &Row, config: &DataCompareConfig) -> f64 {
    let all_keys: std::collections::HashSet<&String> =
        source.keys().chain(target.keys()).collect();
    let relevant_keys: Vec<&&String> = all_keys
        .iter()
        .filter(|k| !config.ignore_columns.contains(k))
        .collect();

    if relevant_keys.is_empty() {
        return 1.0;
    }

    let matching = relevant_keys
        .iter()
        .filter(|col| {
            let src_val = source.get(col.as_str());
            let tgt_val = target.get(col.as_str());
            match (src_val, tgt_val) {
                (Some(s), Some(t)) => values_equal(s, t, config),
                (None, None) => true,
                _ => false,
            }
        })
        .count();

    matching as f64 / relevant_keys.len() as f64
}

/// Custom expression comparison - placeholder implementation
fn compare_custom_expression(
    source_rows: &[Row],
    target_rows: &[Row],
    _config: &DataCompareConfig,
    _expression: &str,
) -> DataDiffResult {
    // Custom expressions are not yet implemented; treat all rows as unmatched
    DataDiffResult {
        source_table: String::new(),
        target_table: String::new(),
        matched_rows: 0,
        inserted_rows: source_rows.to_vec(),
        updated_rows: Vec::new(),
        deleted_rows: target_rows.to_vec(),
        error_rows: vec![RowError {
            row: HashMap::new(),
            error: "CustomExpression matching is not yet implemented".to_string(),
        }],
    }
}

/// Find columns that differ between two matched rows, excluding key and ignored columns
fn find_changed_columns(
    source: &Row,
    target: &Row,
    key_columns: &[String],
    config: &DataCompareConfig,
) -> Vec<String> {
    let mut changed = Vec::new();

    // Check all columns in either row
    let all_keys: std::collections::HashSet<&String> =
        source.keys().chain(target.keys()).collect();

    for col in all_keys {
        if key_columns.contains(col) || config.ignore_columns.contains(col) {
            continue;
        }

        let src_val = source.get(col).unwrap_or(&serde_json::Value::Null);
        let tgt_val = target.get(col).unwrap_or(&serde_json::Value::Null);

        if !values_equal(src_val, tgt_val, config) {
            changed.push(col.clone());
        }
    }

    changed.sort();
    changed
}

/// Find changed columns between two rows, checking all non-ignored columns (no key exclusion)
fn find_changed_columns_all(
    source: &Row,
    target: &Row,
    config: &DataCompareConfig,
) -> Vec<String> {
    find_changed_columns(source, target, &[], config)
}

/// Compute simple Levenshtein distance between two strings
fn _levenshtein_distance(a: &str, b: &str) -> usize {
    let a_len = a.len();
    let b_len = b.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut prev_row: Vec<usize> = (0..=b_len).collect();
    let mut curr_row = vec![0; b_len + 1];

    for (i, a_char) in a.chars().enumerate() {
        curr_row[0] = i + 1;
        for (j, b_char) in b.chars().enumerate() {
            let cost = if a_char == b_char { 0 } else { 1 };
            curr_row[j + 1] = (prev_row[j + 1] + 1)
                .min(curr_row[j] + 1)
                .min(prev_row[j] + cost);
        }
        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[b_len]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Helper to create a row from key-value pairs
    fn row(pairs: &[(&str, serde_json::Value)]) -> Row {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    #[test]
    fn test_identical_rows() {
        let source = vec![
            row(&[("id", json!(1)), ("name", json!("Alice"))]),
            row(&[("id", json!(2)), ("name", json!("Bob"))]),
        ];
        let target = source.clone();
        let config = DataCompareConfig::default();

        let result = compare_data(&source, &target, &config);
        assert_eq!(result.matched_rows, 2);
        assert!(result.inserted_rows.is_empty());
        assert!(result.updated_rows.is_empty());
        assert!(result.deleted_rows.is_empty());
    }

    #[test]
    fn test_inserted_rows() {
        let source = vec![
            row(&[("id", json!(1)), ("name", json!("Alice"))]),
            row(&[("id", json!(2)), ("name", json!("Bob"))]),
            row(&[("id", json!(3)), ("name", json!("Charlie"))]),
        ];
        let target = vec![
            row(&[("id", json!(1)), ("name", json!("Alice"))]),
            row(&[("id", json!(2)), ("name", json!("Bob"))]),
        ];
        let config = DataCompareConfig::default();

        let result = compare_data(&source, &target, &config);
        assert_eq!(result.matched_rows, 2);
        assert_eq!(result.inserted_rows.len(), 1);
        assert_eq!(result.inserted_rows[0].get("id"), Some(&json!(3)));
    }

    #[test]
    fn test_deleted_rows() {
        let source = vec![row(&[("id", json!(1)), ("name", json!("Alice"))])];
        let target = vec![
            row(&[("id", json!(1)), ("name", json!("Alice"))]),
            row(&[("id", json!(2)), ("name", json!("Bob"))]),
        ];
        let config = DataCompareConfig::default();

        let result = compare_data(&source, &target, &config);
        assert_eq!(result.matched_rows, 1);
        assert_eq!(result.deleted_rows.len(), 1);
        assert_eq!(result.deleted_rows[0].get("id"), Some(&json!(2)));
    }

    #[test]
    fn test_updated_rows() {
        let source = vec![
            row(&[("id", json!(1)), ("name", json!("Alice")), ("email", json!("alice@old.com"))]),
        ];
        let target = vec![
            row(&[("id", json!(1)), ("name", json!("Alice")), ("email", json!("alice@new.com"))]),
        ];
        let config = DataCompareConfig::default();

        let result = compare_data(&source, &target, &config);
        assert_eq!(result.matched_rows, 0);
        assert_eq!(result.updated_rows.len(), 1);
        assert!(result.updated_rows[0].changed_columns.contains(&"email".to_string()));
    }

    #[test]
    fn test_composite_key() {
        let source = vec![
            row(&[("tenant", json!("a")), ("id", json!(1)), ("val", json!("x"))]),
            row(&[("tenant", json!("b")), ("id", json!(1)), ("val", json!("y"))]),
        ];
        let target = vec![
            row(&[("tenant", json!("a")), ("id", json!(1)), ("val", json!("x"))]),
            row(&[("tenant", json!("b")), ("id", json!(1)), ("val", json!("z"))]),
        ];
        let config = DataCompareConfig {
            match_strategy: MatchStrategy::CompositeKey(vec![
                "tenant".to_string(),
                "id".to_string(),
            ]),
            ..Default::default()
        };

        let result = compare_data(&source, &target, &config);
        assert_eq!(result.matched_rows, 1);
        assert_eq!(result.updated_rows.len(), 1);
        assert!(result.updated_rows[0].changed_columns.contains(&"val".to_string()));
    }

    #[test]
    fn test_ignore_columns() {
        let source = vec![
            row(&[("id", json!(1)), ("name", json!("Alice")), ("updated_at", json!("2024-01-01"))]),
        ];
        let target = vec![
            row(&[("id", json!(1)), ("name", json!("Alice")), ("updated_at", json!("2024-06-01"))]),
        ];
        let config = DataCompareConfig {
            ignore_columns: vec!["updated_at".to_string()],
            ..Default::default()
        };

        let result = compare_data(&source, &target, &config);
        assert_eq!(result.matched_rows, 1);
        assert!(result.updated_rows.is_empty());
    }

    #[test]
    fn test_case_insensitive() {
        let source = vec![row(&[("id", json!(1)), ("name", json!("Alice"))])];
        let target = vec![row(&[("id", json!(1)), ("name", json!("alice"))])];
        let config = DataCompareConfig {
            case_insensitive: true,
            ..Default::default()
        };

        let result = compare_data(&source, &target, &config);
        assert_eq!(result.matched_rows, 1);
        assert!(result.updated_rows.is_empty());
    }

    #[test]
    fn test_normalize_whitespace() {
        let source = vec![row(&[("id", json!(1)), ("desc", json!("hello  world"))])];
        let target = vec![row(&[("id", json!(1)), ("desc", json!("hello world"))])];
        let config = DataCompareConfig {
            normalize_whitespace: true,
            ..Default::default()
        };

        let result = compare_data(&source, &target, &config);
        assert_eq!(result.matched_rows, 1);
        assert!(result.updated_rows.is_empty());
    }

    #[test]
    fn test_numeric_tolerance() {
        let source = vec![row(&[("id", json!(1)), ("price", json!(9.99))])];
        let target = vec![row(&[("id", json!(1)), ("price", json!(9.991))])];
        let config = DataCompareConfig {
            numeric_tolerance: Some(0.01),
            ..Default::default()
        };

        let result = compare_data(&source, &target, &config);
        assert_eq!(result.matched_rows, 1);
        assert!(result.updated_rows.is_empty());
    }

    #[test]
    fn test_numeric_tolerance_exceeded() {
        let source = vec![row(&[("id", json!(1)), ("price", json!(9.99))])];
        let target = vec![row(&[("id", json!(1)), ("price", json!(10.50))])];
        let config = DataCompareConfig {
            numeric_tolerance: Some(0.01),
            ..Default::default()
        };

        let result = compare_data(&source, &target, &config);
        assert_eq!(result.updated_rows.len(), 1);
    }

    #[test]
    fn test_null_equals_empty() {
        let source = vec![row(&[("id", json!(1)), ("notes", json!(null))])];
        let target = vec![row(&[("id", json!(1)), ("notes", json!(""))])];
        let config = DataCompareConfig {
            null_equals_empty: true,
            ..Default::default()
        };

        let result = compare_data(&source, &target, &config);
        assert_eq!(result.matched_rows, 1);
        assert!(result.updated_rows.is_empty());
    }

    #[test]
    fn test_null_not_equals_empty_by_default() {
        let source = vec![row(&[("id", json!(1)), ("notes", json!(null))])];
        let target = vec![row(&[("id", json!(1)), ("notes", json!(""))])];
        let config = DataCompareConfig::default();

        let result = compare_data(&source, &target, &config);
        assert_eq!(result.updated_rows.len(), 1);
    }

    #[test]
    fn test_direct_mode_no_hash() {
        let source = vec![
            row(&[("id", json!(1)), ("name", json!("Alice"))]),
            row(&[("id", json!(2)), ("name", json!("Bob"))]),
        ];
        let target = vec![
            row(&[("id", json!(1)), ("name", json!("Alice_modified"))]),
            row(&[("id", json!(2)), ("name", json!("Bob"))]),
        ];
        let config = DataCompareConfig {
            use_hash_mode: false,
            ..Default::default()
        };

        let result = compare_data(&source, &target, &config);
        assert_eq!(result.matched_rows, 1);
        assert_eq!(result.updated_rows.len(), 1);
        assert!(result.updated_rows[0].changed_columns.contains(&"name".to_string()));
    }

    #[test]
    fn test_empty_source() {
        let source: Vec<Row> = vec![];
        let target = vec![row(&[("id", json!(1)), ("name", json!("Alice"))])];
        let config = DataCompareConfig::default();

        let result = compare_data(&source, &target, &config);
        assert_eq!(result.matched_rows, 0);
        assert_eq!(result.deleted_rows.len(), 1);
        assert!(result.inserted_rows.is_empty());
    }

    #[test]
    fn test_empty_target() {
        let source = vec![row(&[("id", json!(1)), ("name", json!("Alice"))])];
        let target: Vec<Row> = vec![];
        let config = DataCompareConfig::default();

        let result = compare_data(&source, &target, &config);
        assert_eq!(result.matched_rows, 0);
        assert_eq!(result.inserted_rows.len(), 1);
        assert!(result.deleted_rows.is_empty());
    }

    #[test]
    fn test_both_empty() {
        let source: Vec<Row> = vec![];
        let target: Vec<Row> = vec![];
        let config = DataCompareConfig::default();

        let result = compare_data(&source, &target, &config);
        assert_eq!(result.matched_rows, 0);
        assert!(result.inserted_rows.is_empty());
        assert!(result.deleted_rows.is_empty());
        assert!(result.updated_rows.is_empty());
    }

    #[test]
    fn test_fuzzy_matching() {
        let source = vec![
            row(&[("name", json!("Alice")), ("age", json!(30)), ("city", json!("NYC"))]),
            row(&[("name", json!("Bob")), ("age", json!(25)), ("city", json!("LA"))]),
        ];
        let target = vec![
            row(&[("name", json!("Alice")), ("age", json!(31)), ("city", json!("NYC"))]),
            row(&[("name", json!("Charlie")), ("age", json!(40)), ("city", json!("SF"))]),
        ];
        let config = DataCompareConfig {
            match_strategy: MatchStrategy::Fuzzy { threshold: 0.5 },
            ..Default::default()
        };

        let result = compare_data(&source, &target, &config);
        // Alice matches with changed age, Bob doesn't match Charlie well enough
        assert!(result.matched_rows + result.updated_rows.len() >= 1);
    }

    #[test]
    fn test_custom_expression_placeholder() {
        let source = vec![row(&[("id", json!(1))])];
        let target = vec![row(&[("id", json!(2))])];
        let config = DataCompareConfig {
            match_strategy: MatchStrategy::CustomExpression("id = id".to_string()),
            ..Default::default()
        };

        let result = compare_data(&source, &target, &config);
        // Custom expression is a placeholder; everything is unmatched
        assert_eq!(result.error_rows.len(), 1);
        assert!(result.error_rows[0].error.contains("not yet implemented"));
    }

    #[test]
    fn test_hash_consistency() {
        let r1 = row(&[("id", json!(1)), ("name", json!("Alice")), ("age", json!(30))]);
        let r2 = row(&[("id", json!(1)), ("name", json!("Alice")), ("age", json!(30))]);
        let config = DataCompareConfig::default();
        let key_cols = vec!["id".to_string()];

        let h1 = hash_row(&r1, &key_cols, &config);
        let h2 = hash_row(&r2, &key_cols, &config);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_differs_on_value_change() {
        let r1 = row(&[("id", json!(1)), ("name", json!("Alice"))]);
        let r2 = row(&[("id", json!(1)), ("name", json!("Bob"))]);
        let config = DataCompareConfig::default();
        let key_cols = vec!["id".to_string()];

        let h1 = hash_row(&r1, &key_cols, &config);
        let h2 = hash_row(&r2, &key_cols, &config);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_ignores_key_columns() {
        let r1 = row(&[("id", json!(1)), ("name", json!("Alice"))]);
        let r2 = row(&[("id", json!(999)), ("name", json!("Alice"))]);
        let config = DataCompareConfig::default();
        let key_cols = vec!["id".to_string()];

        let h1 = hash_row(&r1, &key_cols, &config);
        let h2 = hash_row(&r2, &key_cols, &config);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_ignores_ignored_columns() {
        let r1 = row(&[("id", json!(1)), ("name", json!("Alice")), ("ts", json!("2024-01-01"))]);
        let r2 = row(&[("id", json!(1)), ("name", json!("Alice")), ("ts", json!("2024-12-31"))]);
        let config = DataCompareConfig {
            ignore_columns: vec!["ts".to_string()],
            ..Default::default()
        };
        let key_cols = vec!["id".to_string()];

        let h1 = hash_row(&r1, &key_cols, &config);
        let h2 = hash_row(&r2, &key_cols, &config);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_mixed_inserts_updates_deletes() {
        let source = vec![
            row(&[("id", json!(1)), ("name", json!("Alice"))]),        // unchanged
            row(&[("id", json!(2)), ("name", json!("Bob_updated"))]),   // updated
            row(&[("id", json!(4)), ("name", json!("Dave"))]),          // inserted
        ];
        let target = vec![
            row(&[("id", json!(1)), ("name", json!("Alice"))]),
            row(&[("id", json!(2)), ("name", json!("Bob"))]),
            row(&[("id", json!(3)), ("name", json!("Charlie"))]),       // deleted
        ];
        let config = DataCompareConfig::default();

        let result = compare_data(&source, &target, &config);
        assert_eq!(result.matched_rows, 1);
        assert_eq!(result.updated_rows.len(), 1);
        assert_eq!(result.inserted_rows.len(), 1);
        assert_eq!(result.deleted_rows.len(), 1);
        assert_eq!(result.inserted_rows[0].get("id"), Some(&json!(4)));
        assert_eq!(result.deleted_rows[0].get("id"), Some(&json!(3)));
    }

    #[test]
    fn test_values_equal_basic() {
        let config = DataCompareConfig::default();
        assert!(values_equal(&json!(1), &json!(1), &config));
        assert!(!values_equal(&json!(1), &json!(2), &config));
        assert!(values_equal(&json!("hello"), &json!("hello"), &config));
        assert!(!values_equal(&json!("hello"), &json!("world"), &config));
        assert!(values_equal(&json!(null), &json!(null), &config));
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(_levenshtein_distance("", ""), 0);
        assert_eq!(_levenshtein_distance("abc", "abc"), 0);
        assert_eq!(_levenshtein_distance("abc", ""), 3);
        assert_eq!(_levenshtein_distance("", "abc"), 3);
        assert_eq!(_levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_row_similarity() {
        let config = DataCompareConfig::default();
        let r1 = row(&[("a", json!(1)), ("b", json!(2)), ("c", json!(3))]);
        let r2 = row(&[("a", json!(1)), ("b", json!(2)), ("c", json!(3))]);
        assert_eq!(row_similarity(&r1, &r2, &config), 1.0);

        let r3 = row(&[("a", json!(1)), ("b", json!(99)), ("c", json!(3))]);
        let sim = row_similarity(&r1, &r3, &config);
        assert!((sim - 2.0 / 3.0).abs() < 0.001);
    }
}
