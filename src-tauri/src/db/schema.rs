use serde::{Deserialize, Serialize};

/// Represents the complete schema of a database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaInfo {
    pub database_name: String,
    pub tables: Vec<TableInfo>,
}

/// Represents a single database table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    pub schema_name: String,
    pub table_name: String,
    pub columns: Vec<ColumnInfo>,
    pub indexes: Vec<IndexInfo>,
    pub constraints: Vec<ConstraintInfo>,
    pub row_count: Option<i64>,
}

/// Represents a column in a table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
    pub max_length: Option<i32>,
    pub precision: Option<i32>,
    pub scale: Option<i32>,
    pub default_value: Option<String>,
    pub ordinal_position: i32,
}

/// Represents an index on a table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexInfo {
    pub name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
    pub is_clustered: bool,
    pub index_type: String,
}

/// Represents a constraint on a table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintInfo {
    pub name: String,
    pub constraint_type: ConstraintType,
    pub columns: Vec<String>,
    pub referenced_table: Option<String>,
    pub referenced_columns: Option<Vec<String>>,
}

/// Type of database constraint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConstraintType {
    PrimaryKey,
    ForeignKey,
    Unique,
    Check,
    Default,
}

/// Represents a database row as a map of column names to JSON values
pub type Row = std::collections::HashMap<String, serde_json::Value>;
