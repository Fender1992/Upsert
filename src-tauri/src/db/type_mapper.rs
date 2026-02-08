use super::connectors::DatabaseEngine;
use serde::{Deserialize, Serialize};

/// Canonical type system for cross-engine type mapping
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CanonicalType {
    Boolean,
    TinyInt,
    SmallInt,
    Int,
    BigInt,
    Float,
    Double,
    Decimal { precision: u8, scale: u8 },
    Char(u32),
    Varchar(u32),
    Text,
    NChar(u32),
    NVarchar(u32),
    NText,
    Binary(u32),
    Varbinary(u32),
    Blob,
    Date,
    Time,
    DateTime,
    Timestamp,
    Uuid,
    Json,
    Xml,
    Array(Box<CanonicalType>),
    Unknown(String),
}

/// A type mapping override configured by the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeOverride {
    pub source_engine: DatabaseEngine,
    pub source_type: String,
    pub target_engine: DatabaseEngine,
    pub target_type: String,
}

/// Result of a type mapping that includes warnings about potential issues
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TypeMappingResult {
    pub target_type: String,
    pub warnings: Vec<String>,
    pub is_lossy: bool,
}

/// Parse a native type string into base type and optional parameters.
///
/// Examples:
/// - "VARCHAR(255)" -> ("varchar", Some(255), None)
/// - "DECIMAL(18,2)" -> ("decimal", Some(18), Some(2))
/// - "INT" -> ("int", None, None)
pub fn parse_native_type(type_str: &str) -> (String, Option<u32>, Option<u32>) {
    let trimmed = type_str.trim().to_lowercase();

    if let Some(paren_start) = trimmed.find('(') {
        let base = trimmed[..paren_start].trim().to_string();
        let params_str = trimmed[paren_start + 1..].trim_end_matches(')').trim();

        if params_str.eq_ignore_ascii_case("max") {
            return (base, Some(u32::MAX), None);
        }

        let parts: Vec<&str> = params_str.split(',').collect();
        let p1 = parts.first().and_then(|s| s.trim().parse::<u32>().ok());
        let p2 = parts.get(1).and_then(|s| s.trim().parse::<u32>().ok());
        (base, p1, p2)
    } else {
        (trimmed, None, None)
    }
}

/// Map a native type string from one engine to the canonical type
pub fn to_canonical(engine: &DatabaseEngine, native_type: &str) -> CanonicalType {
    let lower = native_type.to_lowercase().trim().to_string();
    match engine {
        DatabaseEngine::SqlServer => sqlserver_to_canonical(&lower),
        DatabaseEngine::PostgreSql => postgres_to_canonical(&lower),
        DatabaseEngine::MySql => mysql_to_canonical(&lower),
        DatabaseEngine::Sqlite => sqlite_to_canonical(&lower),
        DatabaseEngine::Oracle => oracle_to_canonical(&lower),
        DatabaseEngine::MongoDb => mongodb_to_canonical(&lower),
        DatabaseEngine::CosmosDb => cosmosdb_to_canonical(&lower),
    }
}

/// Map a canonical type to the native type string for a specific engine
pub fn from_canonical(engine: &DatabaseEngine, canonical: &CanonicalType) -> String {
    match engine {
        DatabaseEngine::SqlServer => canonical_to_sqlserver(canonical),
        DatabaseEngine::PostgreSql => canonical_to_postgres(canonical),
        DatabaseEngine::MySql => canonical_to_mysql(canonical),
        DatabaseEngine::Sqlite => canonical_to_sqlite(canonical),
        DatabaseEngine::Oracle => canonical_to_oracle(canonical),
        DatabaseEngine::MongoDb => canonical_to_mongodb(canonical),
        DatabaseEngine::CosmosDb => canonical_to_cosmosdb(canonical),
    }
}

/// Map a native type from one engine directly to another engine's native type
pub fn map_type(
    source_engine: &DatabaseEngine,
    target_engine: &DatabaseEngine,
    native_type: &str,
) -> String {
    let canonical = to_canonical(source_engine, native_type);
    from_canonical(target_engine, &canonical)
}

/// Map a native type with warnings about potential precision loss or compatibility issues
pub fn map_type_with_warnings(
    source_engine: &DatabaseEngine,
    target_engine: &DatabaseEngine,
    native_type: &str,
) -> TypeMappingResult {
    let canonical = to_canonical(source_engine, native_type);
    let target_type = from_canonical(target_engine, &canonical);
    let mut warnings = Vec::new();
    let mut is_lossy = false;

    // Detect precision loss when mapping decimal types to float
    match &canonical {
        CanonicalType::Decimal { precision, scale } => {
            let target_canonical = to_canonical(target_engine, &target_type);
            match target_canonical {
                CanonicalType::Float | CanonicalType::Double => {
                    warnings.push(format!(
                        "DECIMAL({},{}) mapped to floating point; precision loss possible",
                        precision, scale
                    ));
                    is_lossy = true;
                }
                CanonicalType::Decimal {
                    precision: tp,
                    scale: ts,
                } => {
                    if tp < *precision || ts < *scale {
                        warnings.push(format!(
                            "Target DECIMAL({},{}) has less precision than source DECIMAL({},{})",
                            tp, ts, precision, scale
                        ));
                        is_lossy = true;
                    }
                }
                _ => {}
            }
        }
        CanonicalType::BigInt => {
            let target_canonical = to_canonical(target_engine, &target_type);
            if matches!(target_canonical, CanonicalType::Int) {
                warnings.push("BIGINT mapped to INT; overflow possible for large values".into());
                is_lossy = true;
            }
        }
        CanonicalType::DateTime | CanonicalType::Timestamp => {
            let target_canonical = to_canonical(target_engine, &target_type);
            if matches!(target_canonical, CanonicalType::Text) {
                warnings
                    .push("DateTime/Timestamp mapped to TEXT; type safety lost in target".into());
                is_lossy = true;
            }
        }
        CanonicalType::NVarchar(_) | CanonicalType::NChar(_) | CanonicalType::NText => {
            let target_canonical = to_canonical(target_engine, &target_type);
            match target_canonical {
                CanonicalType::Varchar(_) | CanonicalType::Char(_) | CanonicalType::Text => {
                    warnings.push(
                        "Unicode type mapped to non-Unicode type; character loss possible".into(),
                    );
                    is_lossy = true;
                }
                _ => {}
            }
        }
        CanonicalType::Json => {
            let target_canonical = to_canonical(target_engine, &target_type);
            if matches!(
                target_canonical,
                CanonicalType::Text | CanonicalType::NText
            ) {
                warnings.push("JSON mapped to text type; JSON validation lost in target".into());
            }
        }
        CanonicalType::Uuid => {
            let target_canonical = to_canonical(target_engine, &target_type);
            if matches!(
                target_canonical,
                CanonicalType::Text
                    | CanonicalType::Varchar(_)
                    | CanonicalType::Char(_)
            ) {
                warnings.push("UUID mapped to string type; UUID validation lost in target".into());
            }
        }
        CanonicalType::Unknown(s) => {
            warnings.push(format!("Unknown source type '{}'; mapped as-is", s));
        }
        _ => {}
    }

    TypeMappingResult {
        target_type,
        warnings,
        is_lossy,
    }
}

// ---------------------------------------------------------------------------
// SQL Server
// ---------------------------------------------------------------------------

fn sqlserver_to_canonical(native: &str) -> CanonicalType {
    let (base, p1, p2) = parse_native_type(native);
    match base.as_str() {
        "bit" => CanonicalType::Boolean,
        "tinyint" => CanonicalType::TinyInt,
        "smallint" => CanonicalType::SmallInt,
        "int" => CanonicalType::Int,
        "bigint" => CanonicalType::BigInt,
        "real" => CanonicalType::Float,
        "float" => CanonicalType::Double,
        "decimal" | "numeric" => CanonicalType::Decimal {
            precision: p1.unwrap_or(18) as u8,
            scale: p2.unwrap_or(0) as u8,
        },
        "money" => CanonicalType::Decimal {
            precision: 19,
            scale: 4,
        },
        "smallmoney" => CanonicalType::Decimal {
            precision: 10,
            scale: 4,
        },
        "char" => CanonicalType::Char(p1.unwrap_or(1)),
        "varchar" => {
            if p1 == Some(u32::MAX) {
                CanonicalType::Text
            } else {
                CanonicalType::Varchar(p1.unwrap_or(1))
            }
        }
        "text" => CanonicalType::Text,
        "nchar" => CanonicalType::NChar(p1.unwrap_or(1)),
        "nvarchar" => {
            if p1 == Some(u32::MAX) {
                CanonicalType::NText
            } else {
                CanonicalType::NVarchar(p1.unwrap_or(1))
            }
        }
        "ntext" => CanonicalType::NText,
        "binary" => CanonicalType::Binary(p1.unwrap_or(1)),
        "varbinary" => {
            if p1 == Some(u32::MAX) {
                CanonicalType::Blob
            } else {
                CanonicalType::Varbinary(p1.unwrap_or(1))
            }
        }
        "image" => CanonicalType::Blob,
        "date" => CanonicalType::Date,
        "time" => CanonicalType::Time,
        "datetime" | "datetime2" | "smalldatetime" => CanonicalType::DateTime,
        "datetimeoffset" => CanonicalType::Timestamp,
        "uniqueidentifier" => CanonicalType::Uuid,
        "xml" => CanonicalType::Xml,
        "timestamp" | "rowversion" => CanonicalType::Varbinary(8),
        "sql_variant" => CanonicalType::Unknown("sql_variant".into()),
        "geography" => CanonicalType::Unknown("geography".into()),
        "geometry" => CanonicalType::Unknown("geometry".into()),
        "hierarchyid" => CanonicalType::Unknown("hierarchyid".into()),
        _ => CanonicalType::Unknown(native.to_string()),
    }
}

fn canonical_to_sqlserver(canonical: &CanonicalType) -> String {
    match canonical {
        CanonicalType::Boolean => "BIT".into(),
        CanonicalType::TinyInt => "TINYINT".into(),
        CanonicalType::SmallInt => "SMALLINT".into(),
        CanonicalType::Int => "INT".into(),
        CanonicalType::BigInt => "BIGINT".into(),
        CanonicalType::Float => "REAL".into(),
        CanonicalType::Double => "FLOAT".into(),
        CanonicalType::Decimal { precision, scale } => format!("DECIMAL({},{})", precision, scale),
        CanonicalType::Char(n) => format!("CHAR({})", n),
        CanonicalType::Varchar(n) => format!("VARCHAR({})", n),
        CanonicalType::Text => "VARCHAR(MAX)".into(),
        CanonicalType::NChar(n) => format!("NCHAR({})", n),
        CanonicalType::NVarchar(n) => format!("NVARCHAR({})", n),
        CanonicalType::NText => "NVARCHAR(MAX)".into(),
        CanonicalType::Binary(n) => format!("BINARY({})", n),
        CanonicalType::Varbinary(n) => format!("VARBINARY({})", n),
        CanonicalType::Blob => "VARBINARY(MAX)".into(),
        CanonicalType::Date => "DATE".into(),
        CanonicalType::Time => "TIME".into(),
        CanonicalType::DateTime => "DATETIME2".into(),
        CanonicalType::Timestamp => "DATETIMEOFFSET".into(),
        CanonicalType::Uuid => "UNIQUEIDENTIFIER".into(),
        CanonicalType::Json => "NVARCHAR(MAX)".into(),
        CanonicalType::Xml => "XML".into(),
        CanonicalType::Array(_) => "NVARCHAR(MAX)".into(),
        CanonicalType::Unknown(s) => s.to_uppercase(),
    }
}

// ---------------------------------------------------------------------------
// PostgreSQL
// ---------------------------------------------------------------------------

fn postgres_to_canonical(native: &str) -> CanonicalType {
    let (base, p1, p2) = parse_native_type(native);
    match base.as_str() {
        "boolean" | "bool" => CanonicalType::Boolean,
        "smallint" | "int2" => CanonicalType::SmallInt,
        "smallserial" => CanonicalType::SmallInt,
        "integer" | "int4" | "int" => CanonicalType::Int,
        "serial" => CanonicalType::Int,
        "bigint" | "int8" => CanonicalType::BigInt,
        "bigserial" => CanonicalType::BigInt,
        "real" | "float4" => CanonicalType::Float,
        "double precision" | "float8" => CanonicalType::Double,
        "numeric" | "decimal" => CanonicalType::Decimal {
            precision: p1.unwrap_or(18) as u8,
            scale: p2.unwrap_or(0) as u8,
        },
        "money" => CanonicalType::Decimal {
            precision: 19,
            scale: 4,
        },
        "char" | "character" => CanonicalType::Char(p1.unwrap_or(1)),
        "varchar" | "character varying" => CanonicalType::Varchar(p1.unwrap_or(255)),
        "text" => CanonicalType::Text,
        "bytea" => CanonicalType::Blob,
        "date" => CanonicalType::Date,
        "time" | "time without time zone" => CanonicalType::Time,
        "timestamp" | "timestamp without time zone" => CanonicalType::DateTime,
        "timestamp with time zone" | "timestamptz" => CanonicalType::Timestamp,
        "interval" => CanonicalType::Text,
        "uuid" => CanonicalType::Uuid,
        "json" | "jsonb" => CanonicalType::Json,
        "xml" => CanonicalType::Xml,
        "cidr" | "inet" => CanonicalType::Varchar(43),
        "macaddr" | "macaddr8" => CanonicalType::Varchar(17),
        "tsvector" | "tsquery" => CanonicalType::Text,
        "point" | "line" | "lseg" | "box" | "path" | "polygon" | "circle" => {
            CanonicalType::Unknown(base)
        }
        "bit" => CanonicalType::Binary(p1.map(|v| (v + 7) / 8).unwrap_or(1)),
        "bit varying" | "varbit" => CanonicalType::Varbinary(p1.map(|v| (v + 7) / 8).unwrap_or(1)),
        _ => {
            // Handle array notation e.g. "integer[]"
            let trimmed = native.trim();
            if trimmed.ends_with("[]") {
                let inner = &trimmed[..trimmed.len() - 2];
                let inner_canonical = postgres_to_canonical(inner);
                CanonicalType::Array(Box::new(inner_canonical))
            } else {
                CanonicalType::Unknown(native.to_string())
            }
        }
    }
}

fn canonical_to_postgres(canonical: &CanonicalType) -> String {
    match canonical {
        CanonicalType::Boolean => "BOOLEAN".into(),
        CanonicalType::TinyInt | CanonicalType::SmallInt => "SMALLINT".into(),
        CanonicalType::Int => "INTEGER".into(),
        CanonicalType::BigInt => "BIGINT".into(),
        CanonicalType::Float => "REAL".into(),
        CanonicalType::Double => "DOUBLE PRECISION".into(),
        CanonicalType::Decimal { precision, scale } => format!("NUMERIC({},{})", precision, scale),
        CanonicalType::Char(n) => format!("CHAR({})", n),
        CanonicalType::Varchar(n) => format!("VARCHAR({})", n),
        CanonicalType::Text | CanonicalType::NText => "TEXT".into(),
        CanonicalType::NChar(n) => format!("CHAR({})", n),
        CanonicalType::NVarchar(n) => format!("VARCHAR({})", n),
        CanonicalType::Binary(_) | CanonicalType::Varbinary(_) => "BYTEA".into(),
        CanonicalType::Blob => "BYTEA".into(),
        CanonicalType::Date => "DATE".into(),
        CanonicalType::Time => "TIME".into(),
        CanonicalType::DateTime => "TIMESTAMP".into(),
        CanonicalType::Timestamp => "TIMESTAMPTZ".into(),
        CanonicalType::Uuid => "UUID".into(),
        CanonicalType::Json => "JSONB".into(),
        CanonicalType::Xml => "XML".into(),
        CanonicalType::Array(inner) => format!("{}[]", canonical_to_postgres(inner)),
        CanonicalType::Unknown(s) => s.to_uppercase(),
    }
}

// ---------------------------------------------------------------------------
// MySQL
// ---------------------------------------------------------------------------

fn mysql_to_canonical(native: &str) -> CanonicalType {
    let (base, p1, p2) = parse_native_type(native);
    match base.as_str() {
        "boolean" | "bool" => CanonicalType::Boolean,
        "tinyint" => {
            if p1 == Some(1) {
                CanonicalType::Boolean
            } else {
                CanonicalType::TinyInt
            }
        }
        "smallint" => CanonicalType::SmallInt,
        "mediumint" => CanonicalType::Int,
        "int" | "integer" => CanonicalType::Int,
        "bigint" => CanonicalType::BigInt,
        "float" => CanonicalType::Float,
        "double" | "double precision" => CanonicalType::Double,
        "decimal" | "numeric" | "dec" | "fixed" => CanonicalType::Decimal {
            precision: p1.unwrap_or(10) as u8,
            scale: p2.unwrap_or(0) as u8,
        },
        "char" => CanonicalType::Char(p1.unwrap_or(1)),
        "varchar" => CanonicalType::Varchar(p1.unwrap_or(255)),
        "tinytext" => CanonicalType::Varchar(255),
        "text" => CanonicalType::Text,
        "mediumtext" => CanonicalType::Text,
        "longtext" => CanonicalType::Text,
        "binary" => CanonicalType::Binary(p1.unwrap_or(1)),
        "varbinary" => CanonicalType::Varbinary(p1.unwrap_or(1)),
        "tinyblob" => CanonicalType::Varbinary(255),
        "blob" => CanonicalType::Blob,
        "mediumblob" => CanonicalType::Blob,
        "longblob" => CanonicalType::Blob,
        "date" => CanonicalType::Date,
        "time" => CanonicalType::Time,
        "datetime" => CanonicalType::DateTime,
        "timestamp" => CanonicalType::Timestamp,
        "year" => CanonicalType::SmallInt,
        "json" => CanonicalType::Json,
        "enum" | "set" => CanonicalType::Varchar(255),
        "bit" => CanonicalType::Binary(p1.map(|v| (v + 7) / 8).unwrap_or(1)),
        "geometry" | "point" | "linestring" | "polygon" | "multipoint" | "multilinestring"
        | "multipolygon" | "geometrycollection" => CanonicalType::Unknown(base),
        _ => CanonicalType::Unknown(native.to_string()),
    }
}

fn canonical_to_mysql(canonical: &CanonicalType) -> String {
    match canonical {
        CanonicalType::Boolean => "TINYINT(1)".into(),
        CanonicalType::TinyInt => "TINYINT".into(),
        CanonicalType::SmallInt => "SMALLINT".into(),
        CanonicalType::Int => "INT".into(),
        CanonicalType::BigInt => "BIGINT".into(),
        CanonicalType::Float => "FLOAT".into(),
        CanonicalType::Double => "DOUBLE".into(),
        CanonicalType::Decimal { precision, scale } => format!("DECIMAL({},{})", precision, scale),
        CanonicalType::Char(n) => format!("CHAR({})", n),
        CanonicalType::Varchar(n) => format!("VARCHAR({})", n),
        CanonicalType::Text | CanonicalType::NText => "LONGTEXT".into(),
        CanonicalType::NChar(n) => format!("CHAR({})", n),
        CanonicalType::NVarchar(n) => format!("VARCHAR({})", n),
        CanonicalType::Binary(n) => format!("BINARY({})", n),
        CanonicalType::Varbinary(n) => format!("VARBINARY({})", n),
        CanonicalType::Blob => "LONGBLOB".into(),
        CanonicalType::Date => "DATE".into(),
        CanonicalType::Time => "TIME".into(),
        CanonicalType::DateTime => "DATETIME".into(),
        CanonicalType::Timestamp => "TIMESTAMP".into(),
        CanonicalType::Uuid => "CHAR(36)".into(),
        CanonicalType::Json => "JSON".into(),
        CanonicalType::Xml => "LONGTEXT".into(),
        CanonicalType::Array(_) => "JSON".into(),
        CanonicalType::Unknown(s) => s.to_uppercase(),
    }
}

// ---------------------------------------------------------------------------
// SQLite (type affinity rules)
// ---------------------------------------------------------------------------

fn sqlite_to_canonical(native: &str) -> CanonicalType {
    let lower = native.to_lowercase();
    let trimmed = lower.trim();

    // Exact matches first
    match trimmed {
        "boolean" | "bool" => return CanonicalType::Boolean,
        "blob" => return CanonicalType::Blob,
        "" => return CanonicalType::Blob, // SQLite default for no type
        _ => {}
    }

    // SQLite type affinity rules (per SQLite documentation section 3.1)
    // Rule 1: If the type contains "INT", it has INTEGER affinity
    if trimmed.contains("int") {
        return CanonicalType::BigInt;
    }
    // Rule 2: If the type contains "CHAR", "CLOB", or "TEXT", it has TEXT affinity
    if trimmed.contains("char") || trimmed.contains("clob") || trimmed.contains("text") {
        return CanonicalType::Text;
    }
    // Rule 3: If the type contains "BLOB", it has BLOB affinity
    if trimmed.contains("blob") {
        return CanonicalType::Blob;
    }
    // Rule 4: If the type contains "REAL", "FLOA", or "DOUB", it has REAL affinity
    if trimmed.contains("real") || trimmed.contains("floa") || trimmed.contains("doub") {
        return CanonicalType::Double;
    }
    // Rule 5: Otherwise, NUMERIC affinity -> map to Double
    CanonicalType::Double
}

fn canonical_to_sqlite(canonical: &CanonicalType) -> String {
    match canonical {
        CanonicalType::Boolean => "INTEGER".into(),
        CanonicalType::TinyInt
        | CanonicalType::SmallInt
        | CanonicalType::Int
        | CanonicalType::BigInt => "INTEGER".into(),
        CanonicalType::Float | CanonicalType::Double => "REAL".into(),
        CanonicalType::Decimal { .. } => "REAL".into(),
        CanonicalType::Char(_) | CanonicalType::Varchar(_) | CanonicalType::Text => "TEXT".into(),
        CanonicalType::NChar(_) | CanonicalType::NVarchar(_) | CanonicalType::NText => {
            "TEXT".into()
        }
        CanonicalType::Binary(_) | CanonicalType::Varbinary(_) | CanonicalType::Blob => {
            "BLOB".into()
        }
        CanonicalType::Date
        | CanonicalType::Time
        | CanonicalType::DateTime
        | CanonicalType::Timestamp => "TEXT".into(),
        CanonicalType::Uuid => "TEXT".into(),
        CanonicalType::Json => "TEXT".into(),
        CanonicalType::Xml => "TEXT".into(),
        CanonicalType::Array(_) => "TEXT".into(),
        CanonicalType::Unknown(s) => s.to_uppercase(),
    }
}

// ---------------------------------------------------------------------------
// Oracle
// ---------------------------------------------------------------------------

fn oracle_to_canonical(native: &str) -> CanonicalType {
    let (base, p1, p2) = parse_native_type(native);
    match base.as_str() {
        "number" => {
            match (p1, p2) {
                (Some(1), None) | (Some(1), Some(0)) => CanonicalType::Boolean,
                (Some(p), Some(s)) if s > 0 => CanonicalType::Decimal {
                    precision: p as u8,
                    scale: s as u8,
                },
                (Some(p), _) if p <= 5 => CanonicalType::SmallInt,
                (Some(p), _) if p <= 10 => CanonicalType::Int,
                (Some(p), _) if p <= 19 => CanonicalType::BigInt,
                (Some(p), _) => CanonicalType::Decimal {
                    precision: p as u8,
                    scale: 0,
                },
                (None, _) => CanonicalType::Decimal {
                    precision: 38,
                    scale: 0,
                },
            }
        }
        "binary_float" => CanonicalType::Float,
        "binary_double" => CanonicalType::Double,
        "float" => CanonicalType::Double,
        "char" => CanonicalType::Char(p1.unwrap_or(1)),
        "varchar2" => CanonicalType::Varchar(p1.unwrap_or(1)),
        "nchar" => CanonicalType::NChar(p1.unwrap_or(1)),
        "nvarchar2" => CanonicalType::NVarchar(p1.unwrap_or(1)),
        "clob" => CanonicalType::Text,
        "nclob" => CanonicalType::NText,
        "long" => CanonicalType::Text,
        "raw" => {
            if p1 == Some(16) {
                CanonicalType::Uuid
            } else {
                CanonicalType::Varbinary(p1.unwrap_or(1))
            }
        }
        "long raw" => CanonicalType::Blob,
        "blob" => CanonicalType::Blob,
        "date" => CanonicalType::DateTime,
        "timestamp" => CanonicalType::Timestamp,
        "timestamp with time zone" => CanonicalType::Timestamp,
        "timestamp with local time zone" => CanonicalType::Timestamp,
        "interval year to month" => CanonicalType::Text,
        "interval day to second" => CanonicalType::Text,
        "rowid" | "urowid" => CanonicalType::Varchar(18),
        "xmltype" => CanonicalType::Xml,
        "bfile" => CanonicalType::Blob,
        _ => CanonicalType::Unknown(native.to_string()),
    }
}

fn canonical_to_oracle(canonical: &CanonicalType) -> String {
    match canonical {
        CanonicalType::Boolean => "NUMBER(1)".into(),
        CanonicalType::TinyInt => "NUMBER(3)".into(),
        CanonicalType::SmallInt => "NUMBER(5)".into(),
        CanonicalType::Int => "NUMBER(10)".into(),
        CanonicalType::BigInt => "NUMBER(19)".into(),
        CanonicalType::Float => "BINARY_FLOAT".into(),
        CanonicalType::Double => "BINARY_DOUBLE".into(),
        CanonicalType::Decimal { precision, scale } => format!("NUMBER({},{})", precision, scale),
        CanonicalType::Char(n) => format!("CHAR({})", n),
        CanonicalType::Varchar(n) => format!("VARCHAR2({})", n),
        CanonicalType::Text => "CLOB".into(),
        CanonicalType::NChar(n) => format!("NCHAR({})", n),
        CanonicalType::NVarchar(n) => format!("NVARCHAR2({})", n),
        CanonicalType::NText => "NCLOB".into(),
        CanonicalType::Binary(n) => format!("RAW({})", n),
        CanonicalType::Varbinary(n) => format!("RAW({})", n),
        CanonicalType::Blob => "BLOB".into(),
        CanonicalType::Date => "DATE".into(),
        CanonicalType::Time => "TIMESTAMP".into(),
        CanonicalType::DateTime => "TIMESTAMP".into(),
        CanonicalType::Timestamp => "TIMESTAMP WITH TIME ZONE".into(),
        CanonicalType::Uuid => "RAW(16)".into(),
        CanonicalType::Json => "CLOB".into(),
        CanonicalType::Xml => "XMLTYPE".into(),
        CanonicalType::Array(_) => "CLOB".into(),
        CanonicalType::Unknown(s) => s.to_uppercase(),
    }
}

// ---------------------------------------------------------------------------
// MongoDB
// ---------------------------------------------------------------------------

fn mongodb_to_canonical(native: &str) -> CanonicalType {
    match native.trim() {
        "bool" => CanonicalType::Boolean,
        "int" | "int32" => CanonicalType::Int,
        "long" | "int64" => CanonicalType::BigInt,
        "double" => CanonicalType::Double,
        "decimal128" | "decimal" => CanonicalType::Decimal {
            precision: 34,
            scale: 6,
        },
        "string" => CanonicalType::Text,
        "date" => CanonicalType::DateTime,
        "timestamp" => CanonicalType::Timestamp,
        "objectid" => CanonicalType::Varchar(24),
        "bindata" | "binData" => CanonicalType::Blob,
        "object" => CanonicalType::Json,
        "array" => CanonicalType::Json,
        "regex" => CanonicalType::Varchar(255),
        "javascript" | "javascriptwithscope" => CanonicalType::Text,
        "null" | "undefined" => CanonicalType::Unknown("null".into()),
        "minkey" | "maxkey" => CanonicalType::Unknown(native.to_string()),
        _ => CanonicalType::Unknown(native.to_string()),
    }
}

fn canonical_to_mongodb(canonical: &CanonicalType) -> String {
    match canonical {
        CanonicalType::Boolean => "bool".into(),
        CanonicalType::TinyInt | CanonicalType::SmallInt | CanonicalType::Int => "int".into(),
        CanonicalType::BigInt => "long".into(),
        CanonicalType::Float | CanonicalType::Double => "double".into(),
        CanonicalType::Decimal { .. } => "decimal128".into(),
        CanonicalType::Char(_) | CanonicalType::Varchar(_) | CanonicalType::Text => {
            "string".into()
        }
        CanonicalType::NChar(_) | CanonicalType::NVarchar(_) | CanonicalType::NText => {
            "string".into()
        }
        CanonicalType::Binary(_) | CanonicalType::Varbinary(_) | CanonicalType::Blob => {
            "binData".into()
        }
        CanonicalType::Date
        | CanonicalType::Time
        | CanonicalType::DateTime
        | CanonicalType::Timestamp => "date".into(),
        CanonicalType::Uuid => "string".into(),
        CanonicalType::Json => "object".into(),
        CanonicalType::Xml => "string".into(),
        CanonicalType::Array(_) => "array".into(),
        CanonicalType::Unknown(s) => s.clone(),
    }
}

// ---------------------------------------------------------------------------
// CosmosDB
// ---------------------------------------------------------------------------

fn cosmosdb_to_canonical(native: &str) -> CanonicalType {
    match native.trim() {
        "boolean" => CanonicalType::Boolean,
        "number" => CanonicalType::Double,
        "string" => CanonicalType::Text,
        "array" => CanonicalType::Json,
        "object" => CanonicalType::Json,
        "null" => CanonicalType::Unknown("null".into()),
        _ => CanonicalType::Unknown(native.to_string()),
    }
}

fn canonical_to_cosmosdb(canonical: &CanonicalType) -> String {
    match canonical {
        CanonicalType::Boolean => "boolean".into(),
        CanonicalType::TinyInt
        | CanonicalType::SmallInt
        | CanonicalType::Int
        | CanonicalType::BigInt => "number".into(),
        CanonicalType::Float | CanonicalType::Double | CanonicalType::Decimal { .. } => {
            "number".into()
        }
        CanonicalType::Char(_) | CanonicalType::Varchar(_) | CanonicalType::Text => {
            "string".into()
        }
        CanonicalType::NChar(_) | CanonicalType::NVarchar(_) | CanonicalType::NText => {
            "string".into()
        }
        CanonicalType::Binary(_) | CanonicalType::Varbinary(_) | CanonicalType::Blob => {
            "string".into()
        }
        CanonicalType::Date
        | CanonicalType::Time
        | CanonicalType::DateTime
        | CanonicalType::Timestamp => "string".into(),
        CanonicalType::Uuid => "string".into(),
        CanonicalType::Json => "object".into(),
        CanonicalType::Xml => "string".into(),
        CanonicalType::Array(_) => "array".into(),
        CanonicalType::Unknown(s) => s.clone(),
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // parse_native_type tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_simple_type() {
        let (base, p1, p2) = parse_native_type("INT");
        assert_eq!(base, "int");
        assert_eq!(p1, None);
        assert_eq!(p2, None);
    }

    #[test]
    fn test_parse_single_param() {
        let (base, p1, p2) = parse_native_type("VARCHAR(255)");
        assert_eq!(base, "varchar");
        assert_eq!(p1, Some(255));
        assert_eq!(p2, None);
    }

    #[test]
    fn test_parse_two_params() {
        let (base, p1, p2) = parse_native_type("DECIMAL(18,2)");
        assert_eq!(base, "decimal");
        assert_eq!(p1, Some(18));
        assert_eq!(p2, Some(2));
    }

    #[test]
    fn test_parse_max_param() {
        let (base, p1, p2) = parse_native_type("VARCHAR(MAX)");
        assert_eq!(base, "varchar");
        assert_eq!(p1, Some(u32::MAX));
        assert_eq!(p2, None);
    }

    #[test]
    fn test_parse_nvarchar_max() {
        let (base, p1, _) = parse_native_type("NVARCHAR(MAX)");
        assert_eq!(base, "nvarchar");
        assert_eq!(p1, Some(u32::MAX));
    }

    #[test]
    fn test_parse_empty_string() {
        let (base, p1, p2) = parse_native_type("");
        assert_eq!(base, "");
        assert_eq!(p1, None);
        assert_eq!(p2, None);
    }

    #[test]
    fn test_parse_spaces() {
        let (base, p1, _) = parse_native_type("  VARCHAR( 100 )  ");
        assert_eq!(base, "varchar");
        assert_eq!(p1, Some(100));
    }

    // -----------------------------------------------------------------------
    // SQL Server to_canonical tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_sqlserver_bit() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "BIT"),
            CanonicalType::Boolean
        );
    }

    #[test]
    fn test_sqlserver_tinyint() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "TINYINT"),
            CanonicalType::TinyInt
        );
    }

    #[test]
    fn test_sqlserver_smallint() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "SMALLINT"),
            CanonicalType::SmallInt
        );
    }

    #[test]
    fn test_sqlserver_int() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "INT"),
            CanonicalType::Int
        );
    }

    #[test]
    fn test_sqlserver_bigint() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "BIGINT"),
            CanonicalType::BigInt
        );
    }

    #[test]
    fn test_sqlserver_real() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "REAL"),
            CanonicalType::Float
        );
    }

    #[test]
    fn test_sqlserver_float() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "FLOAT"),
            CanonicalType::Double
        );
    }

    #[test]
    fn test_sqlserver_decimal() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "DECIMAL(18,2)"),
            CanonicalType::Decimal {
                precision: 18,
                scale: 2
            }
        );
    }

    #[test]
    fn test_sqlserver_numeric() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "NUMERIC(10,4)"),
            CanonicalType::Decimal {
                precision: 10,
                scale: 4
            }
        );
    }

    #[test]
    fn test_sqlserver_money() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "MONEY"),
            CanonicalType::Decimal {
                precision: 19,
                scale: 4
            }
        );
    }

    #[test]
    fn test_sqlserver_smallmoney() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "SMALLMONEY"),
            CanonicalType::Decimal {
                precision: 10,
                scale: 4
            }
        );
    }

    #[test]
    fn test_sqlserver_varchar() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "VARCHAR(100)"),
            CanonicalType::Varchar(100)
        );
    }

    #[test]
    fn test_sqlserver_varchar_max() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "VARCHAR(MAX)"),
            CanonicalType::Text
        );
    }

    #[test]
    fn test_sqlserver_nvarchar() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "NVARCHAR(50)"),
            CanonicalType::NVarchar(50)
        );
    }

    #[test]
    fn test_sqlserver_nvarchar_max() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "NVARCHAR(MAX)"),
            CanonicalType::NText
        );
    }

    #[test]
    fn test_sqlserver_nchar() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "NCHAR(10)"),
            CanonicalType::NChar(10)
        );
    }

    #[test]
    fn test_sqlserver_char() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "CHAR(5)"),
            CanonicalType::Char(5)
        );
    }

    #[test]
    fn test_sqlserver_binary() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "BINARY(16)"),
            CanonicalType::Binary(16)
        );
    }

    #[test]
    fn test_sqlserver_varbinary() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "VARBINARY(256)"),
            CanonicalType::Varbinary(256)
        );
    }

    #[test]
    fn test_sqlserver_varbinary_max() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "VARBINARY(MAX)"),
            CanonicalType::Blob
        );
    }

    #[test]
    fn test_sqlserver_image() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "IMAGE"),
            CanonicalType::Blob
        );
    }

    #[test]
    fn test_sqlserver_text() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "TEXT"),
            CanonicalType::Text
        );
    }

    #[test]
    fn test_sqlserver_ntext() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "NTEXT"),
            CanonicalType::NText
        );
    }

    #[test]
    fn test_sqlserver_date() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "DATE"),
            CanonicalType::Date
        );
    }

    #[test]
    fn test_sqlserver_time() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "TIME"),
            CanonicalType::Time
        );
    }

    #[test]
    fn test_sqlserver_datetime() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "DATETIME"),
            CanonicalType::DateTime
        );
    }

    #[test]
    fn test_sqlserver_datetime2() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "DATETIME2"),
            CanonicalType::DateTime
        );
    }

    #[test]
    fn test_sqlserver_smalldatetime() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "SMALLDATETIME"),
            CanonicalType::DateTime
        );
    }

    #[test]
    fn test_sqlserver_datetimeoffset() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "DATETIMEOFFSET"),
            CanonicalType::Timestamp
        );
    }

    #[test]
    fn test_sqlserver_uniqueidentifier() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "UNIQUEIDENTIFIER"),
            CanonicalType::Uuid
        );
    }

    #[test]
    fn test_sqlserver_xml() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "XML"),
            CanonicalType::Xml
        );
    }

    #[test]
    fn test_sqlserver_timestamp() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "TIMESTAMP"),
            CanonicalType::Varbinary(8)
        );
    }

    #[test]
    fn test_sqlserver_rowversion() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "ROWVERSION"),
            CanonicalType::Varbinary(8)
        );
    }

    #[test]
    fn test_sqlserver_geography() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "GEOGRAPHY"),
            CanonicalType::Unknown("geography".into())
        );
    }

    #[test]
    fn test_sqlserver_geometry() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "GEOMETRY"),
            CanonicalType::Unknown("geometry".into())
        );
    }

    #[test]
    fn test_sqlserver_sql_variant() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "SQL_VARIANT"),
            CanonicalType::Unknown("sql_variant".into())
        );
    }

    #[test]
    fn test_sqlserver_hierarchyid() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "HIERARCHYID"),
            CanonicalType::Unknown("hierarchyid".into())
        );
    }

    // -----------------------------------------------------------------------
    // PostgreSQL to_canonical tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_postgres_boolean() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "BOOLEAN"),
            CanonicalType::Boolean
        );
    }

    #[test]
    fn test_postgres_bool() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "bool"),
            CanonicalType::Boolean
        );
    }

    #[test]
    fn test_postgres_smallint() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "SMALLINT"),
            CanonicalType::SmallInt
        );
    }

    #[test]
    fn test_postgres_int2() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "int2"),
            CanonicalType::SmallInt
        );
    }

    #[test]
    fn test_postgres_integer() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "INTEGER"),
            CanonicalType::Int
        );
    }

    #[test]
    fn test_postgres_int4() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "int4"),
            CanonicalType::Int
        );
    }

    #[test]
    fn test_postgres_bigint() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "BIGINT"),
            CanonicalType::BigInt
        );
    }

    #[test]
    fn test_postgres_int8() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "int8"),
            CanonicalType::BigInt
        );
    }

    #[test]
    fn test_postgres_serial() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "SERIAL"),
            CanonicalType::Int
        );
    }

    #[test]
    fn test_postgres_bigserial() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "BIGSERIAL"),
            CanonicalType::BigInt
        );
    }

    #[test]
    fn test_postgres_smallserial() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "SMALLSERIAL"),
            CanonicalType::SmallInt
        );
    }

    #[test]
    fn test_postgres_real() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "REAL"),
            CanonicalType::Float
        );
    }

    #[test]
    fn test_postgres_float4() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "float4"),
            CanonicalType::Float
        );
    }

    #[test]
    fn test_postgres_double_precision() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "double precision"),
            CanonicalType::Double
        );
    }

    #[test]
    fn test_postgres_float8() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "float8"),
            CanonicalType::Double
        );
    }

    #[test]
    fn test_postgres_numeric() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "NUMERIC(12,4)"),
            CanonicalType::Decimal {
                precision: 12,
                scale: 4
            }
        );
    }

    #[test]
    fn test_postgres_decimal() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "DECIMAL(8,2)"),
            CanonicalType::Decimal {
                precision: 8,
                scale: 2
            }
        );
    }

    #[test]
    fn test_postgres_money() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "MONEY"),
            CanonicalType::Decimal {
                precision: 19,
                scale: 4
            }
        );
    }

    #[test]
    fn test_postgres_varchar() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "VARCHAR(200)"),
            CanonicalType::Varchar(200)
        );
    }

    #[test]
    fn test_postgres_character_varying() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "character varying(100)"),
            CanonicalType::Varchar(100)
        );
    }

    #[test]
    fn test_postgres_char() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "CHAR(10)"),
            CanonicalType::Char(10)
        );
    }

    #[test]
    fn test_postgres_character() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "CHARACTER(5)"),
            CanonicalType::Char(5)
        );
    }

    #[test]
    fn test_postgres_text() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "TEXT"),
            CanonicalType::Text
        );
    }

    #[test]
    fn test_postgres_bytea() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "BYTEA"),
            CanonicalType::Blob
        );
    }

    #[test]
    fn test_postgres_date() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "DATE"),
            CanonicalType::Date
        );
    }

    #[test]
    fn test_postgres_time() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "TIME"),
            CanonicalType::Time
        );
    }

    #[test]
    fn test_postgres_time_without_tz() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "time without time zone"),
            CanonicalType::Time
        );
    }

    #[test]
    fn test_postgres_timestamp() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "TIMESTAMP"),
            CanonicalType::DateTime
        );
    }

    #[test]
    fn test_postgres_timestamptz() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "TIMESTAMPTZ"),
            CanonicalType::Timestamp
        );
    }

    #[test]
    fn test_postgres_timestamp_with_tz() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "timestamp with time zone"),
            CanonicalType::Timestamp
        );
    }

    #[test]
    fn test_postgres_interval() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "INTERVAL"),
            CanonicalType::Text
        );
    }

    #[test]
    fn test_postgres_uuid() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "UUID"),
            CanonicalType::Uuid
        );
    }

    #[test]
    fn test_postgres_json() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "JSON"),
            CanonicalType::Json
        );
    }

    #[test]
    fn test_postgres_jsonb() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "JSONB"),
            CanonicalType::Json
        );
    }

    #[test]
    fn test_postgres_xml() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "XML"),
            CanonicalType::Xml
        );
    }

    #[test]
    fn test_postgres_inet() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "INET"),
            CanonicalType::Varchar(43)
        );
    }

    #[test]
    fn test_postgres_cidr() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "CIDR"),
            CanonicalType::Varchar(43)
        );
    }

    #[test]
    fn test_postgres_macaddr() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "MACADDR"),
            CanonicalType::Varchar(17)
        );
    }

    #[test]
    fn test_postgres_tsvector() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "TSVECTOR"),
            CanonicalType::Text
        );
    }

    #[test]
    fn test_postgres_tsquery() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "TSQUERY"),
            CanonicalType::Text
        );
    }

    #[test]
    fn test_postgres_point() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "POINT"),
            CanonicalType::Unknown("point".into())
        );
    }

    #[test]
    fn test_postgres_polygon() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "POLYGON"),
            CanonicalType::Unknown("polygon".into())
        );
    }

    #[test]
    fn test_postgres_integer_array() {
        assert_eq!(
            to_canonical(&DatabaseEngine::PostgreSql, "integer[]"),
            CanonicalType::Array(Box::new(CanonicalType::Int))
        );
    }

    // -----------------------------------------------------------------------
    // MySQL to_canonical tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_mysql_boolean() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "BOOLEAN"),
            CanonicalType::Boolean
        );
    }

    #[test]
    fn test_mysql_tinyint_1() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "TINYINT(1)"),
            CanonicalType::Boolean
        );
    }

    #[test]
    fn test_mysql_tinyint() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "TINYINT"),
            CanonicalType::TinyInt
        );
    }

    #[test]
    fn test_mysql_smallint() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "SMALLINT"),
            CanonicalType::SmallInt
        );
    }

    #[test]
    fn test_mysql_mediumint() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "MEDIUMINT"),
            CanonicalType::Int
        );
    }

    #[test]
    fn test_mysql_int() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "INT"),
            CanonicalType::Int
        );
    }

    #[test]
    fn test_mysql_integer() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "INTEGER"),
            CanonicalType::Int
        );
    }

    #[test]
    fn test_mysql_bigint() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "BIGINT"),
            CanonicalType::BigInt
        );
    }

    #[test]
    fn test_mysql_float() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "FLOAT"),
            CanonicalType::Float
        );
    }

    #[test]
    fn test_mysql_double() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "DOUBLE"),
            CanonicalType::Double
        );
    }

    #[test]
    fn test_mysql_decimal() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "DECIMAL(10,2)"),
            CanonicalType::Decimal {
                precision: 10,
                scale: 2
            }
        );
    }

    #[test]
    fn test_mysql_numeric() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "NUMERIC(8,3)"),
            CanonicalType::Decimal {
                precision: 8,
                scale: 3
            }
        );
    }

    #[test]
    fn test_mysql_char() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "CHAR(20)"),
            CanonicalType::Char(20)
        );
    }

    #[test]
    fn test_mysql_varchar() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "VARCHAR(255)"),
            CanonicalType::Varchar(255)
        );
    }

    #[test]
    fn test_mysql_tinytext() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "TINYTEXT"),
            CanonicalType::Varchar(255)
        );
    }

    #[test]
    fn test_mysql_text() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "TEXT"),
            CanonicalType::Text
        );
    }

    #[test]
    fn test_mysql_mediumtext() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "MEDIUMTEXT"),
            CanonicalType::Text
        );
    }

    #[test]
    fn test_mysql_longtext() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "LONGTEXT"),
            CanonicalType::Text
        );
    }

    #[test]
    fn test_mysql_binary() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "BINARY(16)"),
            CanonicalType::Binary(16)
        );
    }

    #[test]
    fn test_mysql_varbinary() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "VARBINARY(256)"),
            CanonicalType::Varbinary(256)
        );
    }

    #[test]
    fn test_mysql_tinyblob() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "TINYBLOB"),
            CanonicalType::Varbinary(255)
        );
    }

    #[test]
    fn test_mysql_blob() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "BLOB"),
            CanonicalType::Blob
        );
    }

    #[test]
    fn test_mysql_mediumblob() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "MEDIUMBLOB"),
            CanonicalType::Blob
        );
    }

    #[test]
    fn test_mysql_longblob() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "LONGBLOB"),
            CanonicalType::Blob
        );
    }

    #[test]
    fn test_mysql_date() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "DATE"),
            CanonicalType::Date
        );
    }

    #[test]
    fn test_mysql_time() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "TIME"),
            CanonicalType::Time
        );
    }

    #[test]
    fn test_mysql_datetime() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "DATETIME"),
            CanonicalType::DateTime
        );
    }

    #[test]
    fn test_mysql_timestamp() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "TIMESTAMP"),
            CanonicalType::Timestamp
        );
    }

    #[test]
    fn test_mysql_year() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "YEAR"),
            CanonicalType::SmallInt
        );
    }

    #[test]
    fn test_mysql_json() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "JSON"),
            CanonicalType::Json
        );
    }

    #[test]
    fn test_mysql_enum() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "ENUM"),
            CanonicalType::Varchar(255)
        );
    }

    #[test]
    fn test_mysql_set() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "SET"),
            CanonicalType::Varchar(255)
        );
    }

    #[test]
    fn test_mysql_geometry() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "GEOMETRY"),
            CanonicalType::Unknown("geometry".into())
        );
    }

    #[test]
    fn test_mysql_point() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MySql, "POINT"),
            CanonicalType::Unknown("point".into())
        );
    }

    // -----------------------------------------------------------------------
    // SQLite to_canonical tests (type affinity)
    // -----------------------------------------------------------------------

    #[test]
    fn test_sqlite_boolean() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Sqlite, "BOOLEAN"),
            CanonicalType::Boolean
        );
    }

    #[test]
    fn test_sqlite_integer() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Sqlite, "INTEGER"),
            CanonicalType::BigInt
        );
    }

    #[test]
    fn test_sqlite_int() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Sqlite, "INT"),
            CanonicalType::BigInt
        );
    }

    #[test]
    fn test_sqlite_bigint_affinity() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Sqlite, "BIGINT"),
            CanonicalType::BigInt
        );
    }

    #[test]
    fn test_sqlite_tinyint_affinity() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Sqlite, "TINYINT"),
            CanonicalType::BigInt
        );
    }

    #[test]
    fn test_sqlite_smallint_affinity() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Sqlite, "SMALLINT"),
            CanonicalType::BigInt
        );
    }

    #[test]
    fn test_sqlite_mediumint_affinity() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Sqlite, "MEDIUMINT"),
            CanonicalType::BigInt
        );
    }

    #[test]
    fn test_sqlite_int8_affinity() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Sqlite, "INT8"),
            CanonicalType::BigInt
        );
    }

    #[test]
    fn test_sqlite_text() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Sqlite, "TEXT"),
            CanonicalType::Text
        );
    }

    #[test]
    fn test_sqlite_varchar_affinity() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Sqlite, "VARCHAR(255)"),
            CanonicalType::Text
        );
    }

    #[test]
    fn test_sqlite_nchar_affinity() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Sqlite, "NCHAR(10)"),
            CanonicalType::Text
        );
    }

    #[test]
    fn test_sqlite_clob_affinity() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Sqlite, "CLOB"),
            CanonicalType::Text
        );
    }

    #[test]
    fn test_sqlite_blob() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Sqlite, "BLOB"),
            CanonicalType::Blob
        );
    }

    #[test]
    fn test_sqlite_real() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Sqlite, "REAL"),
            CanonicalType::Double
        );
    }

    #[test]
    fn test_sqlite_double_affinity() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Sqlite, "DOUBLE"),
            CanonicalType::Double
        );
    }

    #[test]
    fn test_sqlite_float_affinity() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Sqlite, "FLOAT"),
            CanonicalType::Double
        );
    }

    #[test]
    fn test_sqlite_numeric_affinity() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Sqlite, "NUMERIC"),
            CanonicalType::Double
        );
    }

    #[test]
    fn test_sqlite_decimal_affinity() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Sqlite, "DECIMAL(10,2)"),
            CanonicalType::Double
        );
    }

    #[test]
    fn test_sqlite_empty_type() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Sqlite, ""),
            CanonicalType::Blob
        );
    }

    // -----------------------------------------------------------------------
    // Oracle to_canonical tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_oracle_number_1() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "NUMBER(1)"),
            CanonicalType::Boolean
        );
    }

    #[test]
    fn test_oracle_number_5() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "NUMBER(5)"),
            CanonicalType::SmallInt
        );
    }

    #[test]
    fn test_oracle_number_10() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "NUMBER(10)"),
            CanonicalType::Int
        );
    }

    #[test]
    fn test_oracle_number_19() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "NUMBER(19)"),
            CanonicalType::BigInt
        );
    }

    #[test]
    fn test_oracle_number_38() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "NUMBER(38)"),
            CanonicalType::Decimal {
                precision: 38,
                scale: 0
            }
        );
    }

    #[test]
    fn test_oracle_number_no_params() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "NUMBER"),
            CanonicalType::Decimal {
                precision: 38,
                scale: 0
            }
        );
    }

    #[test]
    fn test_oracle_number_with_scale() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "NUMBER(10,2)"),
            CanonicalType::Decimal {
                precision: 10,
                scale: 2
            }
        );
    }

    #[test]
    fn test_oracle_binary_float() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "BINARY_FLOAT"),
            CanonicalType::Float
        );
    }

    #[test]
    fn test_oracle_binary_double() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "BINARY_DOUBLE"),
            CanonicalType::Double
        );
    }

    #[test]
    fn test_oracle_float() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "FLOAT"),
            CanonicalType::Double
        );
    }

    #[test]
    fn test_oracle_varchar2() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "VARCHAR2(100)"),
            CanonicalType::Varchar(100)
        );
    }

    #[test]
    fn test_oracle_nvarchar2() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "NVARCHAR2(50)"),
            CanonicalType::NVarchar(50)
        );
    }

    #[test]
    fn test_oracle_char() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "CHAR(10)"),
            CanonicalType::Char(10)
        );
    }

    #[test]
    fn test_oracle_nchar() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "NCHAR(5)"),
            CanonicalType::NChar(5)
        );
    }

    #[test]
    fn test_oracle_clob() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "CLOB"),
            CanonicalType::Text
        );
    }

    #[test]
    fn test_oracle_nclob() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "NCLOB"),
            CanonicalType::NText
        );
    }

    #[test]
    fn test_oracle_long() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "LONG"),
            CanonicalType::Text
        );
    }

    #[test]
    fn test_oracle_blob() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "BLOB"),
            CanonicalType::Blob
        );
    }

    #[test]
    fn test_oracle_long_raw() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "LONG RAW"),
            CanonicalType::Blob
        );
    }

    #[test]
    fn test_oracle_raw_16() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "RAW(16)"),
            CanonicalType::Uuid
        );
    }

    #[test]
    fn test_oracle_raw_other() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "RAW(32)"),
            CanonicalType::Varbinary(32)
        );
    }

    #[test]
    fn test_oracle_date() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "DATE"),
            CanonicalType::DateTime
        );
    }

    #[test]
    fn test_oracle_timestamp() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "TIMESTAMP"),
            CanonicalType::Timestamp
        );
    }

    #[test]
    fn test_oracle_timestamp_with_tz() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "TIMESTAMP WITH TIME ZONE"),
            CanonicalType::Timestamp
        );
    }

    #[test]
    fn test_oracle_timestamp_with_local_tz() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "TIMESTAMP WITH LOCAL TIME ZONE"),
            CanonicalType::Timestamp
        );
    }

    #[test]
    fn test_oracle_interval_ym() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "INTERVAL YEAR TO MONTH"),
            CanonicalType::Text
        );
    }

    #[test]
    fn test_oracle_interval_ds() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "INTERVAL DAY TO SECOND"),
            CanonicalType::Text
        );
    }

    #[test]
    fn test_oracle_rowid() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "ROWID"),
            CanonicalType::Varchar(18)
        );
    }

    #[test]
    fn test_oracle_urowid() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "UROWID"),
            CanonicalType::Varchar(18)
        );
    }

    #[test]
    fn test_oracle_xmltype() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "XMLTYPE"),
            CanonicalType::Xml
        );
    }

    #[test]
    fn test_oracle_bfile() {
        assert_eq!(
            to_canonical(&DatabaseEngine::Oracle, "BFILE"),
            CanonicalType::Blob
        );
    }

    // -----------------------------------------------------------------------
    // MongoDB to_canonical tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_mongodb_bool() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MongoDb, "bool"),
            CanonicalType::Boolean
        );
    }

    #[test]
    fn test_mongodb_int() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MongoDb, "int"),
            CanonicalType::Int
        );
    }

    #[test]
    fn test_mongodb_int32() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MongoDb, "int32"),
            CanonicalType::Int
        );
    }

    #[test]
    fn test_mongodb_long() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MongoDb, "long"),
            CanonicalType::BigInt
        );
    }

    #[test]
    fn test_mongodb_int64() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MongoDb, "int64"),
            CanonicalType::BigInt
        );
    }

    #[test]
    fn test_mongodb_double() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MongoDb, "double"),
            CanonicalType::Double
        );
    }

    #[test]
    fn test_mongodb_decimal128() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MongoDb, "decimal128"),
            CanonicalType::Decimal {
                precision: 34,
                scale: 6
            }
        );
    }

    #[test]
    fn test_mongodb_string() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MongoDb, "string"),
            CanonicalType::Text
        );
    }

    #[test]
    fn test_mongodb_date() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MongoDb, "date"),
            CanonicalType::DateTime
        );
    }

    #[test]
    fn test_mongodb_timestamp() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MongoDb, "timestamp"),
            CanonicalType::Timestamp
        );
    }

    #[test]
    fn test_mongodb_objectid() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MongoDb, "objectid"),
            CanonicalType::Varchar(24)
        );
    }

    #[test]
    fn test_mongodb_bindata() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MongoDb, "bindata"),
            CanonicalType::Blob
        );
    }

    #[test]
    fn test_mongodb_object() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MongoDb, "object"),
            CanonicalType::Json
        );
    }

    #[test]
    fn test_mongodb_array() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MongoDb, "array"),
            CanonicalType::Json
        );
    }

    #[test]
    fn test_mongodb_regex() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MongoDb, "regex"),
            CanonicalType::Varchar(255)
        );
    }

    #[test]
    fn test_mongodb_javascript() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MongoDb, "javascript"),
            CanonicalType::Text
        );
    }

    #[test]
    fn test_mongodb_null() {
        assert_eq!(
            to_canonical(&DatabaseEngine::MongoDb, "null"),
            CanonicalType::Unknown("null".into())
        );
    }

    // -----------------------------------------------------------------------
    // CosmosDB to_canonical tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_cosmosdb_boolean() {
        assert_eq!(
            to_canonical(&DatabaseEngine::CosmosDb, "boolean"),
            CanonicalType::Boolean
        );
    }

    #[test]
    fn test_cosmosdb_number() {
        assert_eq!(
            to_canonical(&DatabaseEngine::CosmosDb, "number"),
            CanonicalType::Double
        );
    }

    #[test]
    fn test_cosmosdb_string() {
        assert_eq!(
            to_canonical(&DatabaseEngine::CosmosDb, "string"),
            CanonicalType::Text
        );
    }

    #[test]
    fn test_cosmosdb_array() {
        assert_eq!(
            to_canonical(&DatabaseEngine::CosmosDb, "array"),
            CanonicalType::Json
        );
    }

    #[test]
    fn test_cosmosdb_object() {
        assert_eq!(
            to_canonical(&DatabaseEngine::CosmosDb, "object"),
            CanonicalType::Json
        );
    }

    #[test]
    fn test_cosmosdb_null() {
        assert_eq!(
            to_canonical(&DatabaseEngine::CosmosDb, "null"),
            CanonicalType::Unknown("null".into())
        );
    }

    // -----------------------------------------------------------------------
    // from_canonical tests (all engines)
    // -----------------------------------------------------------------------

    #[test]
    fn test_from_canonical_sqlserver_all() {
        assert_eq!(
            from_canonical(&DatabaseEngine::SqlServer, &CanonicalType::Boolean),
            "BIT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::SqlServer, &CanonicalType::TinyInt),
            "TINYINT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::SqlServer, &CanonicalType::SmallInt),
            "SMALLINT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::SqlServer, &CanonicalType::Int),
            "INT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::SqlServer, &CanonicalType::BigInt),
            "BIGINT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::SqlServer, &CanonicalType::Float),
            "REAL"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::SqlServer, &CanonicalType::Double),
            "FLOAT"
        );
        assert_eq!(
            from_canonical(
                &DatabaseEngine::SqlServer,
                &CanonicalType::Decimal {
                    precision: 10,
                    scale: 2
                }
            ),
            "DECIMAL(10,2)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::SqlServer, &CanonicalType::Char(10)),
            "CHAR(10)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::SqlServer, &CanonicalType::Varchar(255)),
            "VARCHAR(255)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::SqlServer, &CanonicalType::Text),
            "VARCHAR(MAX)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::SqlServer, &CanonicalType::NChar(10)),
            "NCHAR(10)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::SqlServer, &CanonicalType::NVarchar(50)),
            "NVARCHAR(50)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::SqlServer, &CanonicalType::NText),
            "NVARCHAR(MAX)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::SqlServer, &CanonicalType::Binary(16)),
            "BINARY(16)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::SqlServer, &CanonicalType::Varbinary(256)),
            "VARBINARY(256)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::SqlServer, &CanonicalType::Blob),
            "VARBINARY(MAX)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::SqlServer, &CanonicalType::Date),
            "DATE"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::SqlServer, &CanonicalType::Time),
            "TIME"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::SqlServer, &CanonicalType::DateTime),
            "DATETIME2"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::SqlServer, &CanonicalType::Timestamp),
            "DATETIMEOFFSET"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::SqlServer, &CanonicalType::Uuid),
            "UNIQUEIDENTIFIER"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::SqlServer, &CanonicalType::Json),
            "NVARCHAR(MAX)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::SqlServer, &CanonicalType::Xml),
            "XML"
        );
        assert_eq!(
            from_canonical(
                &DatabaseEngine::SqlServer,
                &CanonicalType::Array(Box::new(CanonicalType::Int))
            ),
            "NVARCHAR(MAX)"
        );
    }

    #[test]
    fn test_from_canonical_postgres_all() {
        assert_eq!(
            from_canonical(&DatabaseEngine::PostgreSql, &CanonicalType::Boolean),
            "BOOLEAN"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::PostgreSql, &CanonicalType::TinyInt),
            "SMALLINT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::PostgreSql, &CanonicalType::SmallInt),
            "SMALLINT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::PostgreSql, &CanonicalType::Int),
            "INTEGER"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::PostgreSql, &CanonicalType::BigInt),
            "BIGINT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::PostgreSql, &CanonicalType::Float),
            "REAL"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::PostgreSql, &CanonicalType::Double),
            "DOUBLE PRECISION"
        );
        assert_eq!(
            from_canonical(
                &DatabaseEngine::PostgreSql,
                &CanonicalType::Decimal {
                    precision: 10,
                    scale: 2
                }
            ),
            "NUMERIC(10,2)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::PostgreSql, &CanonicalType::Char(10)),
            "CHAR(10)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::PostgreSql, &CanonicalType::Varchar(255)),
            "VARCHAR(255)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::PostgreSql, &CanonicalType::Text),
            "TEXT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::PostgreSql, &CanonicalType::NText),
            "TEXT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::PostgreSql, &CanonicalType::NChar(10)),
            "CHAR(10)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::PostgreSql, &CanonicalType::NVarchar(50)),
            "VARCHAR(50)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::PostgreSql, &CanonicalType::Binary(16)),
            "BYTEA"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::PostgreSql, &CanonicalType::Varbinary(256)),
            "BYTEA"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::PostgreSql, &CanonicalType::Blob),
            "BYTEA"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::PostgreSql, &CanonicalType::Date),
            "DATE"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::PostgreSql, &CanonicalType::Time),
            "TIME"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::PostgreSql, &CanonicalType::DateTime),
            "TIMESTAMP"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::PostgreSql, &CanonicalType::Timestamp),
            "TIMESTAMPTZ"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::PostgreSql, &CanonicalType::Uuid),
            "UUID"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::PostgreSql, &CanonicalType::Json),
            "JSONB"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::PostgreSql, &CanonicalType::Xml),
            "XML"
        );
        assert_eq!(
            from_canonical(
                &DatabaseEngine::PostgreSql,
                &CanonicalType::Array(Box::new(CanonicalType::Int))
            ),
            "INTEGER[]"
        );
    }

    #[test]
    fn test_from_canonical_mysql_all() {
        assert_eq!(
            from_canonical(&DatabaseEngine::MySql, &CanonicalType::Boolean),
            "TINYINT(1)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MySql, &CanonicalType::TinyInt),
            "TINYINT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MySql, &CanonicalType::SmallInt),
            "SMALLINT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MySql, &CanonicalType::Int),
            "INT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MySql, &CanonicalType::BigInt),
            "BIGINT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MySql, &CanonicalType::Float),
            "FLOAT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MySql, &CanonicalType::Double),
            "DOUBLE"
        );
        assert_eq!(
            from_canonical(
                &DatabaseEngine::MySql,
                &CanonicalType::Decimal {
                    precision: 10,
                    scale: 2
                }
            ),
            "DECIMAL(10,2)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MySql, &CanonicalType::Char(10)),
            "CHAR(10)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MySql, &CanonicalType::Varchar(255)),
            "VARCHAR(255)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MySql, &CanonicalType::Text),
            "LONGTEXT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MySql, &CanonicalType::NText),
            "LONGTEXT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MySql, &CanonicalType::NChar(10)),
            "CHAR(10)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MySql, &CanonicalType::NVarchar(50)),
            "VARCHAR(50)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MySql, &CanonicalType::Binary(16)),
            "BINARY(16)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MySql, &CanonicalType::Varbinary(256)),
            "VARBINARY(256)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MySql, &CanonicalType::Blob),
            "LONGBLOB"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MySql, &CanonicalType::Date),
            "DATE"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MySql, &CanonicalType::Time),
            "TIME"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MySql, &CanonicalType::DateTime),
            "DATETIME"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MySql, &CanonicalType::Timestamp),
            "TIMESTAMP"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MySql, &CanonicalType::Uuid),
            "CHAR(36)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MySql, &CanonicalType::Json),
            "JSON"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MySql, &CanonicalType::Xml),
            "LONGTEXT"
        );
        assert_eq!(
            from_canonical(
                &DatabaseEngine::MySql,
                &CanonicalType::Array(Box::new(CanonicalType::Int))
            ),
            "JSON"
        );
    }

    #[test]
    fn test_from_canonical_sqlite_all() {
        assert_eq!(
            from_canonical(&DatabaseEngine::Sqlite, &CanonicalType::Boolean),
            "INTEGER"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Sqlite, &CanonicalType::TinyInt),
            "INTEGER"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Sqlite, &CanonicalType::SmallInt),
            "INTEGER"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Sqlite, &CanonicalType::Int),
            "INTEGER"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Sqlite, &CanonicalType::BigInt),
            "INTEGER"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Sqlite, &CanonicalType::Float),
            "REAL"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Sqlite, &CanonicalType::Double),
            "REAL"
        );
        assert_eq!(
            from_canonical(
                &DatabaseEngine::Sqlite,
                &CanonicalType::Decimal {
                    precision: 10,
                    scale: 2
                }
            ),
            "REAL"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Sqlite, &CanonicalType::Char(10)),
            "TEXT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Sqlite, &CanonicalType::Varchar(255)),
            "TEXT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Sqlite, &CanonicalType::Text),
            "TEXT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Sqlite, &CanonicalType::NText),
            "TEXT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Sqlite, &CanonicalType::Binary(16)),
            "BLOB"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Sqlite, &CanonicalType::Blob),
            "BLOB"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Sqlite, &CanonicalType::Date),
            "TEXT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Sqlite, &CanonicalType::DateTime),
            "TEXT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Sqlite, &CanonicalType::Uuid),
            "TEXT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Sqlite, &CanonicalType::Json),
            "TEXT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Sqlite, &CanonicalType::Xml),
            "TEXT"
        );
        assert_eq!(
            from_canonical(
                &DatabaseEngine::Sqlite,
                &CanonicalType::Array(Box::new(CanonicalType::Int))
            ),
            "TEXT"
        );
    }

    #[test]
    fn test_from_canonical_oracle_all() {
        assert_eq!(
            from_canonical(&DatabaseEngine::Oracle, &CanonicalType::Boolean),
            "NUMBER(1)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Oracle, &CanonicalType::TinyInt),
            "NUMBER(3)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Oracle, &CanonicalType::SmallInt),
            "NUMBER(5)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Oracle, &CanonicalType::Int),
            "NUMBER(10)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Oracle, &CanonicalType::BigInt),
            "NUMBER(19)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Oracle, &CanonicalType::Float),
            "BINARY_FLOAT"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Oracle, &CanonicalType::Double),
            "BINARY_DOUBLE"
        );
        assert_eq!(
            from_canonical(
                &DatabaseEngine::Oracle,
                &CanonicalType::Decimal {
                    precision: 10,
                    scale: 2
                }
            ),
            "NUMBER(10,2)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Oracle, &CanonicalType::Char(10)),
            "CHAR(10)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Oracle, &CanonicalType::Varchar(100)),
            "VARCHAR2(100)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Oracle, &CanonicalType::Text),
            "CLOB"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Oracle, &CanonicalType::NChar(10)),
            "NCHAR(10)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Oracle, &CanonicalType::NVarchar(50)),
            "NVARCHAR2(50)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Oracle, &CanonicalType::NText),
            "NCLOB"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Oracle, &CanonicalType::Binary(16)),
            "RAW(16)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Oracle, &CanonicalType::Varbinary(32)),
            "RAW(32)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Oracle, &CanonicalType::Blob),
            "BLOB"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Oracle, &CanonicalType::Date),
            "DATE"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Oracle, &CanonicalType::Time),
            "TIMESTAMP"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Oracle, &CanonicalType::DateTime),
            "TIMESTAMP"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Oracle, &CanonicalType::Timestamp),
            "TIMESTAMP WITH TIME ZONE"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Oracle, &CanonicalType::Uuid),
            "RAW(16)"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Oracle, &CanonicalType::Json),
            "CLOB"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::Oracle, &CanonicalType::Xml),
            "XMLTYPE"
        );
        assert_eq!(
            from_canonical(
                &DatabaseEngine::Oracle,
                &CanonicalType::Array(Box::new(CanonicalType::Int))
            ),
            "CLOB"
        );
    }

    #[test]
    fn test_from_canonical_mongodb_all() {
        assert_eq!(
            from_canonical(&DatabaseEngine::MongoDb, &CanonicalType::Boolean),
            "bool"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MongoDb, &CanonicalType::TinyInt),
            "int"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MongoDb, &CanonicalType::Int),
            "int"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MongoDb, &CanonicalType::BigInt),
            "long"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MongoDb, &CanonicalType::Double),
            "double"
        );
        assert_eq!(
            from_canonical(
                &DatabaseEngine::MongoDb,
                &CanonicalType::Decimal {
                    precision: 10,
                    scale: 2
                }
            ),
            "decimal128"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MongoDb, &CanonicalType::Text),
            "string"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MongoDb, &CanonicalType::Varchar(255)),
            "string"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MongoDb, &CanonicalType::Blob),
            "binData"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MongoDb, &CanonicalType::DateTime),
            "date"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MongoDb, &CanonicalType::Uuid),
            "string"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MongoDb, &CanonicalType::Json),
            "object"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::MongoDb, &CanonicalType::Xml),
            "string"
        );
        assert_eq!(
            from_canonical(
                &DatabaseEngine::MongoDb,
                &CanonicalType::Array(Box::new(CanonicalType::Int))
            ),
            "array"
        );
    }

    #[test]
    fn test_from_canonical_cosmosdb_all() {
        assert_eq!(
            from_canonical(&DatabaseEngine::CosmosDb, &CanonicalType::Boolean),
            "boolean"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::CosmosDb, &CanonicalType::Int),
            "number"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::CosmosDb, &CanonicalType::BigInt),
            "number"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::CosmosDb, &CanonicalType::Double),
            "number"
        );
        assert_eq!(
            from_canonical(
                &DatabaseEngine::CosmosDb,
                &CanonicalType::Decimal {
                    precision: 10,
                    scale: 2
                }
            ),
            "number"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::CosmosDb, &CanonicalType::Text),
            "string"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::CosmosDb, &CanonicalType::Varchar(255)),
            "string"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::CosmosDb, &CanonicalType::Blob),
            "string"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::CosmosDb, &CanonicalType::DateTime),
            "string"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::CosmosDb, &CanonicalType::Uuid),
            "string"
        );
        assert_eq!(
            from_canonical(&DatabaseEngine::CosmosDb, &CanonicalType::Json),
            "object"
        );
        assert_eq!(
            from_canonical(
                &DatabaseEngine::CosmosDb,
                &CanonicalType::Array(Box::new(CanonicalType::Int))
            ),
            "array"
        );
    }

    // -----------------------------------------------------------------------
    // Cross-engine map_type tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_sqlserver_int_to_postgres() {
        assert_eq!(
            map_type(&DatabaseEngine::SqlServer, &DatabaseEngine::PostgreSql, "int"),
            "INTEGER"
        );
    }

    #[test]
    fn test_postgres_boolean_to_sqlserver() {
        assert_eq!(
            map_type(
                &DatabaseEngine::PostgreSql,
                &DatabaseEngine::SqlServer,
                "boolean"
            ),
            "BIT"
        );
    }

    #[test]
    fn test_mysql_bigint_to_sqlite() {
        assert_eq!(
            map_type(&DatabaseEngine::MySql, &DatabaseEngine::Sqlite, "bigint"),
            "INTEGER"
        );
    }

    #[test]
    fn test_sqlserver_nvarchar_max_to_postgres() {
        assert_eq!(
            map_type(
                &DatabaseEngine::SqlServer,
                &DatabaseEngine::PostgreSql,
                "NVARCHAR(MAX)"
            ),
            "TEXT"
        );
    }

    #[test]
    fn test_postgres_jsonb_to_mysql() {
        assert_eq!(
            map_type(
                &DatabaseEngine::PostgreSql,
                &DatabaseEngine::MySql,
                "jsonb"
            ),
            "JSON"
        );
    }

    #[test]
    fn test_mongodb_string_to_sqlserver() {
        assert_eq!(
            map_type(
                &DatabaseEngine::MongoDb,
                &DatabaseEngine::SqlServer,
                "string"
            ),
            "VARCHAR(MAX)"
        );
    }

    #[test]
    fn test_sqlserver_money_to_postgres() {
        assert_eq!(
            map_type(
                &DatabaseEngine::SqlServer,
                &DatabaseEngine::PostgreSql,
                "MONEY"
            ),
            "NUMERIC(19,4)"
        );
    }

    #[test]
    fn test_postgres_serial_to_mysql() {
        assert_eq!(
            map_type(
                &DatabaseEngine::PostgreSql,
                &DatabaseEngine::MySql,
                "SERIAL"
            ),
            "INT"
        );
    }

    #[test]
    fn test_mysql_enum_to_postgres() {
        assert_eq!(
            map_type(
                &DatabaseEngine::MySql,
                &DatabaseEngine::PostgreSql,
                "ENUM"
            ),
            "VARCHAR(255)"
        );
    }

    #[test]
    fn test_oracle_varchar2_to_mysql() {
        assert_eq!(
            map_type(
                &DatabaseEngine::Oracle,
                &DatabaseEngine::MySql,
                "VARCHAR2(100)"
            ),
            "VARCHAR(100)"
        );
    }

    #[test]
    fn test_sqlserver_uniqueidentifier_to_postgres() {
        assert_eq!(
            map_type(
                &DatabaseEngine::SqlServer,
                &DatabaseEngine::PostgreSql,
                "UNIQUEIDENTIFIER"
            ),
            "UUID"
        );
    }

    #[test]
    fn test_postgres_uuid_to_mysql() {
        assert_eq!(
            map_type(
                &DatabaseEngine::PostgreSql,
                &DatabaseEngine::MySql,
                "UUID"
            ),
            "CHAR(36)"
        );
    }

    #[test]
    fn test_mysql_json_to_sqlserver() {
        assert_eq!(
            map_type(
                &DatabaseEngine::MySql,
                &DatabaseEngine::SqlServer,
                "JSON"
            ),
            "NVARCHAR(MAX)"
        );
    }

    #[test]
    fn test_oracle_number_to_postgres() {
        assert_eq!(
            map_type(
                &DatabaseEngine::Oracle,
                &DatabaseEngine::PostgreSql,
                "NUMBER(10,2)"
            ),
            "NUMERIC(10,2)"
        );
    }

    #[test]
    fn test_sqlserver_decimal_to_oracle() {
        assert_eq!(
            map_type(
                &DatabaseEngine::SqlServer,
                &DatabaseEngine::Oracle,
                "DECIMAL(18,4)"
            ),
            "NUMBER(18,4)"
        );
    }

    #[test]
    fn test_cosmosdb_number_to_sqlserver() {
        assert_eq!(
            map_type(
                &DatabaseEngine::CosmosDb,
                &DatabaseEngine::SqlServer,
                "number"
            ),
            "FLOAT"
        );
    }

    #[test]
    fn test_mysql_year_to_postgres() {
        assert_eq!(
            map_type(
                &DatabaseEngine::MySql,
                &DatabaseEngine::PostgreSql,
                "YEAR"
            ),
            "SMALLINT"
        );
    }

    #[test]
    fn test_postgres_bytea_to_sqlserver() {
        assert_eq!(
            map_type(
                &DatabaseEngine::PostgreSql,
                &DatabaseEngine::SqlServer,
                "BYTEA"
            ),
            "VARBINARY(MAX)"
        );
    }

    #[test]
    fn test_sqlserver_xml_to_oracle() {
        assert_eq!(
            map_type(
                &DatabaseEngine::SqlServer,
                &DatabaseEngine::Oracle,
                "XML"
            ),
            "XMLTYPE"
        );
    }

    #[test]
    fn test_oracle_clob_to_mysql() {
        assert_eq!(
            map_type(&DatabaseEngine::Oracle, &DatabaseEngine::MySql, "CLOB"),
            "LONGTEXT"
        );
    }

    // -----------------------------------------------------------------------
    // Edge case tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_unknown_type() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "FOOBAR"),
            CanonicalType::Unknown("foobar".into())
        );
    }

    #[test]
    fn test_unknown_type_maps_as_is() {
        let result = map_type(
            &DatabaseEngine::SqlServer,
            &DatabaseEngine::PostgreSql,
            "FOOBAR",
        );
        assert_eq!(result, "FOOBAR");
    }

    #[test]
    fn test_empty_string_sqlserver() {
        // Empty string should be unknown
        let result = to_canonical(&DatabaseEngine::SqlServer, "");
        assert!(matches!(result, CanonicalType::Unknown(_)));
    }

    #[test]
    fn test_very_long_type_name() {
        let long_name = "A".repeat(1000);
        let result = to_canonical(&DatabaseEngine::SqlServer, &long_name);
        if let CanonicalType::Unknown(s) = result {
            assert_eq!(s.len(), 1000);
        } else {
            panic!("Expected Unknown variant");
        }
    }

    #[test]
    fn test_case_insensitivity() {
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "Bit"),
            CanonicalType::Boolean
        );
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "BIT"),
            CanonicalType::Boolean
        );
        assert_eq!(
            to_canonical(&DatabaseEngine::SqlServer, "bit"),
            CanonicalType::Boolean
        );
    }

    // -----------------------------------------------------------------------
    // Roundtrip tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_roundtrip_sqlserver_int() {
        let canonical = to_canonical(&DatabaseEngine::SqlServer, "INT");
        let native = from_canonical(&DatabaseEngine::SqlServer, &canonical);
        assert_eq!(native, "INT");
    }

    #[test]
    fn test_roundtrip_postgres_integer() {
        let canonical = to_canonical(&DatabaseEngine::PostgreSql, "INTEGER");
        let native = from_canonical(&DatabaseEngine::PostgreSql, &canonical);
        assert_eq!(native, "INTEGER");
    }

    #[test]
    fn test_roundtrip_mysql_varchar() {
        let canonical = to_canonical(&DatabaseEngine::MySql, "VARCHAR(255)");
        let native = from_canonical(&DatabaseEngine::MySql, &canonical);
        assert_eq!(native, "VARCHAR(255)");
    }

    #[test]
    fn test_roundtrip_sqlserver_decimal() {
        let canonical = to_canonical(&DatabaseEngine::SqlServer, "DECIMAL(18,2)");
        let native = from_canonical(&DatabaseEngine::SqlServer, &canonical);
        assert_eq!(native, "DECIMAL(18,2)");
    }

    #[test]
    fn test_roundtrip_postgres_boolean() {
        let canonical = to_canonical(&DatabaseEngine::PostgreSql, "BOOLEAN");
        let native = from_canonical(&DatabaseEngine::PostgreSql, &canonical);
        assert_eq!(native, "BOOLEAN");
    }

    #[test]
    fn test_roundtrip_oracle_number_10_2() {
        let canonical = to_canonical(&DatabaseEngine::Oracle, "NUMBER(10,2)");
        let native = from_canonical(&DatabaseEngine::Oracle, &canonical);
        assert_eq!(native, "NUMBER(10,2)");
    }

    #[test]
    fn test_roundtrip_mongodb_bool() {
        let canonical = to_canonical(&DatabaseEngine::MongoDb, "bool");
        let native = from_canonical(&DatabaseEngine::MongoDb, &canonical);
        assert_eq!(native, "bool");
    }

    #[test]
    fn test_roundtrip_cosmosdb_boolean() {
        let canonical = to_canonical(&DatabaseEngine::CosmosDb, "boolean");
        let native = from_canonical(&DatabaseEngine::CosmosDb, &canonical);
        assert_eq!(native, "boolean");
    }

    #[test]
    fn test_roundtrip_canonical_varchar() {
        let canonical = CanonicalType::Varchar(255);
        let sqlserver = from_canonical(&DatabaseEngine::SqlServer, &canonical);
        assert_eq!(sqlserver, "VARCHAR(255)");
    }

    // -----------------------------------------------------------------------
    // TypeMappingResult / map_type_with_warnings tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_warnings_decimal_to_float_via_sqlite() {
        // Mapping DECIMAL(38,10) through SQLite should lose precision since SQLite maps to REAL
        let result = map_type_with_warnings(
            &DatabaseEngine::SqlServer,
            &DatabaseEngine::Sqlite,
            "DECIMAL(38,10)",
        );
        assert_eq!(result.target_type, "REAL");
        assert!(result.is_lossy);
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn test_warnings_datetime_to_text_via_sqlite() {
        let result = map_type_with_warnings(
            &DatabaseEngine::SqlServer,
            &DatabaseEngine::Sqlite,
            "DATETIME",
        );
        assert_eq!(result.target_type, "TEXT");
        assert!(result.is_lossy);
        assert!(result.warnings.iter().any(|w| w.contains("TEXT")));
    }

    #[test]
    fn test_warnings_nvarchar_to_varchar_postgres() {
        let result = map_type_with_warnings(
            &DatabaseEngine::SqlServer,
            &DatabaseEngine::PostgreSql,
            "NVARCHAR(50)",
        );
        assert_eq!(result.target_type, "VARCHAR(50)");
        assert!(result.is_lossy);
        assert!(result.warnings.iter().any(|w| w.contains("Unicode")));
    }

    #[test]
    fn test_warnings_json_to_text_sqlserver() {
        let result = map_type_with_warnings(
            &DatabaseEngine::PostgreSql,
            &DatabaseEngine::SqlServer,
            "JSONB",
        );
        assert_eq!(result.target_type, "NVARCHAR(MAX)");
        assert!(!result.warnings.is_empty());
        assert!(result.warnings.iter().any(|w| w.contains("JSON")));
    }

    #[test]
    fn test_warnings_uuid_to_char_mysql() {
        let result = map_type_with_warnings(
            &DatabaseEngine::PostgreSql,
            &DatabaseEngine::MySql,
            "UUID",
        );
        assert_eq!(result.target_type, "CHAR(36)");
        assert!(!result.warnings.is_empty());
        assert!(result.warnings.iter().any(|w| w.contains("UUID")));
    }

    #[test]
    fn test_warnings_unknown_type() {
        let result = map_type_with_warnings(
            &DatabaseEngine::SqlServer,
            &DatabaseEngine::PostgreSql,
            "FOOBAR",
        );
        assert!(!result.warnings.is_empty());
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("Unknown source type")));
    }

    #[test]
    fn test_no_warnings_simple_int() {
        let result = map_type_with_warnings(
            &DatabaseEngine::SqlServer,
            &DatabaseEngine::PostgreSql,
            "INT",
        );
        assert_eq!(result.target_type, "INTEGER");
        assert!(!result.is_lossy);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_no_warnings_same_engine() {
        let result = map_type_with_warnings(
            &DatabaseEngine::SqlServer,
            &DatabaseEngine::SqlServer,
            "INT",
        );
        assert_eq!(result.target_type, "INT");
        assert!(!result.is_lossy);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_warnings_uuid_to_sqlite_text() {
        let result = map_type_with_warnings(
            &DatabaseEngine::PostgreSql,
            &DatabaseEngine::Sqlite,
            "UUID",
        );
        assert_eq!(result.target_type, "TEXT");
        assert!(!result.warnings.is_empty());
    }
}
