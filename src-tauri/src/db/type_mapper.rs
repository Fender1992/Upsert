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

/// Map a native type string from one engine to the canonical type
pub fn to_canonical(engine: &DatabaseEngine, native_type: &str) -> CanonicalType {
    let lower = native_type.to_lowercase();
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

fn sqlserver_to_canonical(native: &str) -> CanonicalType {
    match native {
        "bit" => CanonicalType::Boolean,
        "tinyint" => CanonicalType::TinyInt,
        "smallint" => CanonicalType::SmallInt,
        "int" => CanonicalType::Int,
        "bigint" => CanonicalType::BigInt,
        "real" => CanonicalType::Float,
        "float" => CanonicalType::Double,
        "date" => CanonicalType::Date,
        "time" => CanonicalType::Time,
        "datetime" | "datetime2" | "smalldatetime" => CanonicalType::DateTime,
        "datetimeoffset" => CanonicalType::Timestamp,
        "uniqueidentifier" => CanonicalType::Uuid,
        "xml" => CanonicalType::Xml,
        "text" => CanonicalType::Text,
        "ntext" => CanonicalType::NText,
        "image" | "varbinary(max)" => CanonicalType::Blob,
        _ => CanonicalType::Unknown(native.to_string()),
    }
}

fn postgres_to_canonical(native: &str) -> CanonicalType {
    match native {
        "boolean" | "bool" => CanonicalType::Boolean,
        "smallint" | "int2" => CanonicalType::SmallInt,
        "integer" | "int4" | "int" => CanonicalType::Int,
        "bigint" | "int8" => CanonicalType::BigInt,
        "real" | "float4" => CanonicalType::Float,
        "double precision" | "float8" => CanonicalType::Double,
        "date" => CanonicalType::Date,
        "time" | "time without time zone" => CanonicalType::Time,
        "timestamp" | "timestamp without time zone" => CanonicalType::DateTime,
        "timestamp with time zone" | "timestamptz" => CanonicalType::Timestamp,
        "uuid" => CanonicalType::Uuid,
        "json" | "jsonb" => CanonicalType::Json,
        "xml" => CanonicalType::Xml,
        "text" => CanonicalType::Text,
        "bytea" => CanonicalType::Blob,
        _ => CanonicalType::Unknown(native.to_string()),
    }
}

fn mysql_to_canonical(native: &str) -> CanonicalType {
    match native {
        "tinyint(1)" | "boolean" | "bool" => CanonicalType::Boolean,
        "tinyint" => CanonicalType::TinyInt,
        "smallint" => CanonicalType::SmallInt,
        "int" | "integer" | "mediumint" => CanonicalType::Int,
        "bigint" => CanonicalType::BigInt,
        "float" => CanonicalType::Float,
        "double" => CanonicalType::Double,
        "date" => CanonicalType::Date,
        "time" => CanonicalType::Time,
        "datetime" => CanonicalType::DateTime,
        "timestamp" => CanonicalType::Timestamp,
        "json" => CanonicalType::Json,
        "text" | "mediumtext" | "longtext" => CanonicalType::Text,
        "blob" | "mediumblob" | "longblob" => CanonicalType::Blob,
        _ => CanonicalType::Unknown(native.to_string()),
    }
}

fn sqlite_to_canonical(native: &str) -> CanonicalType {
    match native {
        "boolean" => CanonicalType::Boolean,
        "integer" | "int" => CanonicalType::BigInt,
        "real" => CanonicalType::Double,
        "text" => CanonicalType::Text,
        "blob" => CanonicalType::Blob,
        _ => CanonicalType::Unknown(native.to_string()),
    }
}

fn oracle_to_canonical(native: &str) -> CanonicalType {
    match native {
        "number(1)" => CanonicalType::Boolean,
        "number" | "number(10)" => CanonicalType::Int,
        "number(19)" => CanonicalType::BigInt,
        "binary_float" => CanonicalType::Float,
        "binary_double" => CanonicalType::Double,
        "date" => CanonicalType::DateTime,
        "timestamp" => CanonicalType::Timestamp,
        "clob" => CanonicalType::Text,
        "nclob" => CanonicalType::NText,
        "blob" => CanonicalType::Blob,
        "raw(16)" => CanonicalType::Uuid,
        _ => CanonicalType::Unknown(native.to_string()),
    }
}

fn mongodb_to_canonical(native: &str) -> CanonicalType {
    match native {
        "bool" => CanonicalType::Boolean,
        "int" | "int32" => CanonicalType::Int,
        "long" | "int64" => CanonicalType::BigInt,
        "double" => CanonicalType::Double,
        "decimal128" => CanonicalType::Decimal { precision: 34, scale: 6 },
        "string" => CanonicalType::Text,
        "date" => CanonicalType::DateTime,
        "objectid" => CanonicalType::Varchar(24),
        "bindata" => CanonicalType::Blob,
        "object" | "array" => CanonicalType::Json,
        _ => CanonicalType::Unknown(native.to_string()),
    }
}

fn cosmosdb_to_canonical(native: &str) -> CanonicalType {
    match native {
        "boolean" => CanonicalType::Boolean,
        "number" => CanonicalType::Double,
        "string" => CanonicalType::Text,
        "array" | "object" => CanonicalType::Json,
        "null" => CanonicalType::Unknown("null".to_string()),
        _ => CanonicalType::Unknown(native.to_string()),
    }
}

fn canonical_to_sqlserver(canonical: &CanonicalType) -> String {
    match canonical {
        CanonicalType::Boolean => "BIT".to_string(),
        CanonicalType::TinyInt => "TINYINT".to_string(),
        CanonicalType::SmallInt => "SMALLINT".to_string(),
        CanonicalType::Int => "INT".to_string(),
        CanonicalType::BigInt => "BIGINT".to_string(),
        CanonicalType::Float => "REAL".to_string(),
        CanonicalType::Double => "FLOAT".to_string(),
        CanonicalType::Decimal { precision, scale } => format!("DECIMAL({},{})", precision, scale),
        CanonicalType::Char(n) => format!("CHAR({})", n),
        CanonicalType::Varchar(n) => format!("VARCHAR({})", n),
        CanonicalType::Text => "VARCHAR(MAX)".to_string(),
        CanonicalType::NChar(n) => format!("NCHAR({})", n),
        CanonicalType::NVarchar(n) => format!("NVARCHAR({})", n),
        CanonicalType::NText => "NVARCHAR(MAX)".to_string(),
        CanonicalType::Binary(n) => format!("BINARY({})", n),
        CanonicalType::Varbinary(n) => format!("VARBINARY({})", n),
        CanonicalType::Blob => "VARBINARY(MAX)".to_string(),
        CanonicalType::Date => "DATE".to_string(),
        CanonicalType::Time => "TIME".to_string(),
        CanonicalType::DateTime => "DATETIME2".to_string(),
        CanonicalType::Timestamp => "DATETIMEOFFSET".to_string(),
        CanonicalType::Uuid => "UNIQUEIDENTIFIER".to_string(),
        CanonicalType::Json => "NVARCHAR(MAX)".to_string(),
        CanonicalType::Xml => "XML".to_string(),
        CanonicalType::Array(_) => "NVARCHAR(MAX)".to_string(),
        CanonicalType::Unknown(s) => s.to_uppercase(),
    }
}

fn canonical_to_postgres(canonical: &CanonicalType) -> String {
    match canonical {
        CanonicalType::Boolean => "BOOLEAN".to_string(),
        CanonicalType::TinyInt | CanonicalType::SmallInt => "SMALLINT".to_string(),
        CanonicalType::Int => "INTEGER".to_string(),
        CanonicalType::BigInt => "BIGINT".to_string(),
        CanonicalType::Float => "REAL".to_string(),
        CanonicalType::Double => "DOUBLE PRECISION".to_string(),
        CanonicalType::Decimal { precision, scale } => format!("NUMERIC({},{})", precision, scale),
        CanonicalType::Char(n) => format!("CHAR({})", n),
        CanonicalType::Varchar(n) => format!("VARCHAR({})", n),
        CanonicalType::Text | CanonicalType::NText => "TEXT".to_string(),
        CanonicalType::NChar(n) => format!("CHAR({})", n),
        CanonicalType::NVarchar(n) => format!("VARCHAR({})", n),
        CanonicalType::Binary(n) | CanonicalType::Varbinary(n) => "BYTEA".to_string(),
        CanonicalType::Blob => "BYTEA".to_string(),
        CanonicalType::Date => "DATE".to_string(),
        CanonicalType::Time => "TIME".to_string(),
        CanonicalType::DateTime => "TIMESTAMP".to_string(),
        CanonicalType::Timestamp => "TIMESTAMPTZ".to_string(),
        CanonicalType::Uuid => "UUID".to_string(),
        CanonicalType::Json => "JSONB".to_string(),
        CanonicalType::Xml => "XML".to_string(),
        CanonicalType::Array(inner) => format!("{}[]", canonical_to_postgres(inner)),
        CanonicalType::Unknown(s) => s.to_uppercase(),
    }
}

fn canonical_to_mysql(canonical: &CanonicalType) -> String {
    match canonical {
        CanonicalType::Boolean => "TINYINT(1)".to_string(),
        CanonicalType::TinyInt => "TINYINT".to_string(),
        CanonicalType::SmallInt => "SMALLINT".to_string(),
        CanonicalType::Int => "INT".to_string(),
        CanonicalType::BigInt => "BIGINT".to_string(),
        CanonicalType::Float => "FLOAT".to_string(),
        CanonicalType::Double => "DOUBLE".to_string(),
        CanonicalType::Decimal { precision, scale } => format!("DECIMAL({},{})", precision, scale),
        CanonicalType::Char(n) => format!("CHAR({})", n),
        CanonicalType::Varchar(n) => format!("VARCHAR({})", n),
        CanonicalType::Text | CanonicalType::NText => "LONGTEXT".to_string(),
        CanonicalType::NChar(n) => format!("CHAR({})", n),
        CanonicalType::NVarchar(n) => format!("VARCHAR({})", n),
        CanonicalType::Binary(n) => format!("BINARY({})", n),
        CanonicalType::Varbinary(n) => format!("VARBINARY({})", n),
        CanonicalType::Blob => "LONGBLOB".to_string(),
        CanonicalType::Date => "DATE".to_string(),
        CanonicalType::Time => "TIME".to_string(),
        CanonicalType::DateTime => "DATETIME".to_string(),
        CanonicalType::Timestamp => "TIMESTAMP".to_string(),
        CanonicalType::Uuid => "CHAR(36)".to_string(),
        CanonicalType::Json => "JSON".to_string(),
        CanonicalType::Xml => "LONGTEXT".to_string(),
        CanonicalType::Array(_) => "JSON".to_string(),
        CanonicalType::Unknown(s) => s.to_uppercase(),
    }
}

fn canonical_to_sqlite(canonical: &CanonicalType) -> String {
    match canonical {
        CanonicalType::Boolean => "INTEGER".to_string(),
        CanonicalType::TinyInt | CanonicalType::SmallInt | CanonicalType::Int | CanonicalType::BigInt => "INTEGER".to_string(),
        CanonicalType::Float | CanonicalType::Double => "REAL".to_string(),
        CanonicalType::Decimal { .. } => "REAL".to_string(),
        CanonicalType::Char(_) | CanonicalType::Varchar(_) | CanonicalType::Text => "TEXT".to_string(),
        CanonicalType::NChar(_) | CanonicalType::NVarchar(_) | CanonicalType::NText => "TEXT".to_string(),
        CanonicalType::Binary(_) | CanonicalType::Varbinary(_) | CanonicalType::Blob => "BLOB".to_string(),
        CanonicalType::Date | CanonicalType::Time | CanonicalType::DateTime | CanonicalType::Timestamp => "TEXT".to_string(),
        CanonicalType::Uuid => "TEXT".to_string(),
        CanonicalType::Json => "TEXT".to_string(),
        CanonicalType::Xml => "TEXT".to_string(),
        CanonicalType::Array(_) => "TEXT".to_string(),
        CanonicalType::Unknown(s) => s.to_uppercase(),
    }
}

fn canonical_to_oracle(canonical: &CanonicalType) -> String {
    match canonical {
        CanonicalType::Boolean => "NUMBER(1)".to_string(),
        CanonicalType::TinyInt => "NUMBER(3)".to_string(),
        CanonicalType::SmallInt => "NUMBER(5)".to_string(),
        CanonicalType::Int => "NUMBER(10)".to_string(),
        CanonicalType::BigInt => "NUMBER(19)".to_string(),
        CanonicalType::Float => "BINARY_FLOAT".to_string(),
        CanonicalType::Double => "BINARY_DOUBLE".to_string(),
        CanonicalType::Decimal { precision, scale } => format!("NUMBER({},{})", precision, scale),
        CanonicalType::Char(n) => format!("CHAR({})", n),
        CanonicalType::Varchar(n) => format!("VARCHAR2({})", n),
        CanonicalType::Text => "CLOB".to_string(),
        CanonicalType::NChar(n) => format!("NCHAR({})", n),
        CanonicalType::NVarchar(n) => format!("NVARCHAR2({})", n),
        CanonicalType::NText => "NCLOB".to_string(),
        CanonicalType::Binary(n) => format!("RAW({})", n),
        CanonicalType::Varbinary(n) => format!("RAW({})", n),
        CanonicalType::Blob => "BLOB".to_string(),
        CanonicalType::Date => "DATE".to_string(),
        CanonicalType::Time => "TIMESTAMP".to_string(),
        CanonicalType::DateTime => "TIMESTAMP".to_string(),
        CanonicalType::Timestamp => "TIMESTAMP WITH TIME ZONE".to_string(),
        CanonicalType::Uuid => "RAW(16)".to_string(),
        CanonicalType::Json => "CLOB".to_string(),
        CanonicalType::Xml => "XMLTYPE".to_string(),
        CanonicalType::Array(_) => "CLOB".to_string(),
        CanonicalType::Unknown(s) => s.to_uppercase(),
    }
}

fn canonical_to_mongodb(canonical: &CanonicalType) -> String {
    match canonical {
        CanonicalType::Boolean => "bool".to_string(),
        CanonicalType::TinyInt | CanonicalType::SmallInt | CanonicalType::Int => "int".to_string(),
        CanonicalType::BigInt => "long".to_string(),
        CanonicalType::Float | CanonicalType::Double => "double".to_string(),
        CanonicalType::Decimal { .. } => "decimal128".to_string(),
        CanonicalType::Char(_) | CanonicalType::Varchar(_) | CanonicalType::Text => "string".to_string(),
        CanonicalType::NChar(_) | CanonicalType::NVarchar(_) | CanonicalType::NText => "string".to_string(),
        CanonicalType::Binary(_) | CanonicalType::Varbinary(_) | CanonicalType::Blob => "binData".to_string(),
        CanonicalType::Date | CanonicalType::Time | CanonicalType::DateTime | CanonicalType::Timestamp => "date".to_string(),
        CanonicalType::Uuid => "string".to_string(),
        CanonicalType::Json => "object".to_string(),
        CanonicalType::Xml => "string".to_string(),
        CanonicalType::Array(_) => "array".to_string(),
        CanonicalType::Unknown(s) => s.clone(),
    }
}

fn canonical_to_cosmosdb(canonical: &CanonicalType) -> String {
    match canonical {
        CanonicalType::Boolean => "boolean".to_string(),
        CanonicalType::TinyInt | CanonicalType::SmallInt | CanonicalType::Int | CanonicalType::BigInt => "number".to_string(),
        CanonicalType::Float | CanonicalType::Double | CanonicalType::Decimal { .. } => "number".to_string(),
        CanonicalType::Char(_) | CanonicalType::Varchar(_) | CanonicalType::Text => "string".to_string(),
        CanonicalType::NChar(_) | CanonicalType::NVarchar(_) | CanonicalType::NText => "string".to_string(),
        CanonicalType::Binary(_) | CanonicalType::Varbinary(_) | CanonicalType::Blob => "string".to_string(),
        CanonicalType::Date | CanonicalType::Time | CanonicalType::DateTime | CanonicalType::Timestamp => "string".to_string(),
        CanonicalType::Uuid => "string".to_string(),
        CanonicalType::Json => "object".to_string(),
        CanonicalType::Xml => "string".to_string(),
        CanonicalType::Array(_) => "array".to_string(),
        CanonicalType::Unknown(s) => s.clone(),
    }
}

/// Map a native type from one engine directly to another engine's native type
pub fn map_type(source_engine: &DatabaseEngine, target_engine: &DatabaseEngine, native_type: &str) -> String {
    let canonical = to_canonical(source_engine, native_type);
    from_canonical(target_engine, &canonical)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlserver_int_to_postgres() {
        let result = map_type(&DatabaseEngine::SqlServer, &DatabaseEngine::PostgreSql, "int");
        assert_eq!(result, "INTEGER");
    }

    #[test]
    fn test_postgres_boolean_to_sqlserver() {
        let result = map_type(&DatabaseEngine::PostgreSql, &DatabaseEngine::SqlServer, "boolean");
        assert_eq!(result, "BIT");
    }

    #[test]
    fn test_mysql_to_sqlite() {
        let result = map_type(&DatabaseEngine::MySql, &DatabaseEngine::Sqlite, "bigint");
        assert_eq!(result, "INTEGER");
    }

    #[test]
    fn test_roundtrip_canonical() {
        let canonical = CanonicalType::Varchar(255);
        let sqlserver = from_canonical(&DatabaseEngine::SqlServer, &canonical);
        assert_eq!(sqlserver, "VARCHAR(255)");
    }
}
