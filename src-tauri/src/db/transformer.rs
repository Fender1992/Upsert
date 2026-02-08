use serde::{Deserialize, Serialize};

use super::schema::Row;

/// A transformation rule for data migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransformRule {
    RenameColumn { from: String, to: String },
    TypeCast { column: String, target_type: String },
    ValueMap { column: String, mappings: Vec<ValueMapping> },
    ComputedColumn { name: String, expression: String },
    DefaultForNull { column: String, default_value: serde_json::Value },
    RowFilter { expression: String },
    DropColumn { column: String },
}

/// A value mapping entry for lookup-based transforms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueMapping {
    pub source_value: serde_json::Value,
    pub target_value: serde_json::Value,
}

/// Configuration for a transform pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformPipeline {
    pub rules: Vec<TransformRule>,
}

impl TransformPipeline {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule(&mut self, rule: TransformRule) {
        self.rules.push(rule);
    }

    /// Apply all transformation rules to a set of rows.
    /// Rules are applied in order. Each rule transforms the entire row set
    /// before the next rule runs.
    pub fn apply(&self, rows: &[Row]) -> Vec<Row> {
        let mut current: Vec<Row> = rows.to_vec();

        for rule in &self.rules {
            current = match rule {
                TransformRule::RenameColumn { from, to } => {
                    apply_rename_column(current, from, to)
                }
                TransformRule::TypeCast { column, target_type } => {
                    apply_type_cast(current, column, target_type)
                }
                TransformRule::ValueMap { column, mappings } => {
                    apply_value_map(current, column, mappings)
                }
                TransformRule::ComputedColumn { name, expression } => {
                    apply_computed_column(current, name, expression)
                }
                TransformRule::DefaultForNull { column, default_value } => {
                    apply_default_for_null(current, column, default_value)
                }
                TransformRule::RowFilter { expression } => {
                    apply_row_filter(current, expression)
                }
                TransformRule::DropColumn { column } => {
                    apply_drop_column(current, column)
                }
            };
        }

        current
    }
}

impl Default for TransformPipeline {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Rule implementations
// ---------------------------------------------------------------------------

/// Rename key `from` to `to` in each row. If `from` does not exist, the row
/// is left unchanged.
fn apply_rename_column(mut rows: Vec<Row>, from: &str, to: &str) -> Vec<Row> {
    for row in &mut rows {
        if let Some(value) = row.remove(from) {
            row.insert(to.to_string(), value);
        }
    }
    rows
}

/// Convert column value to the requested target type.
///
/// Supported target types (case-insensitive): "string", "number", "boolean".
///
/// Conversion rules:
/// - **String**  : Number -> formatted string, Boolean -> "true"/"false",
///                  Null -> "null", anything else -> JSON serialization.
/// - **Number**  : String -> parse as f64, Boolean -> 0/1,
///                  Null left as null.
/// - **Boolean** : Number -> 0 = false / else true, String "true"/"false",
///                  Null left as null.
///
/// On parse failure the value is left unchanged (no error, no panic).
fn apply_type_cast(mut rows: Vec<Row>, column: &str, target_type: &str) -> Vec<Row> {
    let tt = target_type.to_lowercase();
    for row in &mut rows {
        if let Some(val) = row.get(column).cloned() {
            let converted = cast_value(&val, &tt);
            row.insert(column.to_string(), converted);
        }
    }
    rows
}

fn cast_value(val: &serde_json::Value, target: &str) -> serde_json::Value {
    use serde_json::Value;

    match target {
        "string" => match val {
            Value::String(_) => val.clone(),
            Value::Number(n) => Value::String(n.to_string()),
            Value::Bool(b) => Value::String(b.to_string()),
            Value::Null => Value::String("null".to_string()),
            other => Value::String(serde_json::to_string(other).unwrap_or_default()),
        },
        "number" => match val {
            Value::Number(_) => val.clone(),
            Value::String(s) => {
                if let Ok(n) = s.parse::<i64>() {
                    Value::Number(serde_json::Number::from(n))
                } else if let Ok(f) = s.parse::<f64>() {
                    serde_json::Number::from_f64(f)
                        .map(Value::Number)
                        .unwrap_or_else(|| val.clone())
                } else {
                    // parse failure – leave unchanged
                    val.clone()
                }
            }
            Value::Bool(b) => {
                Value::Number(serde_json::Number::from(if *b { 1 } else { 0 }))
            }
            Value::Null => Value::Null,
            _ => val.clone(),
        },
        "boolean" => match val {
            Value::Bool(_) => val.clone(),
            Value::Number(n) => {
                let zero = n.as_f64().map(|f| f == 0.0).unwrap_or(false);
                Value::Bool(!zero)
            }
            Value::String(s) => match s.to_lowercase().as_str() {
                "true" | "1" | "yes" => Value::Bool(true),
                "false" | "0" | "no" => Value::Bool(false),
                _ => val.clone(), // leave unchanged on unrecognised string
            },
            Value::Null => Value::Null,
            _ => val.clone(),
        },
        _ => val.clone(), // unknown target type – no change
    }
}

/// Replace values that match a source_value in the mappings list with the
/// corresponding target_value. Unmatched values pass through unchanged.
fn apply_value_map(mut rows: Vec<Row>, column: &str, mappings: &[ValueMapping]) -> Vec<Row> {
    for row in &mut rows {
        if let Some(val) = row.get(column).cloned() {
            for mapping in mappings {
                if val == mapping.source_value {
                    row.insert(column.to_string(), mapping.target_value.clone());
                    break;
                }
            }
        }
    }
    rows
}

/// Add a computed column to every row.
///
/// Supported expression forms:
/// - `'literal_value'` – a string literal (single-quoted)
/// - `col1 || col2`    – concatenation of two column values (as strings)
/// - `column_name`     – copy value from another column
fn apply_computed_column(mut rows: Vec<Row>, name: &str, expression: &str) -> Vec<Row> {
    let expr = expression.trim();

    // 1. String literal: 'some value'
    if expr.starts_with('\'') && expr.ends_with('\'') && expr.len() >= 2 {
        let literal = &expr[1..expr.len() - 1];
        for row in &mut rows {
            row.insert(name.to_string(), serde_json::Value::String(literal.to_string()));
        }
        return rows;
    }

    // 2. Concatenation: col1 || col2
    if let Some(pos) = expr.find("||") {
        let left = expr[..pos].trim();
        let right = expr[pos + 2..].trim();
        for row in &mut rows {
            let left_val = value_to_string(row.get(left));
            let right_val = value_to_string(row.get(right));
            row.insert(
                name.to_string(),
                serde_json::Value::String(format!("{}{}", left_val, right_val)),
            );
        }
        return rows;
    }

    // 3. Column reference
    for row in &mut rows {
        let val = row.get(expr).cloned().unwrap_or(serde_json::Value::Null);
        row.insert(name.to_string(), val);
    }
    rows
}

/// Helper: convert a JSON value to a string for concatenation purposes.
fn value_to_string(val: Option<&serde_json::Value>) -> String {
    match val {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::Bool(b)) => b.to_string(),
        Some(serde_json::Value::Null) | None => String::new(),
        Some(other) => serde_json::to_string(other).unwrap_or_default(),
    }
}

/// If the column value is null (or missing), replace it with the provided
/// default.
fn apply_default_for_null(
    mut rows: Vec<Row>,
    column: &str,
    default_value: &serde_json::Value,
) -> Vec<Row> {
    for row in &mut rows {
        let is_null_or_missing = row
            .get(column)
            .map(|v| v.is_null())
            .unwrap_or(true);
        if is_null_or_missing {
            row.insert(column.to_string(), default_value.clone());
        }
    }
    rows
}

/// Filter rows based on a simple expression.
///
/// Supported expressions:
/// - `column_name IS NOT NULL`
/// - `column_name IS NULL`
/// - `column_name = 'value'`
/// - `column_name != 'value'`
fn apply_row_filter(rows: Vec<Row>, expression: &str) -> Vec<Row> {
    let expr = expression.trim();

    // IS NOT NULL  (check before IS NULL so we don't match the shorter form first)
    if let Some(col) = expr.strip_suffix(" IS NOT NULL") {
        let col = col.trim();
        return rows
            .into_iter()
            .filter(|row| {
                row.get(col).map(|v| !v.is_null()).unwrap_or(false)
            })
            .collect();
    }

    // IS NULL
    if let Some(col) = expr.strip_suffix(" IS NULL") {
        let col = col.trim();
        return rows
            .into_iter()
            .filter(|row| {
                row.get(col).map(|v| v.is_null()).unwrap_or(true)
            })
            .collect();
    }

    // != 'value'
    if let Some((col, val)) = parse_comparison(expr, "!=") {
        return rows
            .into_iter()
            .filter(|row| {
                row.get(&col)
                    .map(|v| v != &serde_json::Value::String(val.clone()))
                    .unwrap_or(true)
            })
            .collect();
    }

    // = 'value'
    if let Some((col, val)) = parse_comparison(expr, "=") {
        return rows
            .into_iter()
            .filter(|row| {
                row.get(&col)
                    .map(|v| v == &serde_json::Value::String(val.clone()))
                    .unwrap_or(false)
            })
            .collect();
    }

    // Unknown expression – return all rows unchanged
    rows
}

/// Parse `"column_name OP 'value'"` into `(column_name, value)`.
fn parse_comparison(expr: &str, op: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = expr.splitn(2, op).collect();
    if parts.len() != 2 {
        return None;
    }
    let col = parts[0].trim().to_string();
    let val_raw = parts[1].trim();
    // strip surrounding single quotes
    if val_raw.starts_with('\'') && val_raw.ends_with('\'') && val_raw.len() >= 2 {
        let val = val_raw[1..val_raw.len() - 1].to_string();
        Some((col, val))
    } else {
        None
    }
}

/// Remove the named column from every row.
fn apply_drop_column(mut rows: Vec<Row>, column: &str) -> Vec<Row> {
    for row in &mut rows {
        row.remove(column);
    }
    rows
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    /// Helper: build a single-row vec from a JSON object literal.
    fn rows_from_json(vals: Vec<serde_json::Value>) -> Vec<Row> {
        vals.into_iter()
            .map(|v| {
                if let serde_json::Value::Object(map) = v {
                    map.into_iter().collect::<HashMap<String, serde_json::Value>>()
                } else {
                    panic!("expected JSON object");
                }
            })
            .collect()
    }

    // -----------------------------------------------------------------------
    // RenameColumn
    // -----------------------------------------------------------------------

    #[test]
    fn test_rename_column_basic() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::RenameColumn {
            from: "old_name".into(),
            to: "new_name".into(),
        });
        let rows = rows_from_json(vec![json!({"old_name": "hello", "other": 1})]);
        let result = pipeline.apply(&rows);
        assert_eq!(result[0].get("new_name"), Some(&json!("hello")));
        assert!(!result[0].contains_key("old_name"));
        assert_eq!(result[0].get("other"), Some(&json!(1)));
    }

    #[test]
    fn test_rename_column_missing_key() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::RenameColumn {
            from: "nonexistent".into(),
            to: "target".into(),
        });
        let rows = rows_from_json(vec![json!({"a": 1})]);
        let result = pipeline.apply(&rows);
        // Row should be unchanged
        assert_eq!(result[0].get("a"), Some(&json!(1)));
        assert!(!result[0].contains_key("target"));
    }

    // -----------------------------------------------------------------------
    // TypeCast
    // -----------------------------------------------------------------------

    #[test]
    fn test_typecast_string_to_number() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::TypeCast {
            column: "val".into(),
            target_type: "number".into(),
        });
        let rows = rows_from_json(vec![json!({"val": "42"})]);
        let result = pipeline.apply(&rows);
        assert_eq!(result[0].get("val"), Some(&json!(42)));
    }

    #[test]
    fn test_typecast_string_to_number_float() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::TypeCast {
            column: "val".into(),
            target_type: "number".into(),
        });
        let rows = rows_from_json(vec![json!({"val": "3.14"})]);
        let result = pipeline.apply(&rows);
        assert_eq!(result[0].get("val"), Some(&json!(3.14)));
    }

    #[test]
    fn test_typecast_number_to_string() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::TypeCast {
            column: "val".into(),
            target_type: "string".into(),
        });
        let rows = rows_from_json(vec![json!({"val": 99})]);
        let result = pipeline.apply(&rows);
        assert_eq!(result[0].get("val"), Some(&json!("99")));
    }

    #[test]
    fn test_typecast_bool_to_number() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::TypeCast {
            column: "flag".into(),
            target_type: "number".into(),
        });
        let rows = rows_from_json(vec![
            json!({"flag": true}),
            json!({"flag": false}),
        ]);
        let result = pipeline.apply(&rows);
        assert_eq!(result[0].get("flag"), Some(&json!(1)));
        assert_eq!(result[1].get("flag"), Some(&json!(0)));
    }

    #[test]
    fn test_typecast_number_to_boolean() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::TypeCast {
            column: "val".into(),
            target_type: "boolean".into(),
        });
        let rows = rows_from_json(vec![
            json!({"val": 0}),
            json!({"val": 5}),
        ]);
        let result = pipeline.apply(&rows);
        assert_eq!(result[0].get("val"), Some(&json!(false)));
        assert_eq!(result[1].get("val"), Some(&json!(true)));
    }

    #[test]
    fn test_typecast_invalid_string_to_number_leaves_unchanged() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::TypeCast {
            column: "val".into(),
            target_type: "number".into(),
        });
        let rows = rows_from_json(vec![json!({"val": "not_a_number"})]);
        let result = pipeline.apply(&rows);
        assert_eq!(result[0].get("val"), Some(&json!("not_a_number")));
    }

    #[test]
    fn test_typecast_null_to_string() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::TypeCast {
            column: "val".into(),
            target_type: "string".into(),
        });
        let rows = rows_from_json(vec![json!({"val": null})]);
        let result = pipeline.apply(&rows);
        assert_eq!(result[0].get("val"), Some(&json!("null")));
    }

    #[test]
    fn test_typecast_null_to_number_stays_null() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::TypeCast {
            column: "val".into(),
            target_type: "number".into(),
        });
        let rows = rows_from_json(vec![json!({"val": null})]);
        let result = pipeline.apply(&rows);
        assert_eq!(result[0].get("val"), Some(&json!(null)));
    }

    // -----------------------------------------------------------------------
    // ValueMap
    // -----------------------------------------------------------------------

    #[test]
    fn test_value_map_basic() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::ValueMap {
            column: "status".into(),
            mappings: vec![
                ValueMapping { source_value: json!("A"), target_value: json!("Active") },
                ValueMapping { source_value: json!("I"), target_value: json!("Inactive") },
            ],
        });
        let rows = rows_from_json(vec![
            json!({"status": "A"}),
            json!({"status": "I"}),
            json!({"status": "X"}),
        ]);
        let result = pipeline.apply(&rows);
        assert_eq!(result[0].get("status"), Some(&json!("Active")));
        assert_eq!(result[1].get("status"), Some(&json!("Inactive")));
        assert_eq!(result[2].get("status"), Some(&json!("X"))); // unmatched passes through
    }

    #[test]
    fn test_value_map_numeric() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::ValueMap {
            column: "code".into(),
            mappings: vec![
                ValueMapping { source_value: json!(1), target_value: json!("one") },
            ],
        });
        let rows = rows_from_json(vec![json!({"code": 1}), json!({"code": 2})]);
        let result = pipeline.apply(&rows);
        assert_eq!(result[0].get("code"), Some(&json!("one")));
        assert_eq!(result[1].get("code"), Some(&json!(2)));
    }

    // -----------------------------------------------------------------------
    // ComputedColumn
    // -----------------------------------------------------------------------

    #[test]
    fn test_computed_column_literal() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::ComputedColumn {
            name: "source".into(),
            expression: "'migration_v1'".into(),
        });
        let rows = rows_from_json(vec![json!({"a": 1})]);
        let result = pipeline.apply(&rows);
        assert_eq!(result[0].get("source"), Some(&json!("migration_v1")));
    }

    #[test]
    fn test_computed_column_reference() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::ComputedColumn {
            name: "copy_of_a".into(),
            expression: "a".into(),
        });
        let rows = rows_from_json(vec![json!({"a": 42})]);
        let result = pipeline.apply(&rows);
        assert_eq!(result[0].get("copy_of_a"), Some(&json!(42)));
    }

    #[test]
    fn test_computed_column_concat() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::ComputedColumn {
            name: "full_name".into(),
            expression: "first || last".into(),
        });
        let rows = rows_from_json(vec![json!({"first": "John", "last": "Doe"})]);
        let result = pipeline.apply(&rows);
        assert_eq!(result[0].get("full_name"), Some(&json!("JohnDoe")));
    }

    #[test]
    fn test_computed_column_reference_missing() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::ComputedColumn {
            name: "missing".into(),
            expression: "nonexistent".into(),
        });
        let rows = rows_from_json(vec![json!({"a": 1})]);
        let result = pipeline.apply(&rows);
        assert_eq!(result[0].get("missing"), Some(&json!(null)));
    }

    // -----------------------------------------------------------------------
    // DefaultForNull
    // -----------------------------------------------------------------------

    #[test]
    fn test_default_for_null_replaces_null() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::DefaultForNull {
            column: "val".into(),
            default_value: json!("N/A"),
        });
        let rows = rows_from_json(vec![json!({"val": null})]);
        let result = pipeline.apply(&rows);
        assert_eq!(result[0].get("val"), Some(&json!("N/A")));
    }

    #[test]
    fn test_default_for_null_does_not_replace_non_null() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::DefaultForNull {
            column: "val".into(),
            default_value: json!("N/A"),
        });
        let rows = rows_from_json(vec![json!({"val": "existing"})]);
        let result = pipeline.apply(&rows);
        assert_eq!(result[0].get("val"), Some(&json!("existing")));
    }

    #[test]
    fn test_default_for_null_missing_column() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::DefaultForNull {
            column: "missing".into(),
            default_value: json!(0),
        });
        let rows = rows_from_json(vec![json!({"a": 1})]);
        let result = pipeline.apply(&rows);
        assert_eq!(result[0].get("missing"), Some(&json!(0)));
    }

    // -----------------------------------------------------------------------
    // RowFilter
    // -----------------------------------------------------------------------

    #[test]
    fn test_row_filter_is_not_null() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::RowFilter {
            expression: "name IS NOT NULL".into(),
        });
        let rows = rows_from_json(vec![
            json!({"name": "Alice"}),
            json!({"name": null}),
            json!({"other": 1}),
        ]);
        let result = pipeline.apply(&rows);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get("name"), Some(&json!("Alice")));
    }

    #[test]
    fn test_row_filter_is_null() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::RowFilter {
            expression: "name IS NULL".into(),
        });
        let rows = rows_from_json(vec![
            json!({"name": "Alice"}),
            json!({"name": null}),
            json!({"other": 1}), // missing column -> treated as null
        ]);
        let result = pipeline.apply(&rows);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_row_filter_equals() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::RowFilter {
            expression: "status = 'active'".into(),
        });
        let rows = rows_from_json(vec![
            json!({"status": "active"}),
            json!({"status": "inactive"}),
            json!({"status": "active"}),
        ]);
        let result = pipeline.apply(&rows);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_row_filter_not_equals() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::RowFilter {
            expression: "status != 'deleted'".into(),
        });
        let rows = rows_from_json(vec![
            json!({"status": "active"}),
            json!({"status": "deleted"}),
        ]);
        let result = pipeline.apply(&rows);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get("status"), Some(&json!("active")));
    }

    // -----------------------------------------------------------------------
    // DropColumn
    // -----------------------------------------------------------------------

    #[test]
    fn test_drop_column_basic() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::DropColumn {
            column: "secret".into(),
        });
        let rows = rows_from_json(vec![json!({"id": 1, "secret": "pw123"})]);
        let result = pipeline.apply(&rows);
        assert!(!result[0].contains_key("secret"));
        assert_eq!(result[0].get("id"), Some(&json!(1)));
    }

    #[test]
    fn test_drop_column_missing() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::DropColumn {
            column: "nonexistent".into(),
        });
        let rows = rows_from_json(vec![json!({"a": 1})]);
        let result = pipeline.apply(&rows);
        assert_eq!(result[0].get("a"), Some(&json!(1)));
    }

    // -----------------------------------------------------------------------
    // Empty rows
    // -----------------------------------------------------------------------

    #[test]
    fn test_empty_rows() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::RenameColumn {
            from: "a".into(),
            to: "b".into(),
        });
        let result = pipeline.apply(&[]);
        assert!(result.is_empty());
    }

    // -----------------------------------------------------------------------
    // Combined / ordering tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_combined_pipeline() {
        let mut pipeline = TransformPipeline::new();
        // 1. Filter out nulls
        pipeline.add_rule(TransformRule::RowFilter {
            expression: "name IS NOT NULL".into(),
        });
        // 2. Rename column
        pipeline.add_rule(TransformRule::RenameColumn {
            from: "name".into(),
            to: "full_name".into(),
        });
        // 3. Add literal column
        pipeline.add_rule(TransformRule::ComputedColumn {
            name: "source".into(),
            expression: "'legacy'".into(),
        });
        // 4. Drop another column
        pipeline.add_rule(TransformRule::DropColumn {
            column: "tmp".into(),
        });

        let rows = rows_from_json(vec![
            json!({"name": "Alice", "tmp": "x"}),
            json!({"name": null, "tmp": "y"}),
            json!({"name": "Bob", "tmp": "z"}),
        ]);
        let result = pipeline.apply(&rows);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].get("full_name"), Some(&json!("Alice")));
        assert_eq!(result[0].get("source"), Some(&json!("legacy")));
        assert!(!result[0].contains_key("tmp"));
        assert!(!result[0].contains_key("name"));
    }

    #[test]
    fn test_ordering_rename_then_drop() {
        // Rename `a` -> `b`, then drop `b`. Column should disappear.
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::RenameColumn {
            from: "a".into(),
            to: "b".into(),
        });
        pipeline.add_rule(TransformRule::DropColumn {
            column: "b".into(),
        });

        let rows = rows_from_json(vec![json!({"a": 1, "c": 2})]);
        let result = pipeline.apply(&rows);
        assert!(!result[0].contains_key("a"));
        assert!(!result[0].contains_key("b"));
        assert_eq!(result[0].get("c"), Some(&json!(2)));
    }

    #[test]
    fn test_ordering_drop_then_rename() {
        // Drop `a`, then rename `a` -> `b`.
        // Since `a` is already gone, rename is a no-op.
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::DropColumn {
            column: "a".into(),
        });
        pipeline.add_rule(TransformRule::RenameColumn {
            from: "a".into(),
            to: "b".into(),
        });

        let rows = rows_from_json(vec![json!({"a": 1, "c": 2})]);
        let result = pipeline.apply(&rows);
        assert!(!result[0].contains_key("a"));
        assert!(!result[0].contains_key("b"));
        assert_eq!(result[0].get("c"), Some(&json!(2)));
    }

    #[test]
    fn test_typecast_then_value_map() {
        // Cast "1" -> 1, then map 1 -> "one"
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::TypeCast {
            column: "val".into(),
            target_type: "number".into(),
        });
        pipeline.add_rule(TransformRule::ValueMap {
            column: "val".into(),
            mappings: vec![ValueMapping {
                source_value: json!(1),
                target_value: json!("one"),
            }],
        });

        let rows = rows_from_json(vec![json!({"val": "1"})]);
        let result = pipeline.apply(&rows);
        assert_eq!(result[0].get("val"), Some(&json!("one")));
    }

    #[test]
    fn test_default_then_filter() {
        // Set default for null, then filter on that column value.
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::DefaultForNull {
            column: "status".into(),
            default_value: json!("unknown"),
        });
        pipeline.add_rule(TransformRule::RowFilter {
            expression: "status != 'unknown'".into(),
        });

        let rows = rows_from_json(vec![
            json!({"status": "active"}),
            json!({"status": null}),
            json!({"other": 1}),
        ]);
        let result = pipeline.apply(&rows);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get("status"), Some(&json!("active")));
    }

    #[test]
    fn test_no_rules_returns_clone() {
        let pipeline = TransformPipeline::new();
        let rows = rows_from_json(vec![json!({"a": 1})]);
        let result = pipeline.apply(&rows);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get("a"), Some(&json!(1)));
    }

    #[test]
    fn test_typecast_string_to_boolean() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::TypeCast {
            column: "val".into(),
            target_type: "boolean".into(),
        });
        let rows = rows_from_json(vec![
            json!({"val": "true"}),
            json!({"val": "false"}),
            json!({"val": "yes"}),
            json!({"val": "random"}),
        ]);
        let result = pipeline.apply(&rows);
        assert_eq!(result[0].get("val"), Some(&json!(true)));
        assert_eq!(result[1].get("val"), Some(&json!(false)));
        assert_eq!(result[2].get("val"), Some(&json!(true)));
        // Unrecognised string left unchanged
        assert_eq!(result[3].get("val"), Some(&json!("random")));
    }

    #[test]
    fn test_computed_column_concat_with_numbers() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::ComputedColumn {
            name: "label".into(),
            expression: "prefix || id".into(),
        });
        let rows = rows_from_json(vec![json!({"prefix": "item_", "id": 42})]);
        let result = pipeline.apply(&rows);
        assert_eq!(result[0].get("label"), Some(&json!("item_42")));
    }

    #[test]
    fn test_row_filter_equals_missing_column() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::RowFilter {
            expression: "status = 'active'".into(),
        });
        // Row doesn't have "status" at all
        let rows = rows_from_json(vec![json!({"other": 1})]);
        let result = pipeline.apply(&rows);
        assert!(result.is_empty());
    }

    #[test]
    fn test_row_filter_not_equals_missing_column() {
        let mut pipeline = TransformPipeline::new();
        pipeline.add_rule(TransformRule::RowFilter {
            expression: "status != 'deleted'".into(),
        });
        // Row doesn't have "status" – treated as != 'deleted' -> keep
        let rows = rows_from_json(vec![json!({"other": 1})]);
        let result = pipeline.apply(&rows);
        assert_eq!(result.len(), 1);
    }
}
