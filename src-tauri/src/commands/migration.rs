use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use crate::db::data_comparator::{compare_data, DataCompareConfig, MatchStrategy};
use crate::db::migrator::{
    plan_migration, CancellationToken, MigrationConfig, MigrationMode,
};
use crate::db::registry::{ConnectionRegistry, MigrationState};
use crate::db::schema::{ColumnInfo, ConstraintType, Row};
use crate::db::sql_generator::SqlGenerator;

// ── DTOs ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableMappingDto {
    pub source_table: String,
    pub target_table: String,
    pub key_columns: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationConfigDto {
    pub mode: String,
    pub conflict_resolution: String,
    pub batch_size: usize,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DryRunRequest {
    pub source_connection_id: String,
    pub target_connection_id: String,
    pub tables: Vec<TableMappingDto>,
    pub config: MigrationConfigDto,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DryRunTableResult {
    pub source_table: String,
    pub target_table: String,
    pub source_rows: usize,
    pub target_rows: usize,
    pub inserts: usize,
    pub updates: usize,
    pub deletes: usize,
    pub skips: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationProgressEvent {
    pub migration_id: String,
    pub table: String,
    pub processed_rows: usize,
    pub total_rows: usize,
    pub inserted: usize,
    pub updated: usize,
    pub deleted: usize,
    pub skipped: usize,
    pub errors: usize,
    pub status: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationResultDto {
    pub rows_inserted: usize,
    pub rows_updated: usize,
    pub rows_deleted: usize,
    pub rows_skipped: usize,
    pub error_count: usize,
    pub duration_ms: u64,
    pub status: String,
}

// ── Helpers ──────────────────────────────────────────────────────────

fn parse_mode(s: &str) -> MigrationMode {
    match s {
        "Mirror" => MigrationMode::Mirror,
        "AppendOnly" => MigrationMode::AppendOnly,
        "Merge" => MigrationMode::Merge,
        "SchemaOnly" => MigrationMode::SchemaOnly,
        _ => MigrationMode::Upsert,
    }
}

/// Filter a row to only include columns present in the target table.
fn filter_row_to_target(row: &Row, target_columns: &std::collections::HashSet<String>) -> Row {
    row.iter()
        .filter(|(k, _)| target_columns.contains(k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

fn build_migration_config(dto: &MigrationConfigDto, key_columns: &[String]) -> MigrationConfig {
    MigrationConfig {
        mode: parse_mode(&dto.mode),
        batch_size: dto.batch_size.max(1),
        key_columns: key_columns.to_vec(),
        ..Default::default()
    }
}

/// Sort table mappings so parent tables (FK targets) are processed before
/// children. Uses Kahn's algorithm for topological sort. Tables involved
/// in circular FK references are appended at the end in their original order.
fn sort_tables_by_fk(
    tables: &[TableMappingDto],
    fk_deps: &std::collections::HashMap<String, Vec<String>>,
) -> Vec<TableMappingDto> {
    let table_set: std::collections::HashSet<&str> =
        tables.iter().map(|t| t.target_table.as_str()).collect();

    // Map target_table name -> index in input slice
    let idx_map: std::collections::HashMap<&str, usize> = tables
        .iter()
        .enumerate()
        .map(|(i, t)| (t.target_table.as_str(), i))
        .collect();

    // Build in-degree counts (only for tables in our working set)
    let mut in_degree: std::collections::HashMap<&str, usize> = tables
        .iter()
        .map(|t| (t.target_table.as_str(), 0usize))
        .collect();

    for t in tables {
        if let Some(deps) = fk_deps.get(&t.target_table) {
            for dep in deps {
                if table_set.contains(dep.as_str()) && dep != &t.target_table {
                    *in_degree.entry(t.target_table.as_str()).or_insert(0) += 1;
                }
            }
        }
    }

    // Kahn's algorithm
    let mut queue: std::collections::VecDeque<&str> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&name, _)| name)
        .collect();

    // Sort the initial queue for deterministic order
    let mut q_vec: Vec<&str> = queue.drain(..).collect();
    q_vec.sort();
    queue.extend(q_vec);

    let mut sorted: Vec<usize> = Vec::with_capacity(tables.len());
    let mut visited: std::collections::HashSet<&str> = std::collections::HashSet::new();

    while let Some(current) = queue.pop_front() {
        if let Some(&idx) = idx_map.get(current) {
            sorted.push(idx);
            visited.insert(current);
        }
        // Decrease in-degree for tables that depend on `current`
        for t in tables {
            if visited.contains(t.target_table.as_str()) {
                continue;
            }
            if let Some(deps) = fk_deps.get(&t.target_table) {
                if deps.iter().any(|d| d == current) {
                    let deg = in_degree.entry(t.target_table.as_str()).or_insert(1);
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 {
                        queue.push_back(t.target_table.as_str());
                    }
                }
            }
        }
    }

    // Append any remaining tables (circular deps) in original order
    for (i, t) in tables.iter().enumerate() {
        if !visited.contains(t.target_table.as_str()) {
            sorted.push(i);
            log::warn!(
                "Table '{}' has circular FK dependencies; processing in original order",
                t.target_table
            );
        }
    }

    sorted.iter().map(|&i| tables[i].clone()).collect()
}

// ── Commands ─────────────────────────────────────────────────────────

/// Perform a dry-run: fetch data from source + target, run plan_migration,
/// return counts for each table.
#[tauri::command]
pub async fn dry_run(
    request: DryRunRequest,
    registry: State<'_, Arc<Mutex<ConnectionRegistry>>>,
) -> Result<Vec<DryRunTableResult>, String> {
    // Pre-fetch target schemas for validation and FK ordering
    let mut table_schemas: std::collections::HashMap<String, Vec<ColumnInfo>> =
        std::collections::HashMap::new();
    let mut fk_deps: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    {
        let reg = registry.lock().await;
        let tgt_conn = reg
            .get(&request.target_connection_id)
            .ok_or("Target connection not found")?;
        let guard = tgt_conn.lock().await;
        for table in &request.tables {
            if let Ok(info) = guard.get_table_info(&table.target_table).await {
                table_schemas.insert(table.target_table.clone(), info.columns.clone());
                let deps: Vec<String> = info
                    .constraints
                    .iter()
                    .filter(|c| c.constraint_type == ConstraintType::ForeignKey)
                    .filter_map(|c| c.referenced_table.clone())
                    .filter(|rt| rt != &table.target_table)
                    .collect();
                fk_deps.insert(table.target_table.clone(), deps);
            }
        }
    }

    let sorted_tables = sort_tables_by_fk(&request.tables, &fk_deps);
    let mut results = Vec::new();

    for table in &sorted_tables {
        // Fetch rows from source
        let source_rows = {
            let reg = registry.lock().await;
            let src_conn = reg
                .get(&request.source_connection_id)
                .ok_or("Source connection not found")?;
            let guard = src_conn.lock().await;
            guard
                .get_rows(&table.source_table, None, None)
                .await
                .map_err(|e| format!("Source fetch error ({}): {}", table.source_table, e))?
        };

        // Fetch rows from target
        let target_rows = {
            let reg = registry.lock().await;
            let tgt_conn = reg
                .get(&request.target_connection_id)
                .ok_or("Target connection not found")?;
            let guard = tgt_conn.lock().await;
            guard
                .get_rows(&table.target_table, None, None)
                .await
                .map_err(|e| format!("Target fetch error ({}): {}", table.target_table, e))?
        };

        // Use pre-fetched schema; fall back to row keys
        let schema = table_schemas
            .get(&table.target_table)
            .cloned()
            .unwrap_or_default();
        let target_columns: std::collections::HashSet<String> = if !schema.is_empty() {
            schema.iter().map(|c| c.name.clone()).collect()
        } else if let Some(first) = target_rows.first() {
            first.keys().cloned().collect()
        } else {
            std::collections::HashSet::new()
        };

        // Filter source rows to only target-compatible columns for accurate comparison
        let filtered_source: Vec<Row> = if !target_columns.is_empty() {
            source_rows
                .iter()
                .map(|r| filter_row_to_target(r, &target_columns))
                .collect()
        } else {
            source_rows.clone()
        };

        let mig_config = build_migration_config(&request.config, &table.key_columns);
        let plan = plan_migration(&filtered_source, &target_rows, &mig_config);

        // Check for schema incompatibilities and count rows that would be skipped
        let mut warnings = Vec::new();
        let source_col_names: std::collections::HashSet<&str> = filtered_source
            .first()
            .map(|r| r.keys().map(|k| k.as_str()).collect())
            .unwrap_or_default();
        for col in &schema {
            if !col.is_nullable
                && col.default_value.is_none()
                && !source_col_names.contains(col.name.as_str())
            {
                warnings.push(format!(
                    "Target column '{}' is NOT NULL without default and missing from source - inserts will be skipped",
                    col.name
                ));
            }
        }

        // Count rows that would be skipped by prepare_row_for_insert
        let sql_gen = SqlGenerator::new({
            let reg = registry.lock().await;
            let tgt_conn = reg
                .get(&request.target_connection_id)
                .ok_or("Target connection not found")?;
            let guard = tgt_conn.lock().await;
            guard.engine()
        });
        let mut valid_inserts = 0usize;
        let mut skipped_inserts = 0usize;
        for row in &plan.rows_to_insert {
            let (prepared, _) = sql_gen.prepare_row_for_insert(row, &schema);
            if prepared.is_some() {
                valid_inserts += 1;
            } else {
                skipped_inserts += 1;
            }
        }

        results.push(DryRunTableResult {
            source_table: table.source_table.clone(),
            target_table: table.target_table.clone(),
            source_rows: source_rows.len(),
            target_rows: target_rows.len(),
            inserts: valid_inserts,
            updates: plan.rows_to_update.len(),
            deletes: plan.rows_to_delete.len(),
            skips: plan.rows_to_review.len() + skipped_inserts,
            warnings,
        });
    }

    Ok(results)
}

/// Execute a real migration: fetch data, compute plan, generate SQL,
/// execute on target, emit progress events.
#[tauri::command]
pub async fn execute_migration(
    request: DryRunRequest,
    migration_id: String,
    app_handle: AppHandle,
    registry: State<'_, Arc<Mutex<ConnectionRegistry>>>,
    migration_state: State<'_, Arc<Mutex<MigrationState>>>,
) -> Result<MigrationResultDto, String> {
    // Set up cancellation token
    let cancel_token = CancellationToken::new();
    {
        let mut ms = migration_state.lock().await;
        ms.insert(migration_id.clone(), cancel_token.clone());
    }

    let start = std::time::Instant::now();
    let mut total_inserted = 0usize;
    let mut total_updated = 0usize;
    let mut total_deleted = 0usize;
    let mut total_skipped = 0usize;
    let mut total_errors = 0usize;
    let mut final_status = "completed".to_string();

    // ── Pre-fetch target schemas and FK dependencies for all tables ──
    let mut table_schemas: std::collections::HashMap<String, Vec<ColumnInfo>> =
        std::collections::HashMap::new();
    let mut fk_deps: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    {
        let reg = registry.lock().await;
        let tgt_conn = reg
            .get(&request.target_connection_id)
            .ok_or("Target connection not found")?;
        let guard = tgt_conn.lock().await;
        for table in &request.tables {
            if let Ok(info) = guard.get_table_info(&table.target_table).await {
                table_schemas.insert(table.target_table.clone(), info.columns.clone());
                let deps: Vec<String> = info
                    .constraints
                    .iter()
                    .filter(|c| c.constraint_type == ConstraintType::ForeignKey)
                    .filter_map(|c| c.referenced_table.clone())
                    .filter(|rt| rt != &table.target_table)
                    .collect();
                fk_deps.insert(table.target_table.clone(), deps);
            }
        }
    }

    // ── Sort tables by FK dependency order (parents first) ──
    let sorted_tables = sort_tables_by_fk(&request.tables, &fk_deps);

    for table in &sorted_tables {
        if cancel_token.is_cancelled() {
            final_status = "cancelled".to_string();
            break;
        }

        // Emit table-start event
        let _ = app_handle.emit(
            "migration:progress",
            MigrationProgressEvent {
                migration_id: migration_id.clone(),
                table: table.source_table.clone(),
                processed_rows: 0,
                total_rows: 0,
                inserted: total_inserted,
                updated: total_updated,
                deleted: total_deleted,
                skipped: total_skipped,
                errors: total_errors,
                status: "running".to_string(),
            },
        );

        // Fetch source rows
        let source_rows = {
            let reg = registry.lock().await;
            let src_conn = reg
                .get(&request.source_connection_id)
                .ok_or("Source connection not found")?;
            let guard = src_conn.lock().await;
            guard
                .get_rows(&table.source_table, None, None)
                .await
                .map_err(|e| format!("Source fetch error: {}", e))?
        };

        // Fetch target rows
        let target_rows = {
            let reg = registry.lock().await;
            let tgt_conn = reg
                .get(&request.target_connection_id)
                .ok_or("Target connection not found")?;
            let guard = tgt_conn.lock().await;
            guard
                .get_rows(&table.target_table, None, None)
                .await
                .map_err(|e| format!("Target fetch error: {}", e))?
        };

        // Determine target engine and target columns for SQL generation
        let (target_engine, target_columns, target_schema) = {
            let reg = registry.lock().await;
            let tgt_conn = reg
                .get(&request.target_connection_id)
                .ok_or("Target connection not found")?;
            let guard = tgt_conn.lock().await;
            let engine = guard.engine();
            // Use pre-fetched schema; fall back to row keys
            let schema = table_schemas
                .get(&table.target_table)
                .cloned()
                .unwrap_or_default();
            let cols: std::collections::HashSet<String> = if !schema.is_empty() {
                schema.iter().map(|c| c.name.clone()).collect()
            } else if let Some(first) = target_rows.first() {
                first.keys().cloned().collect()
            } else {
                std::collections::HashSet::new()
            };
            (engine, cols, schema)
        };

        let has_column_filter = !target_columns.is_empty();

        // Filter source rows to only target-compatible columns before planning
        let filtered_source: Vec<Row> = if has_column_filter {
            source_rows.iter().map(|r| filter_row_to_target(r, &target_columns)).collect()
        } else {
            source_rows.clone()
        };

        // Use compare_data directly to get changed_columns for partial updates
        let compare_config = DataCompareConfig {
            match_strategy: if table.key_columns.is_empty() {
                MatchStrategy::PrimaryKey
            } else {
                MatchStrategy::CompositeKey(table.key_columns.clone())
            },
            ignore_columns: Vec::new(),
            normalize_whitespace: false,
            case_insensitive: false,
            numeric_tolerance: None,
            null_equals_empty: false,
            use_hash_mode: false,
            batch_size: request.config.batch_size.max(1),
        };
        let diff = compare_data(&filtered_source, &target_rows, &compare_config);

        // Also build a plan for insert/delete decisions based on mode
        let mig_config = build_migration_config(&request.config, &table.key_columns);
        let plan = plan_migration(&filtered_source, &target_rows, &mig_config);

        let sql_gen = SqlGenerator::new(target_engine);
        let key_cols = &table.key_columns;

        // Execute inserts (with schema-aware validation)
        for row in &plan.rows_to_insert {
            if cancel_token.is_cancelled() {
                break;
            }
            // Validate row against target schema: truncate oversized strings,
            // skip rows missing required NOT NULL columns.
            let (prepared, prep_warnings) =
                sql_gen.prepare_row_for_insert(row, &target_schema);
            for w in &prep_warnings {
                log::warn!("Validation on {}: {}", table.target_table, w);
            }
            let insert_row = match prepared {
                Some(r) => r,
                None => {
                    total_skipped += 1;
                    log::warn!(
                        "Skipping insert on {}: row failed NOT NULL validation",
                        table.target_table
                    );
                    continue;
                }
            };
            let sql = sql_gen.generate_insert(&table.target_table, &insert_row);
            let exec_result = {
                let reg = registry.lock().await;
                let tgt_conn = reg
                    .get(&request.target_connection_id)
                    .ok_or("Target connection not found")?;
                let guard = tgt_conn.lock().await;
                guard.execute_query(&sql).await
            };
            match exec_result {
                Ok(_) => total_inserted += 1,
                Err(e) => {
                    total_errors += 1;
                    log::warn!("Insert error on {}: {:#}\nSQL: {}", table.target_table, e, sql);
                }
            }
        }

        // Execute updates using partial SET (only changed columns)
        for row_diff in &diff.updated_rows {
            if cancel_token.is_cancelled() {
                break;
            }
            let sql = sql_gen.generate_partial_update(
                &table.target_table,
                &row_diff.source_row,
                &row_diff.changed_columns,
                key_cols,
            );
            if sql.is_empty() {
                total_skipped += 1;
                continue;
            }
            let exec_result = {
                let reg = registry.lock().await;
                let tgt_conn = reg
                    .get(&request.target_connection_id)
                    .ok_or("Target connection not found")?;
                let guard = tgt_conn.lock().await;
                guard.execute_query(&sql).await
            };
            match exec_result {
                Ok(_) => total_updated += 1,
                Err(e) => {
                    total_errors += 1;
                    log::warn!("Update error on {}: {:#}\nSQL: {}", table.target_table, e, sql);
                }
            }
        }

        // Execute deletes
        for row in &plan.rows_to_delete {
            if cancel_token.is_cancelled() {
                break;
            }
            let sql = sql_gen.generate_delete(&table.target_table, row, key_cols);
            let exec_result = {
                let reg = registry.lock().await;
                let tgt_conn = reg
                    .get(&request.target_connection_id)
                    .ok_or("Target connection not found")?;
                let guard = tgt_conn.lock().await;
                guard.execute_query(&sql).await
            };
            match exec_result {
                Ok(_) => total_deleted += 1,
                Err(e) => {
                    total_errors += 1;
                    log::warn!("Delete error on {}: {}", table.target_table, e);
                }
            }
        }

        total_skipped += plan.rows_to_review.len();

        // Emit table-done event
        let _ = app_handle.emit(
            "migration:progress",
            MigrationProgressEvent {
                migration_id: migration_id.clone(),
                table: table.source_table.clone(),
                processed_rows: plan.rows_to_insert.len()
                    + plan.rows_to_update.len()
                    + plan.rows_to_delete.len(),
                total_rows: source_rows.len(),
                inserted: total_inserted,
                updated: total_updated,
                deleted: total_deleted,
                skipped: total_skipped,
                errors: total_errors,
                status: "completed".to_string(),
            },
        );
    }

    if cancel_token.is_cancelled() {
        final_status = "cancelled".to_string();
    }

    // Cleanup
    {
        let mut ms = migration_state.lock().await;
        ms.remove(&migration_id);
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(MigrationResultDto {
        rows_inserted: total_inserted,
        rows_updated: total_updated,
        rows_deleted: total_deleted,
        rows_skipped: total_skipped,
        error_count: total_errors,
        duration_ms,
        status: final_status,
    })
}

/// Cancel a running migration.
#[tauri::command]
pub async fn cancel_migration(
    migration_id: String,
    migration_state: State<'_, Arc<Mutex<MigrationState>>>,
) -> Result<bool, String> {
    let ms = migration_state.lock().await;
    Ok(ms.cancel(&migration_id))
}
