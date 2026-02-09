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

// ── Chat / Ollama ────────────────────────────────────────────────────

export interface OllamaModel {
  name: string;
  size: number;
  modified_at: string;
}

export interface RequiredModelStatus {
  name: string;
  present: boolean;
}

export interface OllamaStatus {
  running: boolean;
  models: OllamaModel[];
  required_models: RequiredModelStatus[];
  all_models_ready: boolean;
}

export interface ChatMessageDto {
  role: string;
  content: string;
}

export function checkOllamaStatus(): Promise<OllamaStatus> {
  return invoke<OllamaStatus>("check_ollama_status");
}

export function listOllamaModels(): Promise<OllamaModel[]> {
  return invoke<OllamaModel[]>("list_ollama_models");
}

export function sendChatMessage(
  model: string,
  messages: ChatMessageDto[],
  requestId: string,
): Promise<string> {
  return invoke<string>("send_chat_message", { model, messages, requestId });
}

export function pullModel(name: string): Promise<void> {
  return invoke<void>("pull_model", { name });
}

// ── App Database (persistence) ────────────────────────────────────────

export interface ConnectionProfileDto {
  id: string;
  name: string;
  engine: string;
  host: string | null;
  port: number | null;
  databaseName: string | null;
  username: string | null;
  filePath: string | null;
  readOnly: boolean;
  credentialKey: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface MigrationHistoryDto {
  id: string;
  sourceConnectionId: string | null;
  targetConnectionId: string | null;
  mode: string;
  status: string;
  configJson: string | null;
  resultJson: string | null;
  error: string | null;
  startedAt: string;
  completedAt: string | null;
  rowsInserted: number;
  rowsUpdated: number;
  rowsDeleted: number;
  rowsSkipped: number;
  errorCount: number;
  durationMs: number;
}

export interface ChatMessagePersistDto {
  id: string;
  role: string;
  content: string;
  model: string | null;
  timestamp: number;
}

export function saveConnectionProfile(
  profile: ConnectionProfileDto,
): Promise<void> {
  return invoke<void>("save_connection_profile", { profile });
}

export function getConnectionProfiles(): Promise<ConnectionProfileDto[]> {
  return invoke<ConnectionProfileDto[]>("get_connection_profiles");
}

export function deleteConnectionProfile(id: string): Promise<void> {
  return invoke<void>("delete_connection_profile", { id });
}

export function getSetting(key: string): Promise<string | null> {
  return invoke<string | null>("get_setting", { key });
}

export function setSetting(key: string, value: string): Promise<void> {
  return invoke<void>("set_setting", { key, value });
}

export function getAllSettings(): Promise<Record<string, string>> {
  return invoke<Record<string, string>>("get_all_settings");
}

export function getMigrationHistory(): Promise<MigrationHistoryDto[]> {
  return invoke<MigrationHistoryDto[]>("get_migration_history");
}

export function saveChatMessageToDb(
  message: ChatMessagePersistDto,
): Promise<void> {
  return invoke<void>("save_chat_message", { message });
}

export function loadChatMessagesFromDb(): Promise<ChatMessagePersistDto[]> {
  return invoke<ChatMessagePersistDto[]>("load_chat_messages");
}

export function clearChatMessagesInDb(): Promise<void> {
  return invoke<void>("clear_chat_messages");
}

// ── RAG Context (vectorized search) ───────────────────────────────────

export interface SearchResult {
  label: string;
  content: string;
  chunkType: string;
  score: number;
}

export function indexConnectionContext(
  connectionId: string,
  connectionName: string,
  engine: string,
): Promise<number> {
  return invoke<number>("index_connection_context", {
    connectionId,
    connectionName,
    engine,
  });
}

export function searchContext(
  query: string,
  topK?: number,
): Promise<SearchResult[]> {
  return invoke<SearchResult[]>("search_context", { query, topK });
}

export function indexAppContext(): Promise<void> {
  return invoke<void>("index_app_context");
}
