//! Integration tests against live SQL Server and PostgreSQL databases.
//!
//! Prerequisites:
//!   - SQL Server Express on localhost:1433 with sa/YourPassword123, database UpsertTestSource seeded
//!   - PostgreSQL on localhost:5432 with user postgres, database upsert_test_target seeded

use upsert_lib::db::connectors::{
    postgres::PostgresConnector, sqlserver::SqlServerConnector, ConnectionConfig, DatabaseConnector,
    DatabaseEngine,
};
use upsert_lib::db::schema::ConstraintType;

// ─── helpers ───────────────────────────────────────────────────────────────

fn sqlserver_config() -> ConnectionConfig {
    ConnectionConfig {
        engine: DatabaseEngine::SqlServer,
        host: Some("localhost".to_string()),
        port: Some(1433),
        database: Some("UpsertTestSource".to_string()),
        username: Some("sa".to_string()),
        password: Some("YourPassword123".to_string()),
        ..Default::default()
    }
}

fn postgres_config() -> ConnectionConfig {
    ConnectionConfig {
        engine: DatabaseEngine::PostgreSql,
        host: Some("localhost".to_string()),
        port: Some(5432),
        database: Some("upsert_test_target".to_string()),
        username: Some("postgres".to_string()),
        password: Some("YourPassword123".to_string()),
        ..Default::default()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  SQL SERVER - CONNECTION TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn sqlserver_connect_disconnect() {
    let mut conn = SqlServerConnector::new(sqlserver_config());
    assert!(!conn.is_connected().await);

    conn.connect().await.expect("SQL Server connect failed");
    assert!(conn.is_connected().await);

    conn.disconnect().await.expect("SQL Server disconnect failed");
    assert!(!conn.is_connected().await);
}

#[tokio::test]
async fn sqlserver_connect_wrong_password() {
    let mut cfg = sqlserver_config();
    cfg.password = Some("WrongPassword999".to_string());
    let mut conn = SqlServerConnector::new(cfg);

    let result = conn.connect().await;
    assert!(result.is_err(), "Should fail with wrong credentials");
}

#[tokio::test]
async fn sqlserver_connect_wrong_database() {
    let mut cfg = sqlserver_config();
    cfg.database = Some("NonExistentDb_XYZ_999".to_string());
    let mut conn = SqlServerConnector::new(cfg);

    let result = conn.connect().await;
    // tiberius may connect to master then fail on USE, or fail during connect
    // Either an error or connecting to wrong db is acceptable to detect
    if result.is_ok() {
        // Connected but to a different/default database - verify tables query returns empty
        let tables = conn.get_tables().await;
        // Should either error or return no UpsertTestSource tables
        if let Ok(t) = &tables {
            assert!(
                !t.iter().any(|name| name == "customers"),
                "Should not find UpsertTestSource tables in nonexistent db"
            );
        }
        conn.disconnect().await.ok();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  SQL SERVER - SCHEMA TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn sqlserver_list_tables() {
    let mut conn = SqlServerConnector::new(sqlserver_config());
    conn.connect().await.expect("connect");

    let tables = conn.get_tables().await.expect("get_tables");
    println!("SQL Server tables: {:?}", tables);

    // Should contain all seeded tables
    let expected = vec![
        "audit_log",
        "categories",
        "customers",
        "employees",
        "inventory",
        "order_items",
        "orders",
        "product_tags",
        "products",
        "promotions",
        "reviews",
        "warehouses",
    ];
    for tbl in &expected {
        assert!(
            tables.iter().any(|t| t == *tbl),
            "Missing table: {} in {:?}",
            tbl,
            tables
        );
    }

    conn.disconnect().await.ok();
}

#[tokio::test]
async fn sqlserver_table_info_customers() {
    let mut conn = SqlServerConnector::new(sqlserver_config());
    conn.connect().await.expect("connect");

    let info = conn.get_table_info("customers").await.expect("get_table_info");
    println!("SQL Server customers columns: {:#?}", info.columns);

    assert_eq!(info.table_name, "customers");
    assert_eq!(info.schema_name, "dbo");

    // Verify key columns exist
    let col_names: Vec<&str> = info.columns.iter().map(|c| c.name.as_str()).collect();
    assert!(col_names.contains(&"id"), "Missing 'id' column");
    assert!(col_names.contains(&"email"), "Missing 'email' column");
    assert!(col_names.contains(&"first_name"), "Missing 'first_name'");
    assert!(col_names.contains(&"middle_name"), "Missing 'middle_name' (SQL Server specific)");
    assert!(col_names.contains(&"last_name"), "Missing 'last_name'");
    assert!(col_names.contains(&"credit_limit"), "Missing 'credit_limit' (SQL Server specific)");
    assert!(col_names.contains(&"address_line1"), "Missing 'address_line1'");
    assert!(col_names.contains(&"address_line2"), "Missing 'address_line2'");
    assert!(col_names.contains(&"state"), "Missing 'state'");
    assert!(col_names.contains(&"zip_code"), "Missing 'zip_code'");

    // Verify 'id' is primary key
    let id_col = info.columns.iter().find(|c| c.name == "id").unwrap();
    assert!(id_col.is_primary_key, "'id' should be primary key");
    assert_eq!(id_col.data_type, "int");

    // Verify email column
    let email_col = info.columns.iter().find(|c| c.name == "email").unwrap();
    assert!(!email_col.is_nullable, "email should be NOT NULL");
    assert_eq!(email_col.data_type, "nvarchar");
    assert_eq!(email_col.max_length, Some(255));

    // Verify loyalty_points is INT
    let lp = info.columns.iter().find(|c| c.name == "loyalty_points").unwrap();
    assert_eq!(lp.data_type, "int");

    // Verify credit_limit is money
    let cl = info.columns.iter().find(|c| c.name == "credit_limit").unwrap();
    assert_eq!(cl.data_type, "money");

    // Check constraints - should have PK and FK constraints
    let pk = info.constraints.iter().find(|c| c.constraint_type == ConstraintType::PrimaryKey);
    assert!(pk.is_some(), "Should have primary key constraint");

    // Check indexes
    println!("SQL Server customers indexes: {:?}", info.indexes);
    let idx_names: Vec<&str> = info.indexes.iter().map(|i| i.name.as_str()).collect();
    // Should have ix_customers_email and ix_customers_state
    assert!(
        idx_names.iter().any(|n| n.contains("email")),
        "Missing email index in {:?}",
        idx_names
    );
    assert!(
        idx_names.iter().any(|n| n.contains("state")),
        "Missing state index in {:?}",
        idx_names
    );

    // Verify row count
    assert_eq!(info.row_count, Some(12), "customers should have 12 rows");

    conn.disconnect().await.ok();
}

#[tokio::test]
async fn sqlserver_table_info_products() {
    let mut conn = SqlServerConnector::new(sqlserver_config());
    conn.connect().await.expect("connect");

    let info = conn.get_table_info("products").await.expect("get_table_info");
    println!("SQL Server products columns: {:#?}", info.columns);

    let col_names: Vec<&str> = info.columns.iter().map(|c| c.name.as_str()).collect();
    assert!(col_names.contains(&"sku"), "Missing 'sku'");
    assert!(col_names.contains(&"reorder_point"), "Missing 'reorder_point' (SQL Server specific)");
    assert!(!col_names.contains(&"color"), "'color' should NOT exist (PG only)");
    assert!(!col_names.contains(&"size"), "'size' should NOT exist (PG only)");

    // Verify types
    let sku = info.columns.iter().find(|c| c.name == "sku").unwrap();
    assert_eq!(sku.data_type, "nvarchar");
    assert_eq!(sku.max_length, Some(50));

    let price = info.columns.iter().find(|c| c.name == "price").unwrap();
    assert_eq!(price.data_type, "decimal");
    assert_eq!(price.precision, Some(10));
    assert_eq!(price.scale, Some(2));

    let weight = info.columns.iter().find(|c| c.name == "weight_kg").unwrap();
    assert_eq!(weight.data_type, "decimal");

    // Check constraints - should have CHECK constraints
    let checks: Vec<_> = info
        .constraints
        .iter()
        .filter(|c| c.constraint_type == ConstraintType::Check)
        .collect();
    println!("SQL Server products check constraints: {:?}", checks);

    // Row count: 15 products
    assert_eq!(info.row_count, Some(15), "products should have 15 rows");

    conn.disconnect().await.ok();
}

#[tokio::test]
async fn sqlserver_table_info_orders() {
    let mut conn = SqlServerConnector::new(sqlserver_config());
    conn.connect().await.expect("connect");

    let info = conn.get_table_info("orders").await.expect("get_table_info");

    let col_names: Vec<&str> = info.columns.iter().map(|c| c.name.as_str()).collect();
    assert!(col_names.contains(&"discount_code"), "Missing 'discount_code' (SQL Server specific)");
    assert!(col_names.contains(&"shipping_cost"), "Missing 'shipping_cost' (SQL Server specific)");
    assert!(col_names.contains(&"shipping_address"), "Missing 'shipping_address'");

    // status is nvarchar in SQL Server
    let status = info.columns.iter().find(|c| c.name == "status").unwrap();
    assert_eq!(status.data_type, "nvarchar");

    // Row count: 11 orders
    assert_eq!(info.row_count, Some(11), "orders should have 11 rows");

    // FK to customers
    let fks: Vec<_> = info
        .constraints
        .iter()
        .filter(|c| c.constraint_type == ConstraintType::ForeignKey)
        .collect();
    assert!(!fks.is_empty(), "orders should have FK constraint");
    println!("SQL Server orders FK constraints: {:?}", fks);

    conn.disconnect().await.ok();
}

#[tokio::test]
async fn sqlserver_row_counts() {
    let mut conn = SqlServerConnector::new(sqlserver_config());
    conn.connect().await.expect("connect");

    // Verify exact row counts for all seeded tables
    let expected_counts = vec![
        ("categories", 11),   // 6 top-level + 5 sub-categories
        ("customers", 12),    // 10 regular + 2 Unicode
        ("products", 15),     // 11 shared + 2 SQL-only + 1 mystery + 1 encyclopedia
        ("product_tags", 13),
        ("employees", 5),
        ("orders", 11),
        ("order_items", 17),
        ("reviews", 12),
        ("warehouses", 3),
        ("inventory", 9),
        ("promotions", 5),
        ("audit_log", 3),
    ];

    for (table, expected) in &expected_counts {
        let count = conn
            .get_row_count(table)
            .await
            .unwrap_or_else(|e| panic!("get_row_count({}) failed: {}", table, e));
        assert_eq!(
            count, *expected,
            "Row count mismatch for {}: got {}, expected {}",
            table, count, expected
        );
    }

    conn.disconnect().await.ok();
}

#[tokio::test]
async fn sqlserver_get_table_info_nonexistent() {
    let mut conn = SqlServerConnector::new(sqlserver_config());
    conn.connect().await.expect("connect");

    // Querying a nonexistent table - should return empty columns or error
    let info = conn.get_table_info("nonexistent_table_xyz_999").await;
    // Either an error or a TableInfo with 0 columns is acceptable
    match info {
        Ok(ti) => {
            assert!(
                ti.columns.is_empty(),
                "Nonexistent table should have no columns"
            );
        }
        Err(_) => {
            // Error is also acceptable
        }
    }

    conn.disconnect().await.ok();
}

#[tokio::test]
async fn sqlserver_get_rows() {
    let mut conn = SqlServerConnector::new(sqlserver_config());
    conn.connect().await.expect("connect");

    let rows = conn
        .get_rows("customers", Some(5), None)
        .await
        .expect("get_rows");
    assert_eq!(rows.len(), 5, "Should return 5 rows with LIMIT 5");

    // Verify row structure has expected columns
    let first = &rows[0];
    assert!(first.contains_key("email"), "Row missing 'email'");
    assert!(first.contains_key("first_name"), "Row missing 'first_name'");
    assert!(first.contains_key("loyalty_points"), "Row missing 'loyalty_points'");

    conn.disconnect().await.ok();
}

#[tokio::test]
async fn sqlserver_execute_query() {
    let mut conn = SqlServerConnector::new(sqlserver_config());
    conn.connect().await.expect("connect");

    let rows = conn
        .execute_query("SELECT TOP 3 email, first_name FROM customers ORDER BY email")
        .await
        .expect("execute_query");
    assert_eq!(rows.len(), 3);
    assert!(rows[0].contains_key("email"));
    assert!(rows[0].contains_key("first_name"));

    conn.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  SQL SERVER - SCHEMA DETAILS
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn sqlserver_table_info_sqlserver_only_tables() {
    let mut conn = SqlServerConnector::new(sqlserver_config());
    conn.connect().await.expect("connect");

    // Tables that exist only in SQL Server
    let product_tags = conn.get_table_info("product_tags").await.expect("product_tags");
    assert_eq!(product_tags.row_count, Some(13));
    let pt_cols: Vec<&str> = product_tags.columns.iter().map(|c| c.name.as_str()).collect();
    assert!(pt_cols.contains(&"tag_name"));

    let employees = conn.get_table_info("employees").await.expect("employees");
    assert_eq!(employees.row_count, Some(5));

    let warehouses = conn.get_table_info("warehouses").await.expect("warehouses");
    assert_eq!(warehouses.row_count, Some(3));

    let inventory = conn.get_table_info("inventory").await.expect("inventory");
    assert_eq!(inventory.row_count, Some(9));

    let promotions = conn.get_table_info("promotions").await.expect("promotions");
    assert_eq!(promotions.row_count, Some(5));

    conn.disconnect().await.ok();
}

#[tokio::test]
async fn sqlserver_full_schema() {
    let mut conn = SqlServerConnector::new(sqlserver_config());
    conn.connect().await.expect("connect");

    let schema = conn.get_schema().await.expect("get_schema");
    assert_eq!(schema.database_name, "UpsertTestSource");
    assert!(
        schema.tables.len() >= 12,
        "Should have at least 12 tables, got {}",
        schema.tables.len()
    );

    conn.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  POSTGRESQL - CONNECTION TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn postgres_connect_disconnect() {
    let mut conn = PostgresConnector::new(postgres_config());
    assert!(!conn.is_connected().await);

    conn.connect().await.expect("PostgreSQL connect failed");
    assert!(conn.is_connected().await);

    conn.disconnect().await.expect("PostgreSQL disconnect failed");
    assert!(!conn.is_connected().await);
}

#[tokio::test]
async fn postgres_connect_wrong_database() {
    let mut cfg = postgres_config();
    cfg.database = Some("nonexistent_db_xyz_999".to_string());
    let mut conn = PostgresConnector::new(cfg);

    let result = conn.connect().await;
    assert!(result.is_err(), "Should fail with nonexistent database");
}

// ═══════════════════════════════════════════════════════════════════════════
//  POSTGRESQL - SCHEMA TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn postgres_list_tables() {
    let mut conn = PostgresConnector::new(postgres_config());
    conn.connect().await.expect("connect");

    let tables = conn.get_tables().await.expect("get_tables");
    println!("PostgreSQL tables: {:?}", tables);

    let expected = vec![
        "categories",
        "customers",
        "order_items",
        "orders",
        "product_images",
        "products",
        "reviews",
        "shipping_rates",
        "shipping_zones",
        "wishlists",
    ];
    for tbl in &expected {
        assert!(
            tables.iter().any(|t| t == *tbl),
            "Missing table: {} in {:?}",
            tbl,
            tables
        );
    }

    // Verify SQL Server-only tables do NOT exist in PG
    let ss_only = vec!["product_tags", "employees", "warehouses", "inventory", "promotions", "audit_log"];
    for tbl in &ss_only {
        assert!(
            !tables.iter().any(|t| t == *tbl),
            "Table '{}' should NOT exist in PostgreSQL",
            tbl
        );
    }

    conn.disconnect().await.ok();
}

#[tokio::test]
async fn postgres_table_info_customers() {
    let mut conn = PostgresConnector::new(postgres_config());
    conn.connect().await.expect("connect");

    let info = conn.get_table_info("customers").await.expect("get_table_info");
    println!("PostgreSQL customers columns: {:#?}", info.columns);

    assert_eq!(info.table_name, "customers");
    assert_eq!(info.schema_name, "public");

    let col_names: Vec<&str> = info.columns.iter().map(|c| c.name.as_str()).collect();
    assert!(col_names.contains(&"id"), "Missing 'id'");
    assert!(col_names.contains(&"email"), "Missing 'email'");
    assert!(col_names.contains(&"date_of_birth"), "Missing 'date_of_birth' (PG specific)");
    assert!(col_names.contains(&"street_address"), "Missing 'street_address' (PG naming)");
    assert!(col_names.contains(&"region"), "Missing 'region' (PG naming)");
    assert!(col_names.contains(&"postal_code"), "Missing 'postal_code' (PG naming)");

    // SQL Server-specific columns should NOT exist
    assert!(!col_names.contains(&"middle_name"), "'middle_name' should NOT exist in PG");
    assert!(!col_names.contains(&"address_line1"), "'address_line1' should NOT exist in PG");
    assert!(!col_names.contains(&"address_line2"), "'address_line2' should NOT exist in PG");
    assert!(!col_names.contains(&"state"), "'state' should NOT exist in PG (uses 'region')");
    assert!(!col_names.contains(&"zip_code"), "'zip_code' should NOT exist in PG (uses 'postal_code')");
    assert!(!col_names.contains(&"credit_limit"), "'credit_limit' should NOT exist in PG");

    // Verify 'id' is primary key
    let id_col = info.columns.iter().find(|c| c.name == "id").unwrap();
    assert!(id_col.is_primary_key);

    // Verify loyalty_points is BIGINT (integer in information_schema)
    let lp = info.columns.iter().find(|c| c.name == "loyalty_points").unwrap();
    assert_eq!(lp.data_type, "bigint", "loyalty_points should be bigint in PG");

    // Verify email column
    let email_col = info.columns.iter().find(|c| c.name == "email").unwrap();
    assert!(!email_col.is_nullable);
    assert_eq!(email_col.max_length, Some(200), "PG email max_length=200");

    // Row count: 12 customers
    assert_eq!(info.row_count, Some(12), "PG customers should have 12 rows");

    conn.disconnect().await.ok();
}

#[tokio::test]
async fn postgres_table_info_products() {
    let mut conn = PostgresConnector::new(postgres_config());
    conn.connect().await.expect("connect");

    let info = conn.get_table_info("products").await.expect("get_table_info");
    println!("PostgreSQL products columns: {:#?}", info.columns);

    let col_names: Vec<&str> = info.columns.iter().map(|c| c.name.as_str()).collect();
    assert!(col_names.contains(&"color"), "Missing 'color' (PG specific)");
    assert!(col_names.contains(&"size"), "Missing 'size' (PG specific)");
    assert!(!col_names.contains(&"reorder_point"), "'reorder_point' should NOT exist in PG");

    // sku is character(12) in PG
    let sku = info.columns.iter().find(|c| c.name == "sku").unwrap();
    println!("PG sku data_type: {}", sku.data_type);
    assert!(
        sku.data_type.contains("character") || sku.data_type == "character",
        "PG sku should be character type, got: {}",
        sku.data_type
    );
    assert_eq!(sku.max_length, Some(12));

    // price is numeric(12,4)
    let price = info.columns.iter().find(|c| c.name == "price").unwrap();
    assert_eq!(price.data_type, "numeric");
    assert_eq!(price.precision, Some(12));
    assert_eq!(price.scale, Some(4));

    // weight_kg is real
    let weight = info.columns.iter().find(|c| c.name == "weight_kg").unwrap();
    assert_eq!(weight.data_type, "real");

    // Row count: 14 products
    assert_eq!(info.row_count, Some(14), "PG products should have 14 rows");

    conn.disconnect().await.ok();
}

#[tokio::test]
async fn postgres_table_info_orders() {
    let mut conn = PostgresConnector::new(postgres_config());
    conn.connect().await.expect("connect");

    let info = conn.get_table_info("orders").await.expect("get_table_info");

    let col_names: Vec<&str> = info.columns.iter().map(|c| c.name.as_str()).collect();
    assert!(col_names.contains(&"currency"), "Missing 'currency' (PG specific)");
    assert!(!col_names.contains(&"discount_code"), "'discount_code' should NOT exist in PG");
    assert!(!col_names.contains(&"shipping_cost"), "'shipping_cost' should NOT exist in PG");
    assert!(!col_names.contains(&"shipping_address"), "'shipping_address' should NOT exist in PG");

    // PG uses split address fields
    assert!(col_names.contains(&"ship_to_street"));
    assert!(col_names.contains(&"ship_to_city"));

    // status uses USER-DEFINED (enum) type in PG
    let status = info.columns.iter().find(|c| c.name == "status").unwrap();
    println!("PG orders.status data_type: {}", status.data_type);

    // Row count: 12 orders
    assert_eq!(info.row_count, Some(12), "PG orders should have 12 rows");

    conn.disconnect().await.ok();
}

#[tokio::test]
async fn postgres_row_counts() {
    let mut conn = PostgresConnector::new(postgres_config());
    conn.connect().await.expect("connect");

    let expected_counts = vec![
        ("categories", 8),
        ("customers", 12),
        ("products", 14),
        ("product_images", 6),
        ("shipping_zones", 4),
        ("shipping_rates", 9),
        ("orders", 12),
        ("order_items", 16),
        ("reviews", 14),
        ("wishlists", 8),
    ];

    for (table, expected) in &expected_counts {
        let count = conn
            .get_row_count(table)
            .await
            .unwrap_or_else(|e| panic!("get_row_count({}) failed: {}", table, e));
        assert_eq!(
            count, *expected,
            "Row count mismatch for {}: got {}, expected {}",
            table, count, expected
        );
    }

    conn.disconnect().await.ok();
}

#[tokio::test]
async fn postgres_table_info_pg_only_tables() {
    let mut conn = PostgresConnector::new(postgres_config());
    conn.connect().await.expect("connect");

    // Tables that exist only in PostgreSQL
    let images = conn.get_table_info("product_images").await.expect("product_images");
    assert_eq!(images.row_count, Some(6));
    let img_cols: Vec<&str> = images.columns.iter().map(|c| c.name.as_str()).collect();
    assert!(img_cols.contains(&"url"));
    assert!(img_cols.contains(&"is_primary"));

    let zones = conn.get_table_info("shipping_zones").await.expect("shipping_zones");
    assert_eq!(zones.row_count, Some(4));

    let rates = conn.get_table_info("shipping_rates").await.expect("shipping_rates");
    assert_eq!(rates.row_count, Some(9));

    let wishlists = conn.get_table_info("wishlists").await.expect("wishlists");
    assert_eq!(wishlists.row_count, Some(8));

    conn.disconnect().await.ok();
}

#[tokio::test]
async fn postgres_full_schema() {
    let mut conn = PostgresConnector::new(postgres_config());
    conn.connect().await.expect("connect");

    let schema = conn.get_schema().await.expect("get_schema");
    assert_eq!(schema.database_name, "upsert_test_target");
    assert!(
        schema.tables.len() >= 10,
        "Should have at least 10 tables, got {}",
        schema.tables.len()
    );

    conn.disconnect().await.ok();
}

#[tokio::test]
async fn postgres_get_rows() {
    let mut conn = PostgresConnector::new(postgres_config());
    conn.connect().await.expect("connect");

    let rows = conn
        .get_rows("customers", Some(5), None)
        .await
        .expect("get_rows");
    assert_eq!(rows.len(), 5, "Should return 5 rows with LIMIT 5");

    let first = &rows[0];
    assert!(first.contains_key("email"), "Row missing 'email'");
    assert!(first.contains_key("first_name"), "Row missing 'first_name'");

    conn.disconnect().await.ok();
}

#[tokio::test]
async fn postgres_execute_query() {
    let mut conn = PostgresConnector::new(postgres_config());
    conn.connect().await.expect("connect");

    let rows = conn
        .execute_query("SELECT email, first_name FROM customers ORDER BY email LIMIT 3")
        .await
        .expect("execute_query");
    assert_eq!(rows.len(), 3);
    assert!(rows[0].contains_key("email"));
    assert!(rows[0].contains_key("first_name"));

    conn.disconnect().await.ok();
}

#[tokio::test]
async fn postgres_get_table_info_nonexistent() {
    let mut conn = PostgresConnector::new(postgres_config());
    conn.connect().await.expect("connect");

    let info = conn.get_table_info("nonexistent_table_xyz_999").await;
    match info {
        Ok(ti) => {
            assert!(
                ti.columns.is_empty(),
                "Nonexistent table should have no columns"
            );
        }
        Err(_) => {
            // Error is also acceptable
        }
    }

    conn.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  CROSS-ENGINE DATA COMPARISON SPOTS
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn cross_engine_customer_data_differences() {
    // Verify the deliberate data differences between SQL Server and PostgreSQL
    let mut ss = SqlServerConnector::new(sqlserver_config());
    let mut pg = PostgresConnector::new(postgres_config());
    ss.connect().await.expect("SQL Server connect");
    pg.connect().await.expect("PostgreSQL connect");

    // Carol's loyalty_points: 2100 in SQL Server, 2400 in PG
    let ss_carol = ss
        .execute_query("SELECT loyalty_points FROM customers WHERE email = 'carol@example.com'")
        .await
        .expect("ss query");
    let pg_carol = pg
        .execute_query("SELECT loyalty_points FROM customers WHERE email = 'carol@example.com'")
        .await
        .expect("pg query");

    println!(
        "Carol loyalty_points - SS: {:?}, PG: {:?}",
        ss_carol[0].get("loyalty_points"),
        pg_carol[0].get("loyalty_points")
    );

    // Eve's phone: '555-0105' in SQL Server, '555-9999' in PG
    let ss_eve = ss
        .execute_query("SELECT phone FROM customers WHERE email = 'eve@example.com'")
        .await
        .expect("ss query");
    let pg_eve = pg
        .execute_query("SELECT phone FROM customers WHERE email = 'eve@example.com'")
        .await
        .expect("pg query");

    println!(
        "Eve phone - SS: {:?}, PG: {:?}",
        ss_eve[0].get("phone"),
        pg_eve[0].get("phone")
    );

    ss.disconnect().await.ok();
    pg.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  UNICODE HANDLING
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn sqlserver_unicode_data() {
    let mut conn = SqlServerConnector::new(sqlserver_config());
    conn.connect().await.expect("connect");

    let rows = conn
        .execute_query("SELECT email, first_name, last_name FROM customers WHERE country = 'DE'")
        .await
        .expect("unicode query");

    assert_eq!(rows.len(), 1, "Should find 1 German customer");
    let row = &rows[0];
    println!("German customer: {:?}", row);

    // Verify Unicode characters survived the round trip
    let first_name = row.get("first_name").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        first_name.contains("ü") || first_name.contains("Müller"),
        "German first_name should contain ü, got: {}",
        first_name
    );

    conn.disconnect().await.ok();
}

#[tokio::test]
async fn postgres_unicode_data() {
    let mut conn = PostgresConnector::new(postgres_config());
    conn.connect().await.expect("connect");

    let rows = conn
        .execute_query("SELECT email, first_name, last_name FROM customers WHERE country = 'JP'")
        .await
        .expect("unicode query");

    assert_eq!(rows.len(), 1, "Should find 1 Japanese customer");
    let row = &rows[0];
    println!("Japanese customer: {:?}", row);

    let first_name = row.get("first_name").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        first_name.contains("太郎"),
        "Japanese first_name should contain 太郎, got: {}",
        first_name
    );

    conn.disconnect().await.ok();
}
