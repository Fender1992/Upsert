use std::fmt;

/// Errors that can occur during input validation.
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    TooLong { max: usize, actual: usize },
    NullBytes,
    ControlCharacters,
    SqlInjection(String),
    PathTraversal,
    InvalidIdentifier(String),
    EmptyInput,
    InvalidConnectionString(String),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::TooLong { max, actual } => {
                write!(f, "Input length {} exceeds maximum of {}", actual, max)
            }
            ValidationError::NullBytes => write!(f, "Input contains null bytes"),
            ValidationError::ControlCharacters => write!(f, "Input contains control characters"),
            ValidationError::SqlInjection(pattern) => {
                write!(f, "Potential SQL injection detected: {}", pattern)
            }
            ValidationError::PathTraversal => write!(f, "Path traversal pattern detected"),
            ValidationError::InvalidIdentifier(reason) => {
                write!(f, "Invalid SQL identifier: {}", reason)
            }
            ValidationError::EmptyInput => write!(f, "Input must not be empty"),
            ValidationError::InvalidConnectionString(reason) => {
                write!(f, "Invalid connection string: {}", reason)
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// SQL keywords that should not appear as standalone identifiers.
const DANGEROUS_SQL_KEYWORDS: &[&str] = &[
    "DROP", "DELETE", "INSERT", "UPDATE", "ALTER", "CREATE", "EXEC", "EXECUTE",
    "UNION", "SELECT", "TRUNCATE", "GRANT", "REVOKE",
];

/// Patterns that indicate SQL injection attempts.
const SQL_INJECTION_PATTERNS: &[&str] = &[
    ";--",
    "'; ",
    "' OR ",
    "' AND ",
    "1=1",
    "1 = 1",
    "UNION SELECT",
    "UNION ALL SELECT",
    "DROP TABLE",
    "DROP DATABASE",
    "DELETE FROM",
    "INSERT INTO",
    "UPDATE SET",
    "ALTER TABLE",
    "EXEC(",
    "EXECUTE(",
    "xp_cmdshell",
    "sp_executesql",
    "WAITFOR DELAY",
    "BENCHMARK(",
    "SLEEP(",
    "/*",
    "*/",
];

/// Validate general input against common attack patterns.
pub fn validate_input(input: &str, max_length: usize) -> Result<(), ValidationError> {
    if input.len() > max_length {
        return Err(ValidationError::TooLong {
            max: max_length,
            actual: input.len(),
        });
    }

    if input.contains('\0') {
        return Err(ValidationError::NullBytes);
    }

    if input.chars().any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t') {
        return Err(ValidationError::ControlCharacters);
    }

    // Check for path traversal
    if input.contains("../") || input.contains("..\\") {
        return Err(ValidationError::PathTraversal);
    }

    let upper = input.to_uppercase();
    for pattern in SQL_INJECTION_PATTERNS {
        if upper.contains(pattern) {
            return Err(ValidationError::SqlInjection(pattern.to_string()));
        }
    }

    Ok(())
}

/// Validate that a string is a safe SQL identifier (table name, column name, etc.).
///
/// Valid identifiers:
/// - Start with a letter or underscore
/// - Contain only letters, digits, underscores
/// - Are not SQL keywords
/// - Are between 1 and 128 characters
pub fn validate_table_name(name: &str) -> Result<(), ValidationError> {
    if name.is_empty() {
        return Err(ValidationError::EmptyInput);
    }

    if name.len() > 128 {
        return Err(ValidationError::TooLong {
            max: 128,
            actual: name.len(),
        });
    }

    if name.contains('\0') {
        return Err(ValidationError::NullBytes);
    }

    let first = name.chars().next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return Err(ValidationError::InvalidIdentifier(
            "must start with a letter or underscore".to_string(),
        ));
    }

    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(ValidationError::InvalidIdentifier(
            "must contain only letters, digits, and underscores".to_string(),
        ));
    }

    let upper = name.to_uppercase();
    for keyword in DANGEROUS_SQL_KEYWORDS {
        if upper == *keyword {
            return Err(ValidationError::InvalidIdentifier(format!(
                "'{}' is a reserved SQL keyword",
                name
            )));
        }
    }

    Ok(())
}

/// Validate a connection string for suspicious patterns.
pub fn validate_connection_string(conn_str: &str) -> Result<(), ValidationError> {
    if conn_str.is_empty() {
        return Err(ValidationError::EmptyInput);
    }

    if conn_str.contains('\0') {
        return Err(ValidationError::NullBytes);
    }

    // Check for path traversal in file-based connection strings
    if conn_str.contains("../") || conn_str.contains("..\\") {
        return Err(ValidationError::InvalidConnectionString(
            "path traversal detected".to_string(),
        ));
    }

    let upper = conn_str.to_uppercase();

    // Connection strings should not contain SQL statements
    let dangerous_patterns = [
        "DROP TABLE",
        "DROP DATABASE",
        "DELETE FROM",
        ";--",
        "EXEC(",
        "EXECUTE(",
        "xp_cmdshell",
        "sp_executesql",
    ];

    for pattern in &dangerous_patterns {
        if upper.contains(&pattern.to_uppercase()) {
            return Err(ValidationError::InvalidConnectionString(format!(
                "suspicious pattern: {}",
                pattern
            )));
        }
    }

    // Check for shell injection via backticks or $()
    if conn_str.contains('`') || conn_str.contains("$(") {
        return Err(ValidationError::InvalidConnectionString(
            "shell injection pattern detected".to_string(),
        ));
    }

    Ok(())
}

/// Strip control characters from a string for safe display.
/// Preserves newlines, carriage returns, and tabs.
pub fn sanitize_for_display(input: &str) -> String {
    input
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- validate_input tests ---

    #[test]
    fn test_valid_input() {
        assert!(validate_input("hello world", 100).is_ok());
        assert!(validate_input("SELECT * FROM users", 100).is_ok());
        assert!(validate_input("café", 100).is_ok());
    }

    #[test]
    fn test_input_too_long() {
        let result = validate_input("hello", 3);
        assert_eq!(
            result,
            Err(ValidationError::TooLong { max: 3, actual: 5 })
        );
    }

    #[test]
    fn test_input_exact_max_length() {
        assert!(validate_input("hello", 5).is_ok());
    }

    #[test]
    fn test_input_null_bytes() {
        let result = validate_input("hello\0world", 100);
        assert_eq!(result, Err(ValidationError::NullBytes));
    }

    #[test]
    fn test_input_control_characters() {
        // Bell character
        let result = validate_input("hello\x07world", 100);
        assert_eq!(result, Err(ValidationError::ControlCharacters));
    }

    #[test]
    fn test_input_allows_whitespace() {
        // Tabs and newlines should be allowed
        assert!(validate_input("hello\tworld", 100).is_ok());
        assert!(validate_input("hello\nworld", 100).is_ok());
        assert!(validate_input("hello\r\nworld", 100).is_ok());
    }

    #[test]
    fn test_input_path_traversal() {
        let result = validate_input("../../etc/passwd", 100);
        assert_eq!(result, Err(ValidationError::PathTraversal));

        let result = validate_input("..\\..\\windows\\system32", 100);
        assert_eq!(result, Err(ValidationError::PathTraversal));
    }

    #[test]
    fn test_input_sql_injection_semicolon_comment() {
        let result = validate_input("'; DROP TABLE users;--", 100);
        assert!(result.is_err());
        match result {
            Err(ValidationError::SqlInjection(_)) => {}
            other => panic!("Expected SqlInjection, got {:?}", other),
        }
    }

    #[test]
    fn test_input_sql_injection_union_select() {
        let result = validate_input("1 UNION SELECT * FROM passwords", 100);
        assert!(matches!(result, Err(ValidationError::SqlInjection(_))));
    }

    #[test]
    fn test_input_sql_injection_drop_table() {
        let result = validate_input("x'; DROP TABLE users; --", 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_input_sql_injection_or_1_equals_1() {
        let result = validate_input("' OR 1=1", 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_input_sql_injection_waitfor() {
        let result = validate_input("'; WAITFOR DELAY '00:00:10'", 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_input_sql_injection_sleep() {
        let result = validate_input("'; SLEEP(10)", 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_input_empty_string() {
        assert!(validate_input("", 100).is_ok());
    }

    #[test]
    fn test_input_unicode() {
        assert!(validate_input("日本語テスト", 100).is_ok());
        assert!(validate_input("Ünïcödé", 100).is_ok());
        assert!(validate_input("emoji: 🎉🚀", 100).is_ok());
    }

    #[test]
    fn test_input_max_length_boundary() {
        let s = "a".repeat(1000);
        assert!(validate_input(&s, 1000).is_ok());
        assert!(validate_input(&s, 999).is_err());
    }

    // --- validate_table_name tests ---

    #[test]
    fn test_valid_table_names() {
        assert!(validate_table_name("users").is_ok());
        assert!(validate_table_name("my_table").is_ok());
        assert!(validate_table_name("_private").is_ok());
        assert!(validate_table_name("Table1").is_ok());
        assert!(validate_table_name("a").is_ok());
    }

    #[test]
    fn test_table_name_empty() {
        assert_eq!(validate_table_name(""), Err(ValidationError::EmptyInput));
    }

    #[test]
    fn test_table_name_starts_with_number() {
        assert!(matches!(
            validate_table_name("1table"),
            Err(ValidationError::InvalidIdentifier(_))
        ));
    }

    #[test]
    fn test_table_name_with_spaces() {
        assert!(matches!(
            validate_table_name("my table"),
            Err(ValidationError::InvalidIdentifier(_))
        ));
    }

    #[test]
    fn test_table_name_with_special_characters() {
        assert!(validate_table_name("my-table").is_err());
        assert!(validate_table_name("my.table").is_err());
        assert!(validate_table_name("my;table").is_err());
        assert!(validate_table_name("my'table").is_err());
    }

    #[test]
    fn test_table_name_sql_keywords() {
        assert!(matches!(
            validate_table_name("DROP"),
            Err(ValidationError::InvalidIdentifier(_))
        ));
        assert!(matches!(
            validate_table_name("select"),
            Err(ValidationError::InvalidIdentifier(_))
        ));
        assert!(matches!(
            validate_table_name("DELETE"),
            Err(ValidationError::InvalidIdentifier(_))
        ));
    }

    #[test]
    fn test_table_name_too_long() {
        let name = "a".repeat(129);
        assert!(matches!(
            validate_table_name(&name),
            Err(ValidationError::TooLong { .. })
        ));
    }

    #[test]
    fn test_table_name_with_null_byte() {
        assert_eq!(
            validate_table_name("users\0"),
            Err(ValidationError::NullBytes)
        );
    }

    #[test]
    fn test_table_name_with_sql_injection() {
        assert!(validate_table_name("users;DROP TABLE admin").is_err());
    }

    // --- validate_connection_string tests ---

    #[test]
    fn test_valid_connection_strings() {
        assert!(validate_connection_string(
            "Server=localhost;Database=mydb;User Id=sa;Password=pass123;"
        )
        .is_ok());
        assert!(
            validate_connection_string("host=localhost port=5432 dbname=mydb user=postgres")
                .is_ok()
        );
        assert!(validate_connection_string("sqlite:///path/to/db.sqlite").is_ok());
        assert!(validate_connection_string("mongodb://localhost:27017/mydb").is_ok());
    }

    #[test]
    fn test_connection_string_empty() {
        assert_eq!(
            validate_connection_string(""),
            Err(ValidationError::EmptyInput)
        );
    }

    #[test]
    fn test_connection_string_null_bytes() {
        assert_eq!(
            validate_connection_string("Server=localhost\0"),
            Err(ValidationError::NullBytes)
        );
    }

    #[test]
    fn test_connection_string_path_traversal() {
        assert!(matches!(
            validate_connection_string("sqlite://../../etc/passwd"),
            Err(ValidationError::InvalidConnectionString(_))
        ));
    }

    #[test]
    fn test_connection_string_sql_injection() {
        assert!(validate_connection_string("Server=localhost;DROP TABLE users").is_err());
    }

    #[test]
    fn test_connection_string_shell_injection() {
        assert!(validate_connection_string("Server=`whoami`").is_err());
        assert!(validate_connection_string("Server=$(id)").is_err());
    }

    #[test]
    fn test_connection_string_xp_cmdshell() {
        assert!(
            validate_connection_string("Server=localhost; xp_cmdshell 'dir'").is_err()
        );
    }

    // --- sanitize_for_display tests ---

    #[test]
    fn test_sanitize_normal_text() {
        assert_eq!(sanitize_for_display("hello world"), "hello world");
    }

    #[test]
    fn test_sanitize_preserves_whitespace() {
        assert_eq!(
            sanitize_for_display("hello\tworld\nfoo\r\nbar"),
            "hello\tworld\nfoo\r\nbar"
        );
    }

    #[test]
    fn test_sanitize_strips_control_chars() {
        assert_eq!(sanitize_for_display("hello\x07world"), "helloworld");
        assert_eq!(sanitize_for_display("hello\x00world"), "helloworld");
        assert_eq!(sanitize_for_display("\x01\x02\x03test"), "test");
    }

    #[test]
    fn test_sanitize_unicode() {
        assert_eq!(sanitize_for_display("café 日本語"), "café 日本語");
    }

    #[test]
    fn test_sanitize_empty() {
        assert_eq!(sanitize_for_display(""), "");
    }
}
