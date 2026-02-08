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

    /// Apply all transformation rules to a set of rows
    pub fn apply(&self, _rows: &[Row]) -> Vec<Row> {
        todo!("Transform pipeline apply implementation")
    }
}

impl Default for TransformPipeline {
    fn default() -> Self {
        Self::new()
    }
}
