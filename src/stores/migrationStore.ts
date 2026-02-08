import { create } from "zustand";

export type MigrationMode =
  | "Upsert"
  | "Mirror"
  | "AppendOnly"
  | "Merge"
  | "SchemaOnly";

export type ConflictResolution =
  | "SourceWins"
  | "TargetWins"
  | "NewestWins"
  | "ManualReview";

export type MigrationStatus =
  | "idle"
  | "configuring"
  | "dry-run"
  | "running"
  | "paused"
  | "completed"
  | "failed"
  | "cancelled";

export interface MigrationConfig {
  mode: MigrationMode;
  conflictResolution: ConflictResolution;
  batchSize: number;
  transactionMode: "PerBatch" | "WholeMigration" | "None";
  retryCount: number;
  autoRollback: boolean;
  backupBeforeMigrate: boolean;
  dryRun: boolean;
}

export interface MigrationProgress {
  totalRows: number;
  processedRows: number;
  insertedRows: number;
  updatedRows: number;
  deletedRows: number;
  skippedRows: number;
  errorCount: number;
  currentBatch: number;
  totalBatches: number;
  elapsedMs: number;
}

export interface TableMapping {
  id: string;
  sourceTable: string;
  targetTable: string;
  included: boolean;
  estimatedRows: number;
}

export type TransformRuleType =
  | "rename"
  | "type_cast"
  | "value_map"
  | "default_for_null"
  | "drop_column";

export interface TransformRule {
  id: string;
  tableId: string;
  sourceColumn: string;
  targetColumn: string;
  ruleType: TransformRuleType;
  config: Record<string, string>;
  order: number;
}

export interface TableProgress {
  tableId: string;
  tableName: string;
  status: "pending" | "running" | "completed" | "failed" | "skipped";
  totalRows: number;
  processedRows: number;
  errors: MigrationError[];
}

export interface MigrationError {
  id: string;
  tableId: string;
  rowIndex?: number;
  message: string;
  timestamp: number;
}

export interface DryRunResult {
  tableSummaries: Array<{
    tableId: string;
    tableName: string;
    estimatedRows: number;
    estimatedInserts: number;
    estimatedUpdates: number;
    estimatedDeletes: number;
    estimatedSkips: number;
  }>;
  warnings: string[];
  errors: string[];
  totalEstimatedTime: number;
}

interface MigrationState {
  config: MigrationConfig;
  status: MigrationStatus;
  progress: MigrationProgress | null;
  wizardStep: number;
  error: string | null;

  sourceConnectionId: string | null;
  targetConnectionId: string | null;
  tableMappings: TableMapping[];
  transformRules: TransformRule[];
  tableProgress: TableProgress[];
  dryRunResult: DryRunResult | null;
  elapsedMs: number;

  setConfig: (config: Partial<MigrationConfig>) => void;
  setStatus: (status: MigrationStatus) => void;
  setProgress: (progress: MigrationProgress | null) => void;
  setWizardStep: (step: number) => void;
  setError: (error: string | null) => void;
  setSourceConnection: (id: string | null) => void;
  setTargetConnection: (id: string | null) => void;
  setTableMappings: (mappings: TableMapping[]) => void;
  updateTableMapping: (id: string, updates: Partial<TableMapping>) => void;
  addTransformRule: (rule: TransformRule) => void;
  removeTransformRule: (id: string) => void;
  updateTransformRule: (id: string, updates: Partial<TransformRule>) => void;
  reorderTransformRule: (id: string, newOrder: number) => void;
  setTableProgress: (progress: TableProgress[]) => void;
  updateTableProgress: (tableId: string, updates: Partial<TableProgress>) => void;
  setDryRunResult: (result: DryRunResult | null) => void;
  setElapsedMs: (ms: number) => void;
  startMigration: () => void;
  cancelMigration: () => void;
  reset: () => void;
}

const defaultConfig: MigrationConfig = {
  mode: "Upsert",
  conflictResolution: "SourceWins",
  batchSize: 1000,
  transactionMode: "PerBatch",
  retryCount: 3,
  autoRollback: true,
  backupBeforeMigrate: true,
  dryRun: false,
};

export const useMigrationStore = create<MigrationState>()((set) => ({
  config: defaultConfig,
  status: "idle",
  progress: null,
  wizardStep: 1,
  error: null,

  sourceConnectionId: null,
  targetConnectionId: null,
  tableMappings: [],
  transformRules: [],
  tableProgress: [],
  dryRunResult: null,
  elapsedMs: 0,

  setConfig: (updates) =>
    set((state) => ({ config: { ...state.config, ...updates } })),
  setStatus: (status) => set({ status }),
  setProgress: (progress) => set({ progress }),
  setWizardStep: (wizardStep) => set({ wizardStep }),
  setError: (error) => set({ error }),

  setSourceConnection: (sourceConnectionId) => set({ sourceConnectionId }),
  setTargetConnection: (targetConnectionId) => set({ targetConnectionId }),

  setTableMappings: (tableMappings) => set({ tableMappings }),
  updateTableMapping: (id, updates) =>
    set((state) => ({
      tableMappings: state.tableMappings.map((m) =>
        m.id === id ? { ...m, ...updates } : m,
      ),
    })),

  addTransformRule: (rule) =>
    set((state) => ({
      transformRules: [...state.transformRules, rule],
    })),
  removeTransformRule: (id) =>
    set((state) => ({
      transformRules: state.transformRules.filter((r) => r.id !== id),
    })),
  updateTransformRule: (id, updates) =>
    set((state) => ({
      transformRules: state.transformRules.map((r) =>
        r.id === id ? { ...r, ...updates } : r,
      ),
    })),
  reorderTransformRule: (id, newOrder) =>
    set((state) => ({
      transformRules: state.transformRules.map((r) =>
        r.id === id ? { ...r, order: newOrder } : r,
      ),
    })),

  setTableProgress: (tableProgress) => set({ tableProgress }),
  updateTableProgress: (tableId, updates) =>
    set((state) => ({
      tableProgress: state.tableProgress.map((tp) =>
        tp.tableId === tableId ? { ...tp, ...updates } : tp,
      ),
    })),

  setDryRunResult: (dryRunResult) => set({ dryRunResult }),
  setElapsedMs: (elapsedMs) => set({ elapsedMs }),

  startMigration: () =>
    set((state) => ({
      status: "running",
      progress: {
        totalRows: state.tableMappings
          .filter((m) => m.included)
          .reduce((sum, m) => sum + m.estimatedRows, 0),
        processedRows: 0,
        insertedRows: 0,
        updatedRows: 0,
        deletedRows: 0,
        skippedRows: 0,
        errorCount: 0,
        currentBatch: 0,
        totalBatches: 0,
        elapsedMs: 0,
      },
      elapsedMs: 0,
      tableProgress: state.tableMappings
        .filter((m) => m.included)
        .map((m) => ({
          tableId: m.id,
          tableName: m.sourceTable,
          status: "pending" as const,
          totalRows: m.estimatedRows,
          processedRows: 0,
          errors: [],
        })),
    })),

  cancelMigration: () => set({ status: "cancelled" }),

  reset: () =>
    set({
      config: defaultConfig,
      status: "idle",
      progress: null,
      wizardStep: 1,
      error: null,
      sourceConnectionId: null,
      targetConnectionId: null,
      tableMappings: [],
      transformRules: [],
      tableProgress: [],
      dryRunResult: null,
      elapsedMs: 0,
    }),
}));
