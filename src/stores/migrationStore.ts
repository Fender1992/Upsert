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

interface MigrationState {
  config: MigrationConfig;
  status: MigrationStatus;
  progress: MigrationProgress | null;
  wizardStep: number;
  error: string | null;

  setConfig: (config: Partial<MigrationConfig>) => void;
  setStatus: (status: MigrationStatus) => void;
  setProgress: (progress: MigrationProgress | null) => void;
  setWizardStep: (step: number) => void;
  setError: (error: string | null) => void;
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
  wizardStep: 0,
  error: null,

  setConfig: (updates) =>
    set((state) => ({ config: { ...state.config, ...updates } })),
  setStatus: (status) => set({ status }),
  setProgress: (progress) => set({ progress }),
  setWizardStep: (wizardStep) => set({ wizardStep }),
  setError: (error) => set({ error }),
  reset: () =>
    set({
      config: defaultConfig,
      status: "idle",
      progress: null,
      wizardStep: 0,
      error: null,
    }),
}));
