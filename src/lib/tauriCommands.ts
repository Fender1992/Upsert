import { invoke } from "@tauri-apps/api/core";
import type { DatabaseEngine } from "../stores/connectionStore";

// ── Request / Response types ─────────────────────────────────────────

export interface ConnectionConfigDto {
  engine: DatabaseEngine;
  host?: string;
  port?: number;
  database?: string;
  username?: string;
  password?: string;
  connectionString?: string;
  filePath?: string;
  readOnly: boolean;
}

export interface TableMappingDto {
  sourceTable: string;
  targetTable: string;
  keyColumns: string[];
}

export interface MigrationConfigDto {
  mode: string;
  conflictResolution: string;
  batchSize: number;
}

export interface DryRunRequest {
  sourceConnectionId: string;
  targetConnectionId: string;
  tables: TableMappingDto[];
  config: MigrationConfigDto;
}

export interface DryRunTableResult {
  sourceTable: string;
  targetTable: string;
  sourceRows: number;
  targetRows: number;
  inserts: number;
  updates: number;
  deletes: number;
  skips: number;
  warnings: string[];
}

export interface ColumnInfo {
  name: string;
  dataType: string;
  isNullable: boolean;
  isPrimaryKey: boolean;
  maxLength: number | null;
  precision: number | null;
  scale: number | null;
  defaultValue: string | null;
  ordinalPosition: number;
}

export interface TableInfo {
  schemaName: string;
  tableName: string;
  columns: ColumnInfo[];
  indexes: unknown[];
  constraints: unknown[];
  rowCount: number | null;
}

export interface MigrationProgressEvent {
  migrationId: string;
  table: string;
  processedRows: number;
  totalRows: number;
  inserted: number;
  updated: number;
  deleted: number;
  skipped: number;
  errors: number;
  status: string;
}

export interface MigrationResultDto {
  rowsInserted: number;
  rowsUpdated: number;
  rowsDeleted: number;
  rowsSkipped: number;
  errorCount: number;
  durationMs: number;
  status: string;
}

// ── Typed invoke wrappers ────────────────────────────────────────────

export function testConnection(config: ConnectionConfigDto): Promise<boolean> {
  return invoke<boolean>("test_connection", { config });
}

export function connectDatabase(
  id: string,
  config: ConnectionConfigDto,
): Promise<void> {
  return invoke<void>("connect_database", { id, config });
}

export function disconnectDatabase(id: string): Promise<void> {
  return invoke<void>("disconnect_database", { id });
}

export function getTables(connectionId: string): Promise<string[]> {
  return invoke<string[]>("get_tables", { connectionId });
}

export function getTableInfo(
  connectionId: string,
  tableName: string,
): Promise<TableInfo> {
  return invoke<TableInfo>("get_table_info", { connectionId, tableName });
}

export function getRowCount(
  connectionId: string,
  tableName: string,
): Promise<number> {
  return invoke<number>("get_row_count", { connectionId, tableName });
}

export function dryRun(
  request: DryRunRequest,
): Promise<DryRunTableResult[]> {
  return invoke<DryRunTableResult[]>("dry_run", { request });
}

export function executeMigration(
  request: DryRunRequest,
  migrationId: string,
): Promise<MigrationResultDto> {
  return invoke<MigrationResultDto>("execute_migration", {
    request,
    migrationId,
  });
}

export function cancelMigration(migrationId: string): Promise<boolean> {
  return invoke<boolean>("cancel_migration", { migrationId });
}
