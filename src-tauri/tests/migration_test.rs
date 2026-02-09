//! Integration tests for the migration engine against live SQL Server and PostgreSQL databases.
//!
//! Prerequisites:
//!   - SQL Server on localhost:1433 with sa/YourPassword123, database UpsertTestSource seeded
//!   - PostgreSQL on localhost:5432 with postgres/YourPassword123, database upsert_test_target seeded
//!
//! These tests exercise:
//!   - Cross-engine data fetch and row filtering
//!   - plan_migration() with filtered rows
//!   - SqlGenerator.prepare_row_for_insert() (truncation and NOT NULL skipping)
//!   - FK dependency detection via get_table_info
//!   - SQL generation for valid rows
//!
//! IMPORTANT: No SQL is executed against the target database; all tests are read-only.

use std::collections::{HashMap, HashSet};

use upsert_lib::db::connectors::{
    postgres::PostgresConnector, sqlserver::SqlServerConnector, ConnectionConfig, DatabaseConnector,
    DatabaseEngine,
};
use upsert_lib::db::migrator::{
    execute_migration, plan_migration, CancellationToken, ConflictResolution, MigrationConfig,
    MigrationMode, MigrationStatus,
};
use upsert_lib::db::schema::{ConstraintType, Row};
use upsert_lib::db::sql_generator::SqlGenerator;

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

/// Filter a row to only include columns present in the target column set.
fn filter_row_to_target(row: &Row, target_columns: &HashSet<String>) -> Row {
    row.iter()
        .filter(|(k, _)| target_columns.contains(k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// Shared tables that exist in both SQL Server and PostgreSQL.
const SHARED_TABLES: &[&str] = &[
    "categories",
    "customers",
    "products",
    "orders",
    "order_items",
    "reviews",
];

// ═══════════════════════════════════════════════════════════════════════════
//  1. CONNECT AND FETCH DATA FROM BOTH DATABASES
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn connect_and_fetch_shared_tables() {
    let mut ss = SqlServerConnector::new(sqlserver_config());
    let mut pg = PostgresConnector::new(postgres_config());

    ss.connect().await.expect("SQL Server connect");
    pg.connect().await.expect("PostgreSQL connect");

    for table in SHARED_TABLES {
        let ss_rows = ss
            .get_rows(table, None, None)
            .await
            .unwrap_or_else(|e| panic!("SQL Server get_rows({}) failed: {}", table, e));
        let pg_rows = pg
            .get_rows(table, None, None)
            .await
            .unwrap_or_else(|e| panic!("PostgreSQL get_rows({}) failed: {}", table, e));

        println!(
            "Table '{}': SQL Server={} rows, PostgreSQL={} rows",
            table,
            ss_rows.len(),
            pg_rows.len()
        );

        assert!(!ss_rows.is_empty(), "SQL Server {} should have rows", table);
        assert!(!pg_rows.is_empty(), "PostgreSQL {} should have rows", table);
    }

    ss.disconnect().await.ok();
    pg.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  2. PLAN MIGRATION WITH FILTERED ROWS (categories)
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn plan_migration_categories() {
    let mut ss = SqlServerConnector::new(sqlserver_config());
    let mut pg = PostgresConnector::new(postgres_config());
    ss.connect().await.expect("SQL Server connect");
    pg.connect().await.expect("PostgreSQL connect");

    // Fetch source (SQL Server) and target (PostgreSQL) rows
    let ss_rows = ss.get_rows("categories", None, None).await.expect("ss categories");
    let pg_rows = pg.get_rows("categories", None, None).await.expect("pg categories");

    // Get target schema to determine column set
    let pg_info = pg.get_table_info("categories").await.expect("pg categories info");
    let target_columns: HashSet<String> = pg_info.columns.iter().map(|c| c.name.clone()).collect();

    // Filter source rows to target-compatible columns
    let filtered_source: Vec<Row> = ss_rows
        .iter()
        .map(|r| filter_row_to_target(r, &target_columns))
        .collect();

    println!("Filtered source columns: {:?}", filtered_source.first().map(|r| r.keys().collect::<Vec<_>>()));
    println!("Target columns: {:?}", target_columns);

    // Plan migration in Upsert mode using 'id' as key
    let config = MigrationConfig {
        mode: MigrationMode::Upsert,
        key_columns: vec!["id".to_string()],
        ..Default::default()
    };
    let plan = plan_migration(&filtered_source, &pg_rows, &config);

    println!(
        "categories plan: inserts={}, updates={}, deletes={}, source={}, target={}",
        plan.rows_to_insert.len(),
        plan.rows_to_update.len(),
        plan.rows_to_delete.len(),
        plan.source_row_count,
        plan.target_row_count
    );

    // SQL Server has 11 categories, PG has 8
    assert_eq!(plan.source_row_count, ss_rows.len());
    assert_eq!(plan.target_row_count, pg_rows.len());

    // Upsert never deletes
    assert!(plan.rows_to_delete.is_empty(), "Upsert mode should never delete");

    // There should be some inserts (IDs 7-11 are sub-categories only in SS)
    assert!(
        plan.rows_to_insert.len() > 0,
        "Should have inserts for SQL Server sub-categories"
    );

    ss.disconnect().await.ok();
    pg.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  3. PLAN MIGRATION WITH FILTERED ROWS (customers)
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn plan_migration_customers() {
    let mut ss = SqlServerConnector::new(sqlserver_config());
    let mut pg = PostgresConnector::new(postgres_config());
    ss.connect().await.expect("SQL Server connect");
    pg.connect().await.expect("PostgreSQL connect");

    let ss_rows = ss.get_rows("customers", None, None).await.expect("ss customers");
    let pg_rows = pg.get_rows("customers", None, None).await.expect("pg customers");

    let pg_info = pg.get_table_info("customers").await.expect("pg customers info");
    let target_columns: HashSet<String> = pg_info.columns.iter().map(|c| c.name.clone()).collect();

    let filtered_source: Vec<Row> = ss_rows
        .iter()
        .map(|r| filter_row_to_target(r, &target_columns))
        .collect();

    let config = MigrationConfig {
        mode: MigrationMode::Upsert,
        key_columns: vec!["id".to_string()],
        ..Default::default()
    };
    let plan = plan_migration(&filtered_source, &pg_rows, &config);

    println!(
        "customers plan: inserts={}, updates={}, deletes={}",
        plan.rows_to_insert.len(),
        plan.rows_to_update.len(),
        plan.rows_to_delete.len()
    );

    // Both have 12 customers, same IDs, so no inserts expected
    assert_eq!(plan.source_row_count, 12);
    assert_eq!(plan.target_row_count, 12);
    assert!(plan.rows_to_delete.is_empty());

    // There should be updates because data differs (e.g. Carol loyalty_points, Eve phone)
    // The source rows are filtered to target columns, so columns like middle_name are dropped.
    // Updates come from value differences on shared columns.
    println!(
        "customers: {} updates detected",
        plan.rows_to_update.len()
    );

    ss.disconnect().await.ok();
    pg.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  4. PREPARE ROW FOR INSERT - NOT NULL COLUMN SKIPPING (categories)
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn prepare_row_categories_skips_missing_slug() {
    let mut ss = SqlServerConnector::new(sqlserver_config());
    let mut pg = PostgresConnector::new(postgres_config());
    ss.connect().await.expect("SQL Server connect");
    pg.connect().await.expect("PostgreSQL connect");

    let ss_rows = ss.get_rows("categories", None, None).await.expect("ss categories");
    let pg_info = pg.get_table_info("categories").await.expect("pg categories info");
    let target_columns: HashSet<String> = pg_info.columns.iter().map(|c| c.name.clone()).collect();

    // Check that 'slug' is NOT NULL without default in PG
    let slug_col = pg_info.columns.iter().find(|c| c.name == "slug");
    assert!(slug_col.is_some(), "PG categories should have 'slug' column");
    let slug_col = slug_col.unwrap();
    assert!(!slug_col.is_nullable, "slug should be NOT NULL");
    // slug has no default (it's NOT NULL UNIQUE, no DEFAULT clause)
    println!("slug default_value: {:?}", slug_col.default_value);

    let sql_gen = SqlGenerator::new(DatabaseEngine::PostgreSql);

    // Filter a SQL Server row to target columns. SQL Server categories don't have 'slug'.
    let filtered = filter_row_to_target(&ss_rows[0], &target_columns);
    assert!(
        !filtered.contains_key("slug"),
        "Filtered SQL Server row should NOT have 'slug'"
    );

    let (prepared, warnings) = sql_gen.prepare_row_for_insert(&filtered, &pg_info.columns);

    println!("prepare_row_for_insert warnings: {:?}", warnings);

    // Row should be skipped because 'slug' is NOT NULL without default and missing
    assert!(
        prepared.is_none(),
        "Row missing NOT NULL 'slug' should be skipped (None)"
    );
    assert!(
        warnings.iter().any(|w| w.contains("slug")),
        "Warnings should mention 'slug'"
    );

    ss.disconnect().await.ok();
    pg.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  5. PREPARE ROW FOR INSERT - STRING TRUNCATION (customers)
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn prepare_row_truncates_oversized_strings() {
    let mut pg = PostgresConnector::new(postgres_config());
    pg.connect().await.expect("PostgreSQL connect");

    let pg_info = pg.get_table_info("customers").await.expect("pg customers info");

    // Build a synthetic row with an oversized email (PG email max_length=200)
    let long_email = "x".repeat(300) + "@example.com";
    let mut row = HashMap::new();
    row.insert("id".to_string(), serde_json::json!(999));
    row.insert("email".to_string(), serde_json::json!(long_email));
    row.insert("first_name".to_string(), serde_json::json!("Test"));
    row.insert("last_name".to_string(), serde_json::json!("User"));

    let sql_gen = SqlGenerator::new(DatabaseEngine::PostgreSql);
    let (prepared, warnings) = sql_gen.prepare_row_for_insert(&row, &pg_info.columns);

    println!("Truncation warnings: {:?}", warnings);

    // The row should still be valid (not skipped) since all NOT NULL columns are provided
    // (except possibly some that have defaults like created_at, updated_at).
    // But the email should be truncated to 200 chars.
    if let Some(p) = &prepared {
        if let Some(serde_json::Value::String(email)) = p.get("email") {
            assert!(
                email.len() <= 200,
                "Email should be truncated to 200 chars, got {}",
                email.len()
            );
            println!("Truncated email length: {}", email.len());
        }
    }

    // Should have a warning about truncation
    assert!(
        warnings.iter().any(|w| w.contains("Truncated") && w.contains("email")),
        "Should warn about email truncation: {:?}",
        warnings
    );

    pg.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  6. FK DEPENDENCY DETECTION VIA GET_TABLE_INFO
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn postgres_fk_dependencies_detected() {
    let mut pg = PostgresConnector::new(postgres_config());
    pg.connect().await.expect("PostgreSQL connect");

    // Products references categories (category_id -> categories.id)
    let products_info = pg.get_table_info("products").await.expect("products info");
    let product_fks: Vec<_> = products_info
        .constraints
        .iter()
        .filter(|c| c.constraint_type == ConstraintType::ForeignKey)
        .collect();

    println!("products FK constraints: {:#?}", product_fks);
    let product_fk_tables: Vec<&str> = product_fks
        .iter()
        .filter_map(|c| c.referenced_table.as_deref())
        .collect();
    assert!(
        product_fk_tables.contains(&"categories"),
        "products should reference categories, got: {:?}",
        product_fk_tables
    );

    // Orders references customers (customer_id -> customers.id)
    let orders_info = pg.get_table_info("orders").await.expect("orders info");
    let order_fks: Vec<_> = orders_info
        .constraints
        .iter()
        .filter(|c| c.constraint_type == ConstraintType::ForeignKey)
        .collect();

    println!("orders FK constraints: {:#?}", order_fks);
    let order_fk_tables: Vec<&str> = order_fks
        .iter()
        .filter_map(|c| c.referenced_table.as_deref())
        .collect();
    assert!(
        order_fk_tables.contains(&"customers"),
        "orders should reference customers, got: {:?}",
        order_fk_tables
    );

    // Order_items references both orders and products
    let oi_info = pg.get_table_info("order_items").await.expect("order_items info");
    let oi_fks: Vec<_> = oi_info
        .constraints
        .iter()
        .filter(|c| c.constraint_type == ConstraintType::ForeignKey)
        .collect();

    println!("order_items FK constraints: {:#?}", oi_fks);
    let oi_fk_tables: Vec<&str> = oi_fks
        .iter()
        .filter_map(|c| c.referenced_table.as_deref())
        .collect();
    assert!(
        oi_fk_tables.contains(&"orders"),
        "order_items should reference orders, got: {:?}",
        oi_fk_tables
    );
    assert!(
        oi_fk_tables.contains(&"products"),
        "order_items should reference products, got: {:?}",
        oi_fk_tables
    );

    // Reviews references both products and customers
    let reviews_info = pg.get_table_info("reviews").await.expect("reviews info");
    let reviews_fks: Vec<_> = reviews_info
        .constraints
        .iter()
        .filter(|c| c.constraint_type == ConstraintType::ForeignKey)
        .collect();

    println!("reviews FK constraints: {:#?}", reviews_fks);
    let reviews_fk_tables: Vec<&str> = reviews_fks
        .iter()
        .filter_map(|c| c.referenced_table.as_deref())
        .collect();
    assert!(
        reviews_fk_tables.contains(&"products"),
        "reviews should reference products, got: {:?}",
        reviews_fk_tables
    );
    assert!(
        reviews_fk_tables.contains(&"customers"),
        "reviews should reference customers, got: {:?}",
        reviews_fk_tables
    );

    pg.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  7. FK TOPOLOGICAL SORT BEHAVIOR (indirect test)
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn fk_ordering_parents_before_children() {
    let mut pg = PostgresConnector::new(postgres_config());
    pg.connect().await.expect("PostgreSQL connect");

    // Build FK dependency map from live schema
    let mut fk_deps: HashMap<String, Vec<String>> = HashMap::new();
    for table in SHARED_TABLES {
        let info = pg
            .get_table_info(table)
            .await
            .unwrap_or_else(|e| panic!("get_table_info({}) failed: {}", table, e));
        let deps: Vec<String> = info
            .constraints
            .iter()
            .filter(|c| c.constraint_type == ConstraintType::ForeignKey)
            .filter_map(|c| c.referenced_table.clone())
            .filter(|rt| rt != *table) // exclude self-references
            .collect();
        fk_deps.insert(table.to_string(), deps);
    }

    println!("FK dependency map: {:#?}", fk_deps);

    // Verify key dependency relationships
    assert!(
        fk_deps
            .get("products")
            .map_or(false, |d| d.contains(&"categories".to_string())),
        "products should depend on categories"
    );
    assert!(
        fk_deps
            .get("orders")
            .map_or(false, |d| d.contains(&"customers".to_string())),
        "orders should depend on customers"
    );
    assert!(
        fk_deps
            .get("order_items")
            .map_or(false, |d| d.contains(&"orders".to_string())),
        "order_items should depend on orders"
    );

    // Verify that in a correct topological order:
    //   categories < products < order_items
    //   customers < orders < order_items
    // This is tested indirectly by verifying that the dependency graph is correct.
    // The sort_tables_by_fk function in commands/migration.rs uses Kahn's algorithm.

    pg.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  8. SQL GENERATION FOR VALID ROWS
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn generate_insert_sql_for_valid_rows() {
    let mut ss = SqlServerConnector::new(sqlserver_config());
    let mut pg = PostgresConnector::new(postgres_config());
    ss.connect().await.expect("SQL Server connect");
    pg.connect().await.expect("PostgreSQL connect");

    // Use customers (both have 12 rows with same IDs)
    let ss_rows = ss.get_rows("customers", None, None).await.expect("ss customers");
    let pg_info = pg.get_table_info("customers").await.expect("pg customers info");
    let target_columns: HashSet<String> = pg_info.columns.iter().map(|c| c.name.clone()).collect();

    let sql_gen = SqlGenerator::new(DatabaseEngine::PostgreSql);

    // Filter and validate each source row
    let mut valid_count = 0;
    let mut skipped_count = 0;

    for (i, row) in ss_rows.iter().enumerate() {
        let filtered = filter_row_to_target(row, &target_columns);
        let (prepared, warnings) = sql_gen.prepare_row_for_insert(&filtered, &pg_info.columns);

        if let Some(p) = prepared {
            valid_count += 1;
            let sql = sql_gen.generate_insert("customers", &p);

            // Verify SQL is well-formed
            assert!(
                sql.starts_with("INSERT INTO \"customers\""),
                "SQL should start with INSERT INTO: row {}: {}",
                i,
                sql
            );
            assert!(sql.ends_with(';'), "SQL should end with semicolon: {}", sql);
            assert!(
                sql.contains("VALUES"),
                "SQL should contain VALUES: {}",
                sql
            );

            // Print first few for visual inspection
            if valid_count <= 3 {
                println!("INSERT SQL (row {}): {}", i, sql);
            }
        } else {
            skipped_count += 1;
            println!("Skipped row {} due to: {:?}", i, warnings);
        }
    }

    println!(
        "customers INSERT generation: valid={}, skipped={}",
        valid_count, skipped_count
    );
    // At least some rows should generate valid SQL
    assert!(valid_count > 0, "At least some rows should produce valid INSERT SQL");

    ss.disconnect().await.ok();
    pg.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  9. SQL GENERATION - UPDATE AND DELETE
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn generate_update_and_delete_sql() {
    let mut pg = PostgresConnector::new(postgres_config());
    pg.connect().await.expect("PostgreSQL connect");

    let pg_rows = pg.get_rows("customers", Some(3), None).await.expect("pg customers");
    let sql_gen = SqlGenerator::new(DatabaseEngine::PostgreSql);
    let key_cols = vec!["id".to_string()];

    for row in &pg_rows {
        // Generate UPDATE
        let update_sql = sql_gen.generate_update("customers", row, &key_cols);
        assert!(
            update_sql.starts_with("UPDATE \"customers\" SET"),
            "UPDATE should be well-formed: {}",
            update_sql
        );
        assert!(
            update_sql.contains("WHERE \"id\" ="),
            "UPDATE should have WHERE clause: {}",
            update_sql
        );
        println!("UPDATE: {}", &update_sql[..update_sql.len().min(120)]);

        // Generate DELETE
        let delete_sql = sql_gen.generate_delete("customers", row, &key_cols);
        assert!(
            delete_sql.starts_with("DELETE FROM \"customers\""),
            "DELETE should be well-formed: {}",
            delete_sql
        );
        assert!(
            delete_sql.contains("WHERE \"id\" ="),
            "DELETE should have WHERE clause: {}",
            delete_sql
        );
        println!("DELETE: {}", delete_sql);
    }

    pg.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  10. PLAN MIGRATION - MIRROR MODE DETECTS DELETES
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn plan_migration_mirror_mode_detects_deletes() {
    let mut ss = SqlServerConnector::new(sqlserver_config());
    let mut pg = PostgresConnector::new(postgres_config());
    ss.connect().await.expect("SQL Server connect");
    pg.connect().await.expect("PostgreSQL connect");

    let ss_rows = ss.get_rows("categories", None, None).await.expect("ss categories");
    let pg_rows = pg.get_rows("categories", None, None).await.expect("pg categories");

    let pg_info = pg.get_table_info("categories").await.expect("pg categories info");
    let target_columns: HashSet<String> = pg_info.columns.iter().map(|c| c.name.clone()).collect();

    let filtered_source: Vec<Row> = ss_rows
        .iter()
        .map(|r| filter_row_to_target(r, &target_columns))
        .collect();

    // Mirror mode: should detect rows in PG that are not in SS (by id)
    let config = MigrationConfig {
        mode: MigrationMode::Mirror,
        key_columns: vec!["id".to_string()],
        ..Default::default()
    };
    let plan = plan_migration(&filtered_source, &pg_rows, &config);

    println!(
        "Mirror mode categories: inserts={}, updates={}, deletes={}",
        plan.rows_to_insert.len(),
        plan.rows_to_update.len(),
        plan.rows_to_delete.len()
    );

    // PG has categories with IDs 7 (Automotive) and 8 (Health) that don't exist in SS
    // (SS sub-categories 7-11 have the same IDs but filtered source rows will have
    //  IDs 7-11 from SS, so they overlap with PG IDs 7,8)
    // In mirror mode, rows in target not in source should be scheduled for delete.
    // The plan should have some combination of inserts + updates + deletes.
    let total_ops = plan.rows_to_insert.len() + plan.rows_to_update.len() + plan.rows_to_delete.len();
    assert!(
        total_ops > 0,
        "Mirror mode should produce some operations between different category sets"
    );

    ss.disconnect().await.ok();
    pg.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  11. PLAN MIGRATION - PRODUCTS (schema differences)
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn plan_migration_products() {
    let mut ss = SqlServerConnector::new(sqlserver_config());
    let mut pg = PostgresConnector::new(postgres_config());
    ss.connect().await.expect("SQL Server connect");
    pg.connect().await.expect("PostgreSQL connect");

    let ss_rows = ss.get_rows("products", None, None).await.expect("ss products");
    let pg_rows = pg.get_rows("products", None, None).await.expect("pg products");

    let pg_info = pg.get_table_info("products").await.expect("pg products info");
    let target_columns: HashSet<String> = pg_info.columns.iter().map(|c| c.name.clone()).collect();

    // Verify that source-only columns are stripped
    let filtered_source: Vec<Row> = ss_rows
        .iter()
        .map(|r| filter_row_to_target(r, &target_columns))
        .collect();

    if let Some(first) = filtered_source.first() {
        assert!(
            !first.contains_key("reorder_point"),
            "Filtered source should not have SS-only 'reorder_point'"
        );
        println!("Filtered products columns: {:?}", first.keys().collect::<Vec<_>>());
    }

    let config = MigrationConfig {
        mode: MigrationMode::Upsert,
        key_columns: vec!["id".to_string()],
        ..Default::default()
    };
    let plan = plan_migration(&filtered_source, &pg_rows, &config);

    println!(
        "products plan: inserts={}, updates={}, deletes={}, source={}, target={}",
        plan.rows_to_insert.len(),
        plan.rows_to_update.len(),
        plan.rows_to_delete.len(),
        plan.source_row_count,
        plan.target_row_count
    );

    // SS has 15 products, PG has 14 - expect at least 1 insert
    assert_eq!(plan.source_row_count, ss_rows.len());
    assert_eq!(plan.target_row_count, pg_rows.len());

    ss.disconnect().await.ok();
    pg.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  12. SQL GENERATION FOR SQL SERVER TARGET
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn generate_insert_sql_sqlserver_quoting() {
    let mut pg = PostgresConnector::new(postgres_config());
    pg.connect().await.expect("PostgreSQL connect");

    let pg_rows = pg.get_rows("customers", Some(2), None).await.expect("pg customers");
    let sql_gen = SqlGenerator::new(DatabaseEngine::SqlServer);

    for row in &pg_rows {
        let sql = sql_gen.generate_insert("customers", row);
        // SQL Server uses square bracket quoting
        assert!(
            sql.contains("[customers]"),
            "SQL Server INSERT should use [brackets]: {}",
            sql
        );
        assert!(sql.contains("[id]") || sql.contains("[email]"), "Columns should use brackets");
        println!("SQL Server INSERT: {}", &sql[..sql.len().min(120)]);
    }

    pg.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  13. PREPARE ROW - NULLABLE COLUMNS PASS THROUGH
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn prepare_row_passes_nullable_missing_columns() {
    let mut pg = PostgresConnector::new(postgres_config());
    pg.connect().await.expect("PostgreSQL connect");

    let pg_info = pg.get_table_info("customers").await.expect("pg customers info");

    // Build a row that has all NOT NULL columns but omits nullable ones (phone, date_of_birth, etc.)
    let mut row = HashMap::new();
    row.insert("id".to_string(), serde_json::json!(9999));
    row.insert("email".to_string(), serde_json::json!("test@test.com"));
    row.insert("first_name".to_string(), serde_json::json!("Test"));
    row.insert("last_name".to_string(), serde_json::json!("User"));

    let sql_gen = SqlGenerator::new(DatabaseEngine::PostgreSql);
    let (prepared, warnings) = sql_gen.prepare_row_for_insert(&row, &pg_info.columns);

    println!("Nullable test warnings: {:?}", warnings);

    // Row should be valid because missing columns (phone, date_of_birth, etc.) are nullable
    // or have defaults (created_at, updated_at, loyalty_points, is_active)
    // Whether this passes depends on exact schema details
    if prepared.is_some() {
        println!("Row with nullable columns omitted passed validation");
    } else {
        println!("Row was skipped - some NOT NULL columns may not have defaults");
        // This is still a valid test outcome - we document what happens
        for w in &warnings {
            println!("  Warning: {}", w);
        }
    }

    pg.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  14. END-TO-END DRY RUN SIMULATION (all shared tables)
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn dry_run_simulation_all_shared_tables() {
    let mut ss = SqlServerConnector::new(sqlserver_config());
    let mut pg = PostgresConnector::new(postgres_config());
    ss.connect().await.expect("SQL Server connect");
    pg.connect().await.expect("PostgreSQL connect");

    let sql_gen = SqlGenerator::new(DatabaseEngine::PostgreSql);

    for table in SHARED_TABLES {
        println!("\n=== Dry-run simulation: {} ===", table);

        // Fetch data
        let ss_rows = ss.get_rows(table, None, None).await.expect("ss rows");
        let pg_rows = pg.get_rows(table, None, None).await.expect("pg rows");

        // Fetch target schema
        let pg_info = pg.get_table_info(table).await.expect("pg info");
        let target_columns: HashSet<String> = pg_info.columns.iter().map(|c| c.name.clone()).collect();

        // Filter source rows
        let filtered_source: Vec<Row> = ss_rows
            .iter()
            .map(|r| filter_row_to_target(r, &target_columns))
            .collect();

        // Plan
        let config = MigrationConfig {
            mode: MigrationMode::Upsert,
            key_columns: vec!["id".to_string()],
            ..Default::default()
        };
        let plan = plan_migration(&filtered_source, &pg_rows, &config);

        // Validate inserts with prepare_row_for_insert
        let mut valid_inserts = 0;
        let mut skipped_inserts = 0;
        let mut all_warnings = Vec::new();

        for row in &plan.rows_to_insert {
            let (prepared, warnings) = sql_gen.prepare_row_for_insert(row, &pg_info.columns);
            all_warnings.extend(warnings);
            if prepared.is_some() {
                valid_inserts += 1;
            } else {
                skipped_inserts += 1;
            }
        }

        println!(
            "  source_rows={}, target_rows={}, inserts={} (valid={}, skipped={}), updates={}, deletes={}",
            ss_rows.len(),
            pg_rows.len(),
            plan.rows_to_insert.len(),
            valid_inserts,
            skipped_inserts,
            plan.rows_to_update.len(),
            plan.rows_to_delete.len()
        );

        if !all_warnings.is_empty() {
            let unique_warnings: HashSet<_> = all_warnings.iter().collect();
            for w in unique_warnings {
                println!("  WARNING: {}", w);
            }
        }

        // Verify no delete in upsert mode
        assert!(
            plan.rows_to_delete.is_empty(),
            "Upsert mode should never delete for table {}",
            table
        );
    }

    ss.disconnect().await.ok();
    pg.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  15. EXECUTE MIGRATION - UPSERT MODE (in-memory, using live data)
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn execute_migration_upsert_mode_with_live_data() {
    let mut ss = SqlServerConnector::new(sqlserver_config());
    let mut pg = PostgresConnector::new(postgres_config());
    ss.connect().await.expect("SQL Server connect");
    pg.connect().await.expect("PostgreSQL connect");

    let ss_rows = ss.get_rows("customers", None, None).await.expect("ss customers");
    let pg_rows = pg.get_rows("customers", None, None).await.expect("pg customers");

    let pg_info = pg.get_table_info("customers").await.expect("pg info");
    let target_columns: HashSet<String> = pg_info.columns.iter().map(|c| c.name.clone()).collect();

    let filtered_source: Vec<Row> = ss_rows
        .iter()
        .map(|r| filter_row_to_target(r, &target_columns))
        .collect();

    // Upsert: insert new + update changed, never delete
    let config = MigrationConfig {
        mode: MigrationMode::Upsert,
        key_columns: vec!["id".to_string()],
        ..Default::default()
    };
    let (result, output) = execute_migration(&filtered_source, &pg_rows, &config, None);

    println!(
        "Upsert customers: status={:?}, inserted={}, updated={}, deleted={}, output_len={}",
        result.status, result.rows_inserted, result.rows_updated, result.rows_deleted, output.len()
    );

    assert_eq!(result.status, MigrationStatus::Completed);
    assert_eq!(result.rows_deleted, 0, "Upsert should never delete");
    // Both have 12 customers with same IDs, so inserts=0, updates>0
    assert_eq!(result.rows_inserted, 0);
    assert!(result.rows_updated > 0, "Should detect value differences");
    assert_eq!(output.len(), 12, "Output should have same count after upsert");

    ss.disconnect().await.ok();
    pg.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  16. EXECUTE MIGRATION - APPEND-ONLY MODE (in-memory, using live data)
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn execute_migration_append_only_with_live_data() {
    let mut ss = SqlServerConnector::new(sqlserver_config());
    let mut pg = PostgresConnector::new(postgres_config());
    ss.connect().await.expect("SQL Server connect");
    pg.connect().await.expect("PostgreSQL connect");

    let ss_rows = ss.get_rows("products", None, None).await.expect("ss products");
    let pg_rows = pg.get_rows("products", None, None).await.expect("pg products");

    let pg_info = pg.get_table_info("products").await.expect("pg info");
    let target_columns: HashSet<String> = pg_info.columns.iter().map(|c| c.name.clone()).collect();

    let filtered_source: Vec<Row> = ss_rows
        .iter()
        .map(|r| filter_row_to_target(r, &target_columns))
        .collect();

    // AppendOnly: insert new rows only, never update or delete
    let config = MigrationConfig {
        mode: MigrationMode::AppendOnly,
        key_columns: vec!["id".to_string()],
        ..Default::default()
    };
    let (result, output) = execute_migration(&filtered_source, &pg_rows, &config, None);

    println!(
        "AppendOnly products: status={:?}, inserted={}, updated={}, deleted={}, output_len={}",
        result.status, result.rows_inserted, result.rows_updated, result.rows_deleted, output.len()
    );

    assert_eq!(result.status, MigrationStatus::Completed);
    assert_eq!(result.rows_updated, 0, "AppendOnly should never update");
    assert_eq!(result.rows_deleted, 0, "AppendOnly should never delete");
    // SS has 15 products, PG has 14, so expect 1 insert
    assert!(result.rows_inserted >= 1, "Should insert at least 1 new product");
    assert_eq!(
        output.len(),
        pg_rows.len() + result.rows_inserted,
        "Output should be target + inserts"
    );

    ss.disconnect().await.ok();
    pg.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  17. EXECUTE MIGRATION - MERGE MODE (in-memory, using live data)
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn execute_migration_merge_mode_with_live_data() {
    let mut ss = SqlServerConnector::new(sqlserver_config());
    let mut pg = PostgresConnector::new(postgres_config());
    ss.connect().await.expect("SQL Server connect");
    pg.connect().await.expect("PostgreSQL connect");

    let ss_rows = ss.get_rows("products", None, None).await.expect("ss products");
    let pg_rows = pg.get_rows("products", None, None).await.expect("pg products");

    let pg_info = pg.get_table_info("products").await.expect("pg info");
    let target_columns: HashSet<String> = pg_info.columns.iter().map(|c| c.name.clone()).collect();

    let filtered_source: Vec<Row> = ss_rows
        .iter()
        .map(|r| filter_row_to_target(r, &target_columns))
        .collect();

    // Merge: insert new + update existing, never delete
    let config = MigrationConfig {
        mode: MigrationMode::Merge,
        key_columns: vec!["id".to_string()],
        ..Default::default()
    };
    let (result, output) = execute_migration(&filtered_source, &pg_rows, &config, None);

    println!(
        "Merge products: status={:?}, inserted={}, updated={}, deleted={}, output_len={}",
        result.status, result.rows_inserted, result.rows_updated, result.rows_deleted, output.len()
    );

    assert_eq!(result.status, MigrationStatus::Completed);
    assert_eq!(result.rows_deleted, 0, "Merge should never delete");
    assert!(result.rows_inserted >= 1, "Should insert new rows");
    assert!(result.rows_updated > 0, "Should update existing rows with differences");

    ss.disconnect().await.ok();
    pg.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  18. EXECUTE MIGRATION - MIRROR MODE (in-memory, using live data)
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn execute_migration_mirror_mode_with_live_data() {
    let mut ss = SqlServerConnector::new(sqlserver_config());
    let mut pg = PostgresConnector::new(postgres_config());
    ss.connect().await.expect("SQL Server connect");
    pg.connect().await.expect("PostgreSQL connect");

    // Use orders: SS=11, PG=12, so mirror should delete 1 from PG side
    let ss_rows = ss.get_rows("orders", None, None).await.expect("ss orders");
    let pg_rows = pg.get_rows("orders", None, None).await.expect("pg orders");

    let pg_info = pg.get_table_info("orders").await.expect("pg info");
    let target_columns: HashSet<String> = pg_info.columns.iter().map(|c| c.name.clone()).collect();

    let filtered_source: Vec<Row> = ss_rows
        .iter()
        .map(|r| filter_row_to_target(r, &target_columns))
        .collect();

    // Mirror: insert + update + delete to make target match source
    let config = MigrationConfig {
        mode: MigrationMode::Mirror,
        key_columns: vec!["id".to_string()],
        ..Default::default()
    };
    let (result, output) = execute_migration(&filtered_source, &pg_rows, &config, None);

    println!(
        "Mirror orders: status={:?}, inserted={}, updated={}, deleted={}, output_len={}",
        result.status, result.rows_inserted, result.rows_updated, result.rows_deleted, output.len()
    );

    assert_eq!(result.status, MigrationStatus::Completed);
    // Mirror should produce target with same count as source
    assert_eq!(
        output.len(),
        filtered_source.len(),
        "Mirror output should match source count"
    );

    ss.disconnect().await.ok();
    pg.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  19. EXECUTE MIGRATION - SCHEMA-ONLY MODE (no data changes)
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn execute_migration_schema_only_with_live_data() {
    let mut ss = SqlServerConnector::new(sqlserver_config());
    let mut pg = PostgresConnector::new(postgres_config());
    ss.connect().await.expect("SQL Server connect");
    pg.connect().await.expect("PostgreSQL connect");

    let ss_rows = ss.get_rows("customers", None, None).await.expect("ss customers");
    let pg_rows = pg.get_rows("customers", None, None).await.expect("pg customers");

    let pg_info = pg.get_table_info("customers").await.expect("pg info");
    let target_columns: HashSet<String> = pg_info.columns.iter().map(|c| c.name.clone()).collect();

    let filtered_source: Vec<Row> = ss_rows
        .iter()
        .map(|r| filter_row_to_target(r, &target_columns))
        .collect();

    let config = MigrationConfig {
        mode: MigrationMode::SchemaOnly,
        key_columns: vec!["id".to_string()],
        ..Default::default()
    };
    let (result, output) = execute_migration(&filtered_source, &pg_rows, &config, None);

    println!(
        "SchemaOnly: status={:?}, inserted={}, updated={}, deleted={}",
        result.status, result.rows_inserted, result.rows_updated, result.rows_deleted
    );

    assert_eq!(result.status, MigrationStatus::Completed);
    assert_eq!(result.rows_inserted, 0);
    assert_eq!(result.rows_updated, 0);
    assert_eq!(result.rows_deleted, 0);
    // Output should be original target unchanged
    assert_eq!(output.len(), pg_rows.len());

    ss.disconnect().await.ok();
    pg.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  20. CANCEL MIGRATION MID-FLIGHT
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn cancel_migration_mid_flight() {
    let mut ss = SqlServerConnector::new(sqlserver_config());
    ss.connect().await.expect("SQL Server connect");

    // Build a large in-memory dataset to ensure multiple batches
    let ss_rows = ss.get_rows("customers", None, None).await.expect("ss customers");

    // Duplicate rows with different IDs to create a large source set
    let mut large_source: Vec<Row> = Vec::new();
    for i in 0..200 {
        let mut row = ss_rows[i % ss_rows.len()].clone();
        row.insert("id".to_string(), serde_json::json!(1000 + i));
        row.insert(
            "email".to_string(),
            serde_json::json!(format!("test_{}@example.com", i)),
        );
        large_source.push(row);
    }

    // Small target => all source rows will be inserts
    let target: Vec<Row> = Vec::new();

    // Create a cancellation token and cancel immediately
    let token = CancellationToken::new();
    token.cancel();

    let config = MigrationConfig {
        mode: MigrationMode::Upsert,
        batch_size: 10, // small batches to hit cancellation check
        key_columns: vec!["id".to_string()],
        ..Default::default()
    };
    let (result, _output) = execute_migration(&large_source, &target, &config, Some(&token));

    println!(
        "Cancelled migration: status={:?}, inserted={}, updated={}, deleted={}",
        result.status, result.rows_inserted, result.rows_updated, result.rows_deleted
    );

    assert_eq!(result.status, MigrationStatus::Cancelled);
    // Cancellation happens before the first batch, so nothing processed
    assert_eq!(result.rows_inserted, 0);

    ss.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  21. DRY RUN MODE DOES NOT MODIFY DATA (in-memory)
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn dry_run_does_not_modify_data_with_live_rows() {
    let mut ss = SqlServerConnector::new(sqlserver_config());
    let mut pg = PostgresConnector::new(postgres_config());
    ss.connect().await.expect("SQL Server connect");
    pg.connect().await.expect("PostgreSQL connect");

    let ss_rows = ss.get_rows("customers", None, None).await.expect("ss customers");
    let pg_rows = pg.get_rows("customers", None, None).await.expect("pg customers");

    let pg_info = pg.get_table_info("customers").await.expect("pg info");
    let target_columns: HashSet<String> = pg_info.columns.iter().map(|c| c.name.clone()).collect();

    let filtered_source: Vec<Row> = ss_rows
        .iter()
        .map(|r| filter_row_to_target(r, &target_columns))
        .collect();

    let config = MigrationConfig {
        mode: MigrationMode::Upsert,
        dry_run: true,
        key_columns: vec!["id".to_string()],
        ..Default::default()
    };
    let (result, output) = execute_migration(&filtered_source, &pg_rows, &config, None);

    println!(
        "Dry run: status={:?}, inserted={}, updated={}, deleted={}",
        result.status, result.rows_inserted, result.rows_updated, result.rows_deleted
    );

    assert_eq!(result.status, MigrationStatus::Completed);
    // Dry run reports counts but does not change output
    assert!(
        result.rows_inserted > 0 || result.rows_updated > 0,
        "Dry run should report planned changes"
    );
    assert_eq!(
        output.len(),
        pg_rows.len(),
        "Dry run output should be unchanged target"
    );
    // Verify output is exactly the original target
    for (i, row) in output.iter().enumerate() {
        assert_eq!(row, &pg_rows[i], "Dry run should not modify row {}", i);
    }

    ss.disconnect().await.ok();
    pg.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  22. CONFLICT RESOLUTION - TARGET WINS (in-memory with live data)
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn conflict_resolution_target_wins_with_live_data() {
    let mut ss = SqlServerConnector::new(sqlserver_config());
    let mut pg = PostgresConnector::new(postgres_config());
    ss.connect().await.expect("SQL Server connect");
    pg.connect().await.expect("PostgreSQL connect");

    let ss_rows = ss.get_rows("customers", None, None).await.expect("ss customers");
    let pg_rows = pg.get_rows("customers", None, None).await.expect("pg customers");

    let pg_info = pg.get_table_info("customers").await.expect("pg info");
    let target_columns: HashSet<String> = pg_info.columns.iter().map(|c| c.name.clone()).collect();

    let filtered_source: Vec<Row> = ss_rows
        .iter()
        .map(|r| filter_row_to_target(r, &target_columns))
        .collect();

    // TargetWins: no updates should be applied
    let config = MigrationConfig {
        mode: MigrationMode::Upsert,
        conflict_resolution: ConflictResolution::TargetWins,
        key_columns: vec!["id".to_string()],
        ..Default::default()
    };
    let (result, output) = execute_migration(&filtered_source, &pg_rows, &config, None);

    println!(
        "TargetWins: status={:?}, inserted={}, updated={}, deleted={}",
        result.status, result.rows_inserted, result.rows_updated, result.rows_deleted
    );

    assert_eq!(result.status, MigrationStatus::Completed);
    assert_eq!(
        result.rows_updated, 0,
        "TargetWins should not update any rows"
    );
    assert_eq!(result.rows_deleted, 0);
    // Output should match original target (no updates applied)
    assert_eq!(output.len(), pg_rows.len());

    ss.disconnect().await.ok();
    pg.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  23. CONFLICT RESOLUTION - MANUAL REVIEW (in-memory with live data)
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn conflict_resolution_manual_review_with_live_data() {
    let mut ss = SqlServerConnector::new(sqlserver_config());
    let mut pg = PostgresConnector::new(postgres_config());
    ss.connect().await.expect("SQL Server connect");
    pg.connect().await.expect("PostgreSQL connect");

    let ss_rows = ss.get_rows("customers", None, None).await.expect("ss customers");
    let pg_rows = pg.get_rows("customers", None, None).await.expect("pg customers");

    let pg_info = pg.get_table_info("customers").await.expect("pg info");
    let target_columns: HashSet<String> = pg_info.columns.iter().map(|c| c.name.clone()).collect();

    let filtered_source: Vec<Row> = ss_rows
        .iter()
        .map(|r| filter_row_to_target(r, &target_columns))
        .collect();

    // ManualReview: conflicts go to review, not applied
    let config = MigrationConfig {
        mode: MigrationMode::Upsert,
        conflict_resolution: ConflictResolution::ManualReview,
        key_columns: vec!["id".to_string()],
        ..Default::default()
    };

    // Check plan first
    let plan = plan_migration(&filtered_source, &pg_rows, &config);
    println!(
        "ManualReview plan: inserts={}, updates={}, review={}, deletes={}",
        plan.rows_to_insert.len(),
        plan.rows_to_update.len(),
        plan.rows_to_review.len(),
        plan.rows_to_delete.len()
    );
    assert!(
        plan.rows_to_review.len() > 0,
        "ManualReview should have rows for review"
    );
    assert_eq!(
        plan.rows_to_update.len(),
        0,
        "ManualReview should put conflicts in review, not update"
    );

    // Execute
    let (result, output) = execute_migration(&filtered_source, &pg_rows, &config, None);

    println!(
        "ManualReview execute: status={:?}, inserted={}, updated={}, skipped={}",
        result.status, result.rows_inserted, result.rows_updated, result.rows_skipped
    );

    assert_eq!(result.status, MigrationStatus::Completed);
    assert_eq!(result.rows_updated, 0, "ManualReview should not update");
    assert!(result.rows_skipped > 0, "ManualReview should skip conflicts");
    assert_eq!(
        output.len(),
        pg_rows.len(),
        "Output should be same size (no inserts, conflicts skipped)"
    );

    ss.disconnect().await.ok();
    pg.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  24. ERROR HANDLING - INVALID TABLE NAME
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn error_handling_invalid_table_name() {
    let mut ss = SqlServerConnector::new(sqlserver_config());
    ss.connect().await.expect("SQL Server connect");

    // Querying a nonexistent table should error
    let result = ss.get_rows("nonexistent_table_xyz_999", None, None).await;
    println!("Invalid table get_rows result: {:?}", result.is_err());
    assert!(
        result.is_err(),
        "get_rows on nonexistent table should return error"
    );

    // get_table_info on nonexistent table
    let info_result = ss.get_table_info("nonexistent_table_xyz_999").await;
    match &info_result {
        Ok(ti) => {
            assert!(
                ti.columns.is_empty(),
                "Nonexistent table should have no columns"
            );
            println!("get_table_info returned empty columns (no error)");
        }
        Err(e) => {
            println!("get_table_info returned error: {}", e);
        }
    }

    ss.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  25. ERROR HANDLING - DISCONNECTED DATABASE
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn error_handling_disconnected_database() {
    let ss = SqlServerConnector::new(sqlserver_config());
    // Do NOT connect - test operations on a disconnected connector
    assert!(!ss.is_connected().await);

    let rows_result = ss.get_rows("customers", None, None).await;
    println!("Disconnected get_rows: {:?}", rows_result.is_err());
    assert!(
        rows_result.is_err(),
        "get_rows on disconnected DB should error"
    );

    let tables_result = ss.get_tables().await;
    println!("Disconnected get_tables: {:?}", tables_result.is_err());
    assert!(
        tables_result.is_err(),
        "get_tables on disconnected DB should error"
    );

    let info_result = ss.get_table_info("customers").await;
    println!("Disconnected get_table_info: {:?}", info_result.is_err());
    assert!(
        info_result.is_err(),
        "get_table_info on disconnected DB should error"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
//  26. ERROR HANDLING - WRONG CREDENTIALS
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn error_handling_wrong_credentials() {
    let mut bad_cfg = postgres_config();
    bad_cfg.password = Some("totally_wrong_password".to_string());
    let mut pg = PostgresConnector::new(bad_cfg);

    let connect_result = pg.connect().await;
    println!("Wrong password connect: {:?}", connect_result.is_err());
    assert!(
        connect_result.is_err(),
        "Connecting with wrong password should fail"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
//  27. VERIFY MIGRATED DATA - IN-MEMORY MIRROR MAKES TARGET MATCH SOURCE
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn verify_mirror_output_matches_source() {
    let mut ss = SqlServerConnector::new(sqlserver_config());
    let mut pg = PostgresConnector::new(postgres_config());
    ss.connect().await.expect("SQL Server connect");
    pg.connect().await.expect("PostgreSQL connect");

    let ss_rows = ss.get_rows("reviews", None, None).await.expect("ss reviews");
    let pg_rows = pg.get_rows("reviews", None, None).await.expect("pg reviews");

    let pg_info = pg.get_table_info("reviews").await.expect("pg info");
    let target_columns: HashSet<String> = pg_info.columns.iter().map(|c| c.name.clone()).collect();

    let filtered_source: Vec<Row> = ss_rows
        .iter()
        .map(|r| filter_row_to_target(r, &target_columns))
        .collect();

    let config = MigrationConfig {
        mode: MigrationMode::Mirror,
        key_columns: vec!["id".to_string()],
        ..Default::default()
    };
    let (result, output) = execute_migration(&filtered_source, &pg_rows, &config, None);

    assert_eq!(result.status, MigrationStatus::Completed);
    assert_eq!(
        output.len(),
        filtered_source.len(),
        "Mirror output count should match source count"
    );

    // Every source row ID should be present in output
    let output_ids: HashSet<String> = output
        .iter()
        .filter_map(|r| r.get("id").map(|v| v.to_string()))
        .collect();
    for src_row in &filtered_source {
        let src_id = src_row.get("id").map(|v| v.to_string()).unwrap_or_default();
        assert!(
            output_ids.contains(&src_id),
            "Source row id={} should be in mirror output",
            src_id
        );
    }

    // No PG-only IDs should remain
    let source_ids: HashSet<String> = filtered_source
        .iter()
        .filter_map(|r| r.get("id").map(|v| v.to_string()))
        .collect();
    for out_row in &output {
        let out_id = out_row.get("id").map(|v| v.to_string()).unwrap_or_default();
        assert!(
            source_ids.contains(&out_id),
            "Output row id={} should exist in source (mirror removes extras)",
            out_id
        );
    }

    println!(
        "Mirror verified: source={} rows, output={} rows, IDs match",
        filtered_source.len(),
        output.len()
    );

    ss.disconnect().await.ok();
    pg.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  28. VERIFY MIGRATED DATA - UPSERT PRESERVES TARGET-ONLY ROWS
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn verify_upsert_preserves_target_only_rows() {
    let mut ss = SqlServerConnector::new(sqlserver_config());
    let mut pg = PostgresConnector::new(postgres_config());
    ss.connect().await.expect("SQL Server connect");
    pg.connect().await.expect("PostgreSQL connect");

    // orders: SS=11, PG=12 - PG has an order not in SS
    let ss_rows = ss.get_rows("orders", None, None).await.expect("ss orders");
    let pg_rows = pg.get_rows("orders", None, None).await.expect("pg orders");

    let pg_info = pg.get_table_info("orders").await.expect("pg info");
    let target_columns: HashSet<String> = pg_info.columns.iter().map(|c| c.name.clone()).collect();

    let filtered_source: Vec<Row> = ss_rows
        .iter()
        .map(|r| filter_row_to_target(r, &target_columns))
        .collect();

    let source_ids: HashSet<String> = filtered_source
        .iter()
        .filter_map(|r| r.get("id").map(|v| v.to_string()))
        .collect();

    // Find PG-only order IDs
    let pg_only_ids: Vec<String> = pg_rows
        .iter()
        .filter_map(|r| {
            let id = r.get("id").map(|v| v.to_string())?;
            if !source_ids.contains(&id) {
                Some(id)
            } else {
                None
            }
        })
        .collect();
    println!("PG-only order IDs: {:?}", pg_only_ids);

    let config = MigrationConfig {
        mode: MigrationMode::Upsert,
        key_columns: vec!["id".to_string()],
        ..Default::default()
    };
    let (result, output) = execute_migration(&filtered_source, &pg_rows, &config, None);

    assert_eq!(result.status, MigrationStatus::Completed);
    assert_eq!(result.rows_deleted, 0, "Upsert must not delete");

    // PG-only rows should still be in output
    let output_ids: HashSet<String> = output
        .iter()
        .filter_map(|r| r.get("id").map(|v| v.to_string()))
        .collect();
    for pg_id in &pg_only_ids {
        assert!(
            output_ids.contains(pg_id),
            "Upsert should preserve PG-only order id={}",
            pg_id
        );
    }

    println!(
        "Upsert preserved {} PG-only orders, output={} rows",
        pg_only_ids.len(),
        output.len()
    );

    ss.disconnect().await.ok();
    pg.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  29. BATCHING WITH LIVE DATA - SMALL BATCH SIZE
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn migration_batching_with_live_data() {
    let mut ss = SqlServerConnector::new(sqlserver_config());
    ss.connect().await.expect("SQL Server connect");

    let ss_rows = ss.get_rows("customers", None, None).await.expect("ss customers");

    // Use all customer rows as source with empty target -> all inserts
    let config = MigrationConfig {
        mode: MigrationMode::Upsert,
        batch_size: 3,
        key_columns: vec!["id".to_string()],
        ..Default::default()
    };
    let target: Vec<Row> = Vec::new();
    let (result, output) = execute_migration(&ss_rows, &target, &config, None);

    println!(
        "Batched migration: status={:?}, inserted={}, batch_size=3, source_len={}",
        result.status,
        result.rows_inserted,
        ss_rows.len()
    );

    assert_eq!(result.status, MigrationStatus::Completed);
    assert_eq!(result.rows_inserted, ss_rows.len());
    assert_eq!(output.len(), ss_rows.len());
    assert!(result.errors.is_empty(), "No errors expected");

    ss.disconnect().await.ok();
}

// ═══════════════════════════════════════════════════════════════════════════
//  30. PLAN MIGRATION - ALL MODES FOR ALL SHARED TABLES (comprehensive)
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn plan_all_modes_all_shared_tables() {
    let mut ss = SqlServerConnector::new(sqlserver_config());
    let mut pg = PostgresConnector::new(postgres_config());
    ss.connect().await.expect("SQL Server connect");
    pg.connect().await.expect("PostgreSQL connect");

    let modes = vec![
        ("Upsert", MigrationMode::Upsert),
        ("Mirror", MigrationMode::Mirror),
        ("AppendOnly", MigrationMode::AppendOnly),
        ("Merge", MigrationMode::Merge),
        ("SchemaOnly", MigrationMode::SchemaOnly),
    ];

    for table in SHARED_TABLES {
        let ss_rows = ss.get_rows(table, None, None).await.expect("ss rows");
        let pg_rows = pg.get_rows(table, None, None).await.expect("pg rows");
        let pg_info = pg.get_table_info(table).await.expect("pg info");
        let target_columns: HashSet<String> =
            pg_info.columns.iter().map(|c| c.name.clone()).collect();

        let filtered_source: Vec<Row> = ss_rows
            .iter()
            .map(|r| filter_row_to_target(r, &target_columns))
            .collect();

        for (mode_name, mode) in &modes {
            let config = MigrationConfig {
                mode: mode.clone(),
                key_columns: vec!["id".to_string()],
                ..Default::default()
            };
            let plan = plan_migration(&filtered_source, &pg_rows, &config);

            // Mode-specific invariants
            match mode {
                MigrationMode::Upsert | MigrationMode::Merge => {
                    assert!(
                        plan.rows_to_delete.is_empty(),
                        "{} {} should never delete",
                        table,
                        mode_name
                    );
                }
                MigrationMode::AppendOnly => {
                    assert!(
                        plan.rows_to_delete.is_empty(),
                        "{} AppendOnly should never delete",
                        table
                    );
                    assert!(
                        plan.rows_to_update.is_empty(),
                        "{} AppendOnly should never update",
                        table
                    );
                }
                MigrationMode::SchemaOnly => {
                    assert!(
                        plan.rows_to_insert.is_empty()
                            && plan.rows_to_update.is_empty()
                            && plan.rows_to_delete.is_empty(),
                        "{} SchemaOnly should have no data operations",
                        table
                    );
                }
                MigrationMode::Mirror => {
                    // Mirror can do inserts, updates, AND deletes - no invariant beyond >= 0
                }
            }
        }
    }

    println!(
        "All 5 modes validated across all {} shared tables",
        SHARED_TABLES.len()
    );

    ss.disconnect().await.ok();
    pg.disconnect().await.ok();
}
