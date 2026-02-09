use super::connectors::DatabaseEngine;
use super::schema::{ColumnInfo, Row};

/// Engine-aware SQL statement generator.
pub struct SqlGenerator {
    engine: DatabaseEngine,
}

impl SqlGenerator {
    pub fn new(engine: DatabaseEngine) -> Self {
        Self { engine }
    }

    /// Quote an identifier for the target engine.
    fn quote_ident(&self, name: &str) -> String {
        match self.engine {
            DatabaseEngine::SqlServer => format!("[{}]", name),
            DatabaseEngine::MySql => format!("`{}`", name),
            _ => format!("\"{}\"", name),
        }
    }

    /// Convert a serde_json::Value to an SQL literal.
    fn value_to_sql(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::Null => "NULL".to_string(),
            serde_json::Value::Bool(b) => match self.engine {
                DatabaseEngine::SqlServer => {
                    if *b { "1".to_string() } else { "0".to_string() }
                }
                _ => {
                    if *b { "TRUE".to_string() } else { "FALSE".to_string() }
                }
            },
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => {
                let escaped = s.replace('\'', "''");
                format!("'{}'", escaped)
            }
            other => {
                let escaped = other.to_string().replace('\'', "''");
                format!("'{}'", escaped)
            }
        }
    }

    /// Generate an INSERT statement.
    pub fn generate_insert(&self, table: &str, row: &Row) -> String {
        let mut cols: Vec<&String> = row.keys().collect();
        cols.sort();
        let col_list = cols.iter().map(|c| self.quote_ident(c)).collect::<Vec<_>>().join(", ");
        let val_list = cols.iter().map(|c| self.value_to_sql(&row[*c])).collect::<Vec<_>>().join(", ");
        format!(
            "INSERT INTO {} ({}) VALUES ({});",
            self.quote_ident(table),
            col_list,
            val_list
        )
    }

    /// Generate an UPDATE statement using key_columns for the WHERE clause.
    pub fn generate_update(&self, table: &str, row: &Row, key_columns: &[String]) -> String {
        let mut set_cols: Vec<&String> = row.keys().filter(|k| !key_columns.contains(k)).collect();
        set_cols.sort();
        let set_clause = set_cols
            .iter()
            .map(|c| format!("{} = {}", self.quote_ident(c), self.value_to_sql(&row[*c])))
            .collect::<Vec<_>>()
            .join(", ");
        let where_clause = key_columns
            .iter()
            .map(|k| {
                let val = row.get(k).cloned().unwrap_or(serde_json::Value::Null);
                format!("{} = {}", self.quote_ident(k), self.value_to_sql(&val))
            })
            .collect::<Vec<_>>()
            .join(" AND ");
        format!(
            "UPDATE {} SET {} WHERE {};",
            self.quote_ident(table),
            set_clause,
            where_clause
        )
    }

    /// Generate an UPDATE statement that only SETs the specified columns.
    pub fn generate_partial_update(
        &self,
        table: &str,
        row: &Row,
        update_columns: &[String],
        key_columns: &[String],
    ) -> String {
        let mut set_cols: Vec<&String> = update_columns
            .iter()
            .filter(|c| !key_columns.contains(c) && row.contains_key(c.as_str()))
            .collect();
        set_cols.sort();
        if set_cols.is_empty() {
            // Nothing to update; return a no-op
            return String::new();
        }
        let set_clause = set_cols
            .iter()
            .map(|c| format!("{} = {}", self.quote_ident(c), self.value_to_sql(&row[*c])))
            .collect::<Vec<_>>()
            .join(", ");
        let where_clause = key_columns
            .iter()
            .map(|k| {
                let val = row.get(k).cloned().unwrap_or(serde_json::Value::Null);
                format!("{} = {}", self.quote_ident(k), self.value_to_sql(&val))
            })
            .collect::<Vec<_>>()
            .join(" AND ");
        format!(
            "UPDATE {} SET {} WHERE {};",
            self.quote_ident(table),
            set_clause,
            where_clause
        )
    }

    /// Prepare a row for INSERT by validating against the target table schema.
    ///
    /// Returns `(Some(row), warnings)` when the row can be inserted (possibly
    /// after truncating oversized strings), or `(None, warnings)` when the row
    /// must be skipped because a NOT NULL column without a default is missing
    /// from the source data.
    pub fn prepare_row_for_insert(
        &self,
        row: &Row,
        schema: &[ColumnInfo],
    ) -> (Option<Row>, Vec<String>) {
        let mut prepared = row.clone();
        let mut warnings = Vec::new();
        let mut must_skip = false;

        for col in schema {
            match prepared.get(&col.name) {
                Some(serde_json::Value::String(s)) => {
                    // Truncate strings exceeding the target column's max_length
                    if let Some(max_len) = col.max_length {
                        if max_len > 0 && (s.len() as i32) > max_len {
                            warnings.push(format!(
                                "Truncated '{}': {} -> {} chars",
                                col.name,
                                s.len(),
                                max_len
                            ));
                            let truncated: String =
                                s.chars().take(max_len as usize).collect();
                            prepared.insert(
                                col.name.clone(),
                                serde_json::Value::String(truncated),
                            );
                        }
                    }
                }
                Some(serde_json::Value::Null) => {
                    if !col.is_nullable && col.default_value.is_none() {
                        warnings.push(format!(
                            "Column '{}' is NOT NULL without default but value is NULL",
                            col.name
                        ));
                        must_skip = true;
                    }
                }
                None => {
                    // Column exists in target schema but is missing from source row.
                    // If it has a default or is nullable, the DB handles it.
                    // If NOT NULL without default (and not auto-generated PK), skip.
                    if !col.is_nullable && col.default_value.is_none() {
                        warnings.push(format!(
                            "Column '{}' is NOT NULL without default and missing from source",
                            col.name
                        ));
                        must_skip = true;
                    }
                }
                _ => {}
            }
        }

        if must_skip {
            (None, warnings)
        } else {
            (Some(prepared), warnings)
        }
    }

    /// Generate a DELETE statement using key_columns for the WHERE clause.
    pub fn generate_delete(&self, table: &str, row: &Row, key_columns: &[String]) -> String {
        let where_clause = key_columns
            .iter()
            .map(|k| {
                let val = row.get(k).cloned().unwrap_or(serde_json::Value::Null);
                format!("{} = {}", self.quote_ident(k), self.value_to_sql(&val))
            })
            .collect::<Vec<_>>()
            .join(" AND ");
        format!(
            "DELETE FROM {} WHERE {};",
            self.quote_ident(table),
            where_clause
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::ColumnInfo;
    use serde_json::json;

    fn row(pairs: &[(&str, serde_json::Value)]) -> Row {
        pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
    }

    #[test]
    fn test_insert_postgres() {
        let gen = SqlGenerator::new(DatabaseEngine::PostgreSql);
        let r = row(&[("id", json!(1)), ("name", json!("Alice"))]);
        let sql = gen.generate_insert("users", &r);
        assert!(sql.contains("INSERT INTO \"users\""));
        assert!(sql.contains("\"id\""));
        assert!(sql.contains("'Alice'"));
    }

    #[test]
    fn test_insert_sqlserver() {
        let gen = SqlGenerator::new(DatabaseEngine::SqlServer);
        let r = row(&[("id", json!(1)), ("name", json!("Bob"))]);
        let sql = gen.generate_insert("users", &r);
        assert!(sql.contains("INSERT INTO [users]"));
        assert!(sql.contains("[id]"));
    }

    #[test]
    fn test_update() {
        let gen = SqlGenerator::new(DatabaseEngine::PostgreSql);
        let r = row(&[("id", json!(1)), ("name", json!("Alice")), ("email", json!("a@b.com"))]);
        let sql = gen.generate_update("users", &r, &["id".to_string()]);
        assert!(sql.contains("UPDATE \"users\" SET"));
        assert!(sql.contains("WHERE \"id\" = 1"));
        assert!(sql.contains("\"email\" = 'a@b.com'"));
    }

    #[test]
    fn test_delete() {
        let gen = SqlGenerator::new(DatabaseEngine::PostgreSql);
        let r = row(&[("id", json!(42))]);
        let sql = gen.generate_delete("users", &r, &["id".to_string()]);
        assert_eq!(sql, "DELETE FROM \"users\" WHERE \"id\" = 42;");
    }

    #[test]
    fn test_null_and_bool() {
        let gen = SqlGenerator::new(DatabaseEngine::SqlServer);
        let r = row(&[("id", json!(1)), ("active", json!(true)), ("notes", json!(null))]);
        let sql = gen.generate_insert("t", &r);
        assert!(sql.contains("1")); // bool true -> 1 for SQL Server
        assert!(sql.contains("NULL"));
    }

    #[test]
    fn test_string_escaping() {
        let gen = SqlGenerator::new(DatabaseEngine::PostgreSql);
        let r = row(&[("id", json!(1)), ("name", json!("O'Brien"))]);
        let sql = gen.generate_insert("users", &r);
        assert!(sql.contains("'O''Brien'"));
    }

    #[test]
    fn test_prepare_row_truncates_oversized_string() {
        let gen = SqlGenerator::new(DatabaseEngine::PostgreSql);
        let r = row(&[("id", json!(1)), ("name", json!("A very long name"))]);
        let schema = vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "integer".to_string(),
                is_nullable: false,
                is_primary_key: true,
                max_length: None,
                precision: None,
                scale: None,
                default_value: Some("nextval('users_id_seq')".to_string()),
                ordinal_position: 1,
            },
            ColumnInfo {
                name: "name".to_string(),
                data_type: "varchar".to_string(),
                is_nullable: false,
                is_primary_key: false,
                max_length: Some(10),
                precision: None,
                scale: None,
                default_value: None,
                ordinal_position: 2,
            },
        ];
        let (prepared, warnings) = gen.prepare_row_for_insert(&r, &schema);
        assert!(prepared.is_some());
        let prepared = prepared.unwrap();
        assert_eq!(prepared.get("name").unwrap(), &json!("A very lon"));
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Truncated"));
    }

    #[test]
    fn test_prepare_row_skips_missing_not_null_column() {
        let gen = SqlGenerator::new(DatabaseEngine::PostgreSql);
        let r = row(&[("id", json!(1)), ("name", json!("Alice"))]);
        let schema = vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "integer".to_string(),
                is_nullable: false,
                is_primary_key: true,
                max_length: None,
                precision: None,
                scale: None,
                default_value: Some("nextval('users_id_seq')".to_string()),
                ordinal_position: 1,
            },
            ColumnInfo {
                name: "name".to_string(),
                data_type: "varchar".to_string(),
                is_nullable: false,
                is_primary_key: false,
                max_length: Some(100),
                precision: None,
                scale: None,
                default_value: None,
                ordinal_position: 2,
            },
            ColumnInfo {
                name: "slug".to_string(),
                data_type: "varchar".to_string(),
                is_nullable: false,
                is_primary_key: false,
                max_length: Some(100),
                precision: None,
                scale: None,
                default_value: None,
                ordinal_position: 3,
            },
        ];
        let (prepared, warnings) = gen.prepare_row_for_insert(&r, &schema);
        assert!(prepared.is_none());
        assert!(warnings.iter().any(|w| w.contains("slug")));
    }

    #[test]
    fn test_prepare_row_allows_nullable_missing_column() {
        let gen = SqlGenerator::new(DatabaseEngine::PostgreSql);
        let r = row(&[("id", json!(1)), ("name", json!("Alice"))]);
        let schema = vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "integer".to_string(),
                is_nullable: false,
                is_primary_key: true,
                max_length: None,
                precision: None,
                scale: None,
                default_value: Some("nextval('users_id_seq')".to_string()),
                ordinal_position: 1,
            },
            ColumnInfo {
                name: "name".to_string(),
                data_type: "varchar".to_string(),
                is_nullable: false,
                is_primary_key: false,
                max_length: Some(100),
                precision: None,
                scale: None,
                default_value: None,
                ordinal_position: 2,
            },
            ColumnInfo {
                name: "bio".to_string(),
                data_type: "text".to_string(),
                is_nullable: true,
                is_primary_key: false,
                max_length: None,
                precision: None,
                scale: None,
                default_value: None,
                ordinal_position: 3,
            },
        ];
        let (prepared, warnings) = gen.prepare_row_for_insert(&r, &schema);
        assert!(prepared.is_some());
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_prepare_row_allows_column_with_default() {
        let gen = SqlGenerator::new(DatabaseEngine::PostgreSql);
        let r = row(&[("id", json!(1))]);
        let schema = vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "integer".to_string(),
                is_nullable: false,
                is_primary_key: true,
                max_length: None,
                precision: None,
                scale: None,
                default_value: Some("nextval('users_id_seq')".to_string()),
                ordinal_position: 1,
            },
            ColumnInfo {
                name: "created_at".to_string(),
                data_type: "timestamp".to_string(),
                is_nullable: false,
                is_primary_key: false,
                max_length: None,
                precision: None,
                scale: None,
                default_value: Some("now()".to_string()),
                ordinal_position: 2,
            },
        ];
        let (prepared, warnings) = gen.prepare_row_for_insert(&r, &schema);
        assert!(prepared.is_some());
        assert!(warnings.is_empty());
    }
}
