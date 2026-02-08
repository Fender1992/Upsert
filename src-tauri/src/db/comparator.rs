use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::schema::{ColumnInfo, ConstraintInfo, IndexInfo, TableInfo};

/// Result of comparing two schemas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDiffResult {
    pub source_database: String,
    pub target_database: String,
    pub changes: Vec<SchemaChange>,
    pub summary: DiffSummary,
}

/// Summary counts for a diff operation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiffSummary {
    pub additions: usize,
    pub removals: usize,
    pub modifications: usize,
    pub unchanged: usize,
}

/// A single schema change between source and target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaChange {
    pub object_type: SchemaObjectType,
    pub object_name: String,
    pub change_type: ChangeType,
    pub details: Vec<ChangeDetail>,
}

/// Types of schema objects that can differ
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SchemaObjectType {
    Table,
    Column,
    Index,
    Constraint,
    View,
    StoredProcedure,
    Trigger,
}

/// Types of changes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChangeType {
    Added,
    Removed,
    Modified,
    Unchanged,
}

/// Detail about what specifically changed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeDetail {
    pub property: String,
    pub source_value: Option<String>,
    pub target_value: Option<String>,
}

/// Compare two schemas and produce a diff result
pub fn compare_schemas(
    source_tables: &[TableInfo],
    target_tables: &[TableInfo],
    source_db: &str,
    target_db: &str,
) -> SchemaDiffResult {
    let mut changes = Vec::new();
    let mut summary = DiffSummary::default();

    // Build lookup maps by table name
    let source_map: HashMap<&str, &TableInfo> =
        source_tables.iter().map(|t| (t.table_name.as_str(), t)).collect();
    let target_map: HashMap<&str, &TableInfo> =
        target_tables.iter().map(|t| (t.table_name.as_str(), t)).collect();

    // Tables in source but not in target => Removed
    for table in source_tables {
        if !target_map.contains_key(table.table_name.as_str()) {
            changes.push(SchemaChange {
                object_type: SchemaObjectType::Table,
                object_name: table.table_name.clone(),
                change_type: ChangeType::Removed,
                details: vec![],
            });
            summary.removals += 1;
        }
    }

    // Tables in target but not in source => Added
    for table in target_tables {
        if !source_map.contains_key(table.table_name.as_str()) {
            changes.push(SchemaChange {
                object_type: SchemaObjectType::Table,
                object_name: table.table_name.clone(),
                change_type: ChangeType::Added,
                details: vec![],
            });
            summary.additions += 1;
        }
    }

    // Tables in both => compare internals
    for (name, source_table) in &source_map {
        if let Some(target_table) = target_map.get(name) {
            let table_changes = compare_tables(source_table, target_table);
            if table_changes.is_empty() {
                // Table is unchanged at the detail level
                changes.push(SchemaChange {
                    object_type: SchemaObjectType::Table,
                    object_name: name.to_string(),
                    change_type: ChangeType::Unchanged,
                    details: vec![],
                });
                summary.unchanged += 1;
            } else {
                // Merge sub-object changes (columns, indexes, constraints) into the top-level list
                for change in &table_changes {
                    match change.change_type {
                        ChangeType::Added => summary.additions += 1,
                        ChangeType::Removed => summary.removals += 1,
                        ChangeType::Modified => summary.modifications += 1,
                        ChangeType::Unchanged => summary.unchanged += 1,
                    }
                }
                changes.extend(table_changes);
            }
        }
    }

    SchemaDiffResult {
        source_database: source_db.to_string(),
        target_database: target_db.to_string(),
        changes,
        summary,
    }
}

/// Compare two matched tables and return changes for columns, indexes, and constraints
fn compare_tables(source: &TableInfo, target: &TableInfo) -> Vec<SchemaChange> {
    let table_name = &source.table_name;
    let mut changes = Vec::new();

    changes.extend(compare_columns(table_name, &source.columns, &target.columns));
    changes.extend(compare_indexes(table_name, &source.indexes, &target.indexes));
    changes.extend(compare_constraints(
        table_name,
        &source.constraints,
        &target.constraints,
    ));

    changes
}

/// Compare columns between source and target tables
fn compare_columns(
    table_name: &str,
    source_cols: &[ColumnInfo],
    target_cols: &[ColumnInfo],
) -> Vec<SchemaChange> {
    let mut changes = Vec::new();

    let source_map: HashMap<&str, &ColumnInfo> =
        source_cols.iter().map(|c| (c.name.as_str(), c)).collect();
    let target_map: HashMap<&str, &ColumnInfo> =
        target_cols.iter().map(|c| (c.name.as_str(), c)).collect();

    // Removed columns (in source, not in target)
    for col in source_cols {
        if !target_map.contains_key(col.name.as_str()) {
            changes.push(SchemaChange {
                object_type: SchemaObjectType::Column,
                object_name: format!("{}.{}", table_name, col.name),
                change_type: ChangeType::Removed,
                details: vec![],
            });
        }
    }

    // Added columns (in target, not in source)
    for col in target_cols {
        if !source_map.contains_key(col.name.as_str()) {
            changes.push(SchemaChange {
                object_type: SchemaObjectType::Column,
                object_name: format!("{}.{}", table_name, col.name),
                change_type: ChangeType::Added,
                details: vec![],
            });
        }
    }

    // Matched columns - compare properties
    for (name, src_col) in &source_map {
        if let Some(tgt_col) = target_map.get(name) {
            let details = diff_column_properties(src_col, tgt_col);
            if details.is_empty() {
                changes.push(SchemaChange {
                    object_type: SchemaObjectType::Column,
                    object_name: format!("{}.{}", table_name, name),
                    change_type: ChangeType::Unchanged,
                    details: vec![],
                });
            } else {
                changes.push(SchemaChange {
                    object_type: SchemaObjectType::Column,
                    object_name: format!("{}.{}", table_name, name),
                    change_type: ChangeType::Modified,
                    details,
                });
            }
        }
    }

    changes
}

/// Compare individual column properties and return change details
fn diff_column_properties(source: &ColumnInfo, target: &ColumnInfo) -> Vec<ChangeDetail> {
    let mut details = Vec::new();

    // Compare data types using canonical normalization
    let src_canonical = normalize_type(&source.data_type);
    let tgt_canonical = normalize_type(&target.data_type);
    if src_canonical != tgt_canonical {
        details.push(ChangeDetail {
            property: "data_type".to_string(),
            source_value: Some(source.data_type.clone()),
            target_value: Some(target.data_type.clone()),
        });
    }

    if source.is_nullable != target.is_nullable {
        details.push(ChangeDetail {
            property: "is_nullable".to_string(),
            source_value: Some(source.is_nullable.to_string()),
            target_value: Some(target.is_nullable.to_string()),
        });
    }

    if source.max_length != target.max_length {
        details.push(ChangeDetail {
            property: "max_length".to_string(),
            source_value: source.max_length.map(|v| v.to_string()),
            target_value: target.max_length.map(|v| v.to_string()),
        });
    }

    if source.precision != target.precision {
        details.push(ChangeDetail {
            property: "precision".to_string(),
            source_value: source.precision.map(|v| v.to_string()),
            target_value: target.precision.map(|v| v.to_string()),
        });
    }

    if source.scale != target.scale {
        details.push(ChangeDetail {
            property: "scale".to_string(),
            source_value: source.scale.map(|v| v.to_string()),
            target_value: target.scale.map(|v| v.to_string()),
        });
    }

    if source.default_value != target.default_value {
        details.push(ChangeDetail {
            property: "default_value".to_string(),
            source_value: source.default_value.clone(),
            target_value: target.default_value.clone(),
        });
    }

    details
}

/// Normalize a type string for comparison purposes.
/// Uses a lowercased, trimmed form so that "INT" == "int" == " int ".
/// This handles the simple case; for cross-engine comparison,
/// the caller should use type_mapper::to_canonical.
fn normalize_type(type_str: &str) -> String {
    type_str.trim().to_lowercase()
}

/// Compare indexes between source and target tables
fn compare_indexes(
    table_name: &str,
    source_idxs: &[IndexInfo],
    target_idxs: &[IndexInfo],
) -> Vec<SchemaChange> {
    let mut changes = Vec::new();

    let source_map: HashMap<&str, &IndexInfo> =
        source_idxs.iter().map(|i| (i.name.as_str(), i)).collect();
    let target_map: HashMap<&str, &IndexInfo> =
        target_idxs.iter().map(|i| (i.name.as_str(), i)).collect();

    // Removed indexes
    for idx in source_idxs {
        if !target_map.contains_key(idx.name.as_str()) {
            changes.push(SchemaChange {
                object_type: SchemaObjectType::Index,
                object_name: format!("{}.{}", table_name, idx.name),
                change_type: ChangeType::Removed,
                details: vec![],
            });
        }
    }

    // Added indexes
    for idx in target_idxs {
        if !source_map.contains_key(idx.name.as_str()) {
            changes.push(SchemaChange {
                object_type: SchemaObjectType::Index,
                object_name: format!("{}.{}", table_name, idx.name),
                change_type: ChangeType::Added,
                details: vec![],
            });
        }
    }

    // Matched indexes - compare properties
    for (name, src_idx) in &source_map {
        if let Some(tgt_idx) = target_map.get(name) {
            let details = diff_index_properties(src_idx, tgt_idx);
            if details.is_empty() {
                changes.push(SchemaChange {
                    object_type: SchemaObjectType::Index,
                    object_name: format!("{}.{}", table_name, name),
                    change_type: ChangeType::Unchanged,
                    details: vec![],
                });
            } else {
                changes.push(SchemaChange {
                    object_type: SchemaObjectType::Index,
                    object_name: format!("{}.{}", table_name, name),
                    change_type: ChangeType::Modified,
                    details,
                });
            }
        }
    }

    changes
}

/// Compare individual index properties
fn diff_index_properties(source: &IndexInfo, target: &IndexInfo) -> Vec<ChangeDetail> {
    let mut details = Vec::new();

    if source.columns != target.columns {
        details.push(ChangeDetail {
            property: "columns".to_string(),
            source_value: Some(source.columns.join(", ")),
            target_value: Some(target.columns.join(", ")),
        });
    }

    if source.is_unique != target.is_unique {
        details.push(ChangeDetail {
            property: "is_unique".to_string(),
            source_value: Some(source.is_unique.to_string()),
            target_value: Some(target.is_unique.to_string()),
        });
    }

    if source.is_clustered != target.is_clustered {
        details.push(ChangeDetail {
            property: "is_clustered".to_string(),
            source_value: Some(source.is_clustered.to_string()),
            target_value: Some(target.is_clustered.to_string()),
        });
    }

    details
}

/// Compare constraints between source and target tables
fn compare_constraints(
    table_name: &str,
    source_cons: &[ConstraintInfo],
    target_cons: &[ConstraintInfo],
) -> Vec<SchemaChange> {
    let mut changes = Vec::new();

    let source_map: HashMap<&str, &ConstraintInfo> =
        source_cons.iter().map(|c| (c.name.as_str(), c)).collect();
    let target_map: HashMap<&str, &ConstraintInfo> =
        target_cons.iter().map(|c| (c.name.as_str(), c)).collect();

    // Removed constraints
    for con in source_cons {
        if !target_map.contains_key(con.name.as_str()) {
            changes.push(SchemaChange {
                object_type: SchemaObjectType::Constraint,
                object_name: format!("{}.{}", table_name, con.name),
                change_type: ChangeType::Removed,
                details: vec![],
            });
        }
    }

    // Added constraints
    for con in target_cons {
        if !source_map.contains_key(con.name.as_str()) {
            changes.push(SchemaChange {
                object_type: SchemaObjectType::Constraint,
                object_name: format!("{}.{}", table_name, con.name),
                change_type: ChangeType::Added,
                details: vec![],
            });
        }
    }

    // Matched constraints - compare properties
    for (name, src_con) in &source_map {
        if let Some(tgt_con) = target_map.get(name) {
            let details = diff_constraint_properties(src_con, tgt_con);
            if details.is_empty() {
                changes.push(SchemaChange {
                    object_type: SchemaObjectType::Constraint,
                    object_name: format!("{}.{}", table_name, name),
                    change_type: ChangeType::Unchanged,
                    details: vec![],
                });
            } else {
                changes.push(SchemaChange {
                    object_type: SchemaObjectType::Constraint,
                    object_name: format!("{}.{}", table_name, name),
                    change_type: ChangeType::Modified,
                    details,
                });
            }
        }
    }

    changes
}

/// Compare individual constraint properties
fn diff_constraint_properties(
    source: &ConstraintInfo,
    target: &ConstraintInfo,
) -> Vec<ChangeDetail> {
    let mut details = Vec::new();

    if source.constraint_type != target.constraint_type {
        details.push(ChangeDetail {
            property: "constraint_type".to_string(),
            source_value: Some(format!("{:?}", source.constraint_type)),
            target_value: Some(format!("{:?}", target.constraint_type)),
        });
    }

    if source.columns != target.columns {
        details.push(ChangeDetail {
            property: "columns".to_string(),
            source_value: Some(source.columns.join(", ")),
            target_value: Some(target.columns.join(", ")),
        });
    }

    if source.referenced_table != target.referenced_table {
        details.push(ChangeDetail {
            property: "referenced_table".to_string(),
            source_value: source.referenced_table.clone(),
            target_value: target.referenced_table.clone(),
        });
    }

    if source.referenced_columns != target.referenced_columns {
        details.push(ChangeDetail {
            property: "referenced_columns".to_string(),
            source_value: source.referenced_columns.as_ref().map(|v| v.join(", ")),
            target_value: target.referenced_columns.as_ref().map(|v| v.join(", ")),
        });
    }

    details
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::ConstraintType;

    /// Helper to build a minimal ColumnInfo
    fn col(name: &str, data_type: &str, nullable: bool, pk: bool) -> ColumnInfo {
        ColumnInfo {
            name: name.to_string(),
            data_type: data_type.to_string(),
            is_nullable: nullable,
            is_primary_key: pk,
            max_length: None,
            precision: None,
            scale: None,
            default_value: None,
            ordinal_position: 0,
        }
    }

    /// Helper to build a minimal IndexInfo
    fn idx(name: &str, columns: &[&str], unique: bool) -> IndexInfo {
        IndexInfo {
            name: name.to_string(),
            columns: columns.iter().map(|s| s.to_string()).collect(),
            is_unique: unique,
            is_clustered: false,
            index_type: "BTREE".to_string(),
        }
    }

    /// Helper to build a minimal ConstraintInfo
    fn constraint(name: &str, ctype: ConstraintType, columns: &[&str]) -> ConstraintInfo {
        ConstraintInfo {
            name: name.to_string(),
            constraint_type: ctype,
            columns: columns.iter().map(|s| s.to_string()).collect(),
            referenced_table: None,
            referenced_columns: None,
        }
    }

    /// Helper to build a TableInfo
    fn table(
        name: &str,
        columns: Vec<ColumnInfo>,
        indexes: Vec<IndexInfo>,
        constraints: Vec<ConstraintInfo>,
    ) -> TableInfo {
        TableInfo {
            schema_name: "dbo".to_string(),
            table_name: name.to_string(),
            columns,
            indexes,
            constraints,
            row_count: None,
        }
    }

    #[test]
    fn test_identical_schemas() {
        let source = vec![table(
            "users",
            vec![col("id", "int", false, true), col("name", "varchar", true, false)],
            vec![idx("pk_users", &["id"], true)],
            vec![constraint("pk_users", ConstraintType::PrimaryKey, &["id"])],
        )];
        let target = source.clone();

        let result = compare_schemas(&source, &target, "src_db", "tgt_db");
        assert_eq!(result.summary.additions, 0);
        assert_eq!(result.summary.removals, 0);
        assert_eq!(result.summary.modifications, 0);
        // 1 table unchanged
        assert!(result.summary.unchanged > 0);
    }

    #[test]
    fn test_added_table() {
        let source = vec![table("users", vec![col("id", "int", false, true)], vec![], vec![])];
        let target = vec![
            table("users", vec![col("id", "int", false, true)], vec![], vec![]),
            table("orders", vec![col("id", "int", false, true)], vec![], vec![]),
        ];

        let result = compare_schemas(&source, &target, "src_db", "tgt_db");
        assert_eq!(result.summary.additions, 1);
        let added: Vec<_> = result
            .changes
            .iter()
            .filter(|c| c.change_type == ChangeType::Added && c.object_type == SchemaObjectType::Table)
            .collect();
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].object_name, "orders");
    }

    #[test]
    fn test_removed_table() {
        let source = vec![
            table("users", vec![col("id", "int", false, true)], vec![], vec![]),
            table("orders", vec![col("id", "int", false, true)], vec![], vec![]),
        ];
        let target = vec![table("users", vec![col("id", "int", false, true)], vec![], vec![])];

        let result = compare_schemas(&source, &target, "src_db", "tgt_db");
        assert_eq!(result.summary.removals, 1);
        let removed: Vec<_> = result
            .changes
            .iter()
            .filter(|c| c.change_type == ChangeType::Removed && c.object_type == SchemaObjectType::Table)
            .collect();
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].object_name, "orders");
    }

    #[test]
    fn test_added_column() {
        let source = vec![table(
            "users",
            vec![col("id", "int", false, true)],
            vec![],
            vec![],
        )];
        let target = vec![table(
            "users",
            vec![col("id", "int", false, true), col("email", "varchar", true, false)],
            vec![],
            vec![],
        )];

        let result = compare_schemas(&source, &target, "src_db", "tgt_db");
        let added_cols: Vec<_> = result
            .changes
            .iter()
            .filter(|c| c.change_type == ChangeType::Added && c.object_type == SchemaObjectType::Column)
            .collect();
        assert_eq!(added_cols.len(), 1);
        assert_eq!(added_cols[0].object_name, "users.email");
    }

    #[test]
    fn test_removed_column() {
        let source = vec![table(
            "users",
            vec![col("id", "int", false, true), col("email", "varchar", true, false)],
            vec![],
            vec![],
        )];
        let target = vec![table(
            "users",
            vec![col("id", "int", false, true)],
            vec![],
            vec![],
        )];

        let result = compare_schemas(&source, &target, "src_db", "tgt_db");
        let removed_cols: Vec<_> = result
            .changes
            .iter()
            .filter(|c| c.change_type == ChangeType::Removed && c.object_type == SchemaObjectType::Column)
            .collect();
        assert_eq!(removed_cols.len(), 1);
        assert_eq!(removed_cols[0].object_name, "users.email");
    }

    #[test]
    fn test_modified_column_type() {
        let source = vec![table(
            "users",
            vec![col("id", "int", false, true), col("name", "varchar", true, false)],
            vec![],
            vec![],
        )];
        let target = vec![table(
            "users",
            vec![col("id", "int", false, true), col("name", "text", true, false)],
            vec![],
            vec![],
        )];

        let result = compare_schemas(&source, &target, "src_db", "tgt_db");
        let modified: Vec<_> = result
            .changes
            .iter()
            .filter(|c| c.change_type == ChangeType::Modified && c.object_type == SchemaObjectType::Column)
            .collect();
        assert_eq!(modified.len(), 1);
        assert_eq!(modified[0].object_name, "users.name");
        assert!(modified[0].details.iter().any(|d| d.property == "data_type"));
    }

    #[test]
    fn test_modified_column_nullable() {
        let source = vec![table(
            "users",
            vec![col("name", "varchar", true, false)],
            vec![],
            vec![],
        )];
        let target = vec![table(
            "users",
            vec![col("name", "varchar", false, false)],
            vec![],
            vec![],
        )];

        let result = compare_schemas(&source, &target, "src_db", "tgt_db");
        let modified: Vec<_> = result
            .changes
            .iter()
            .filter(|c| c.change_type == ChangeType::Modified)
            .collect();
        assert_eq!(modified.len(), 1);
        assert!(modified[0].details.iter().any(|d| d.property == "is_nullable"));
    }

    #[test]
    fn test_type_case_insensitive() {
        let source = vec![table(
            "users",
            vec![col("id", "INT", false, true)],
            vec![],
            vec![],
        )];
        let target = vec![table(
            "users",
            vec![col("id", "int", false, true)],
            vec![],
            vec![],
        )];

        let result = compare_schemas(&source, &target, "src_db", "tgt_db");
        // Should be considered unchanged since INT == int after normalization
        let modified: Vec<_> = result
            .changes
            .iter()
            .filter(|c| c.change_type == ChangeType::Modified)
            .collect();
        assert_eq!(modified.len(), 0);
    }

    #[test]
    fn test_added_index() {
        let source = vec![table(
            "users",
            vec![col("id", "int", false, true)],
            vec![],
            vec![],
        )];
        let target = vec![table(
            "users",
            vec![col("id", "int", false, true)],
            vec![idx("idx_users_id", &["id"], true)],
            vec![],
        )];

        let result = compare_schemas(&source, &target, "src_db", "tgt_db");
        let added: Vec<_> = result
            .changes
            .iter()
            .filter(|c| c.change_type == ChangeType::Added && c.object_type == SchemaObjectType::Index)
            .collect();
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].object_name, "users.idx_users_id");
    }

    #[test]
    fn test_removed_index() {
        let source = vec![table(
            "users",
            vec![col("id", "int", false, true)],
            vec![idx("idx_users_id", &["id"], true)],
            vec![],
        )];
        let target = vec![table(
            "users",
            vec![col("id", "int", false, true)],
            vec![],
            vec![],
        )];

        let result = compare_schemas(&source, &target, "src_db", "tgt_db");
        let removed: Vec<_> = result
            .changes
            .iter()
            .filter(|c| c.change_type == ChangeType::Removed && c.object_type == SchemaObjectType::Index)
            .collect();
        assert_eq!(removed.len(), 1);
    }

    #[test]
    fn test_modified_index_columns() {
        let source = vec![table(
            "users",
            vec![col("id", "int", false, true)],
            vec![idx("idx_name", &["name"], false)],
            vec![],
        )];
        let target = vec![table(
            "users",
            vec![col("id", "int", false, true)],
            vec![idx("idx_name", &["name", "email"], false)],
            vec![],
        )];

        let result = compare_schemas(&source, &target, "src_db", "tgt_db");
        let modified: Vec<_> = result
            .changes
            .iter()
            .filter(|c| c.change_type == ChangeType::Modified && c.object_type == SchemaObjectType::Index)
            .collect();
        assert_eq!(modified.len(), 1);
        assert!(modified[0].details.iter().any(|d| d.property == "columns"));
    }

    #[test]
    fn test_modified_index_uniqueness() {
        let source = vec![table(
            "users",
            vec![col("id", "int", false, true)],
            vec![idx("idx_email", &["email"], false)],
            vec![],
        )];
        let target = vec![table(
            "users",
            vec![col("id", "int", false, true)],
            vec![idx("idx_email", &["email"], true)],
            vec![],
        )];

        let result = compare_schemas(&source, &target, "src_db", "tgt_db");
        let modified: Vec<_> = result
            .changes
            .iter()
            .filter(|c| c.change_type == ChangeType::Modified && c.object_type == SchemaObjectType::Index)
            .collect();
        assert_eq!(modified.len(), 1);
        assert!(modified[0].details.iter().any(|d| d.property == "is_unique"));
    }

    #[test]
    fn test_added_constraint() {
        let source = vec![table("users", vec![col("id", "int", false, true)], vec![], vec![])];
        let target = vec![table(
            "users",
            vec![col("id", "int", false, true)],
            vec![],
            vec![constraint("pk_users", ConstraintType::PrimaryKey, &["id"])],
        )];

        let result = compare_schemas(&source, &target, "src_db", "tgt_db");
        let added: Vec<_> = result
            .changes
            .iter()
            .filter(|c| c.change_type == ChangeType::Added && c.object_type == SchemaObjectType::Constraint)
            .collect();
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].object_name, "users.pk_users");
    }

    #[test]
    fn test_modified_constraint_type() {
        let source = vec![table(
            "users",
            vec![col("id", "int", false, true)],
            vec![],
            vec![constraint("uc_email", ConstraintType::Unique, &["email"])],
        )];
        let target = vec![table(
            "users",
            vec![col("id", "int", false, true)],
            vec![],
            vec![constraint("uc_email", ConstraintType::Check, &["email"])],
        )];

        let result = compare_schemas(&source, &target, "src_db", "tgt_db");
        let modified: Vec<_> = result
            .changes
            .iter()
            .filter(|c| c.change_type == ChangeType::Modified && c.object_type == SchemaObjectType::Constraint)
            .collect();
        assert_eq!(modified.len(), 1);
        assert!(modified[0]
            .details
            .iter()
            .any(|d| d.property == "constraint_type"));
    }

    #[test]
    fn test_modified_constraint_columns() {
        let source = vec![table(
            "users",
            vec![],
            vec![],
            vec![constraint("fk_order", ConstraintType::ForeignKey, &["user_id"])],
        )];
        let target = vec![table(
            "users",
            vec![],
            vec![],
            vec![constraint(
                "fk_order",
                ConstraintType::ForeignKey,
                &["user_id", "tenant_id"],
            )],
        )];

        let result = compare_schemas(&source, &target, "src_db", "tgt_db");
        let modified: Vec<_> = result
            .changes
            .iter()
            .filter(|c| c.change_type == ChangeType::Modified)
            .collect();
        assert_eq!(modified.len(), 1);
        assert!(modified[0].details.iter().any(|d| d.property == "columns"));
    }

    #[test]
    fn test_empty_schemas() {
        let result = compare_schemas(&[], &[], "src_db", "tgt_db");
        assert_eq!(result.summary.additions, 0);
        assert_eq!(result.summary.removals, 0);
        assert_eq!(result.summary.modifications, 0);
        assert_eq!(result.summary.unchanged, 0);
        assert!(result.changes.is_empty());
    }

    #[test]
    fn test_complex_multi_change_scenario() {
        let source = vec![
            table(
                "users",
                vec![
                    col("id", "int", false, true),
                    col("name", "varchar", true, false),
                    col("old_field", "text", true, false),
                ],
                vec![idx("idx_name", &["name"], false)],
                vec![constraint("pk_users", ConstraintType::PrimaryKey, &["id"])],
            ),
            table(
                "legacy",
                vec![col("id", "int", false, true)],
                vec![],
                vec![],
            ),
        ];
        let target = vec![
            table(
                "users",
                vec![
                    col("id", "bigint", false, true),  // modified type
                    col("name", "varchar", true, false), // unchanged
                    col("email", "varchar", true, false), // added
                    // old_field removed
                ],
                vec![
                    idx("idx_name", &["name"], true), // modified uniqueness
                    idx("idx_email", &["email"], false), // added
                ],
                vec![constraint("pk_users", ConstraintType::PrimaryKey, &["id"])],
            ),
            table(
                "products",
                vec![col("id", "int", false, true)],
                vec![],
                vec![],
            ),
        ];

        let result = compare_schemas(&source, &target, "src_db", "tgt_db");

        // Should have: 1 removed table (legacy), 1 added table (products),
        //   1 removed column (old_field), 1 added column (email), 1 modified column (id type),
        //   1 modified index (idx_name uniqueness), 1 added index (idx_email)
        assert!(result.summary.additions >= 3); // products table + email col + idx_email
        assert!(result.summary.removals >= 2); // legacy table + old_field col
        assert!(result.summary.modifications >= 2); // id type + idx_name uniqueness
    }

    #[test]
    fn test_diff_summary_counts() {
        let source = vec![table(
            "t1",
            vec![col("a", "int", false, false), col("b", "varchar", true, false)],
            vec![],
            vec![],
        )];
        let target = vec![table(
            "t1",
            vec![col("a", "bigint", false, false), col("c", "text", true, false)],
            vec![],
            vec![],
        )];

        let result = compare_schemas(&source, &target, "src", "tgt");
        // col a: modified (int->bigint), col b: removed, col c: added
        assert_eq!(result.summary.additions, 1);
        assert_eq!(result.summary.removals, 1);
        assert_eq!(result.summary.modifications, 1);
    }

    #[test]
    fn test_constraint_referenced_table_change() {
        let mut src_con = constraint("fk_order", ConstraintType::ForeignKey, &["user_id"]);
        src_con.referenced_table = Some("users".to_string());
        src_con.referenced_columns = Some(vec!["id".to_string()]);

        let mut tgt_con = constraint("fk_order", ConstraintType::ForeignKey, &["user_id"]);
        tgt_con.referenced_table = Some("accounts".to_string());
        tgt_con.referenced_columns = Some(vec!["id".to_string()]);

        let source = vec![table("orders", vec![], vec![], vec![src_con])];
        let target = vec![table("orders", vec![], vec![], vec![tgt_con])];

        let result = compare_schemas(&source, &target, "src", "tgt");
        let modified: Vec<_> = result
            .changes
            .iter()
            .filter(|c| c.change_type == ChangeType::Modified)
            .collect();
        assert_eq!(modified.len(), 1);
        assert!(modified[0]
            .details
            .iter()
            .any(|d| d.property == "referenced_table"));
    }

    #[test]
    fn test_column_default_value_change() {
        let mut src_col = col("status", "varchar", false, false);
        src_col.default_value = Some("'active'".to_string());

        let mut tgt_col = col("status", "varchar", false, false);
        tgt_col.default_value = Some("'inactive'".to_string());

        let source = vec![table("users", vec![src_col], vec![], vec![])];
        let target = vec![table("users", vec![tgt_col], vec![], vec![])];

        let result = compare_schemas(&source, &target, "src", "tgt");
        let modified: Vec<_> = result
            .changes
            .iter()
            .filter(|c| c.change_type == ChangeType::Modified)
            .collect();
        assert_eq!(modified.len(), 1);
        assert!(modified[0]
            .details
            .iter()
            .any(|d| d.property == "default_value"));
    }

    #[test]
    fn test_column_precision_change() {
        let mut src_col = col("amount", "decimal", false, false);
        src_col.precision = Some(18);
        src_col.scale = Some(2);

        let mut tgt_col = col("amount", "decimal", false, false);
        tgt_col.precision = Some(10);
        tgt_col.scale = Some(4);

        let source = vec![table("payments", vec![src_col], vec![], vec![])];
        let target = vec![table("payments", vec![tgt_col], vec![], vec![])];

        let result = compare_schemas(&source, &target, "src", "tgt");
        let modified: Vec<_> = result
            .changes
            .iter()
            .filter(|c| c.change_type == ChangeType::Modified)
            .collect();
        assert_eq!(modified.len(), 1);
        let props: Vec<_> = modified[0].details.iter().map(|d| d.property.as_str()).collect();
        assert!(props.contains(&"precision"));
        assert!(props.contains(&"scale"));
    }
}
