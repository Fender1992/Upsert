import { create } from "zustand";

export type ChangeType = "Added" | "Removed" | "Modified" | "Unchanged";

export interface SchemaChange {
  objectType: string;
  objectName: string;
  changeType: ChangeType;
  details: Array<{
    property: string;
    sourceValue?: string;
    targetValue?: string;
  }>;
}

export interface SchemaDiffResult {
  sourceDatabase: string;
  targetDatabase: string;
  changes: SchemaChange[];
  summary: {
    additions: number;
    removals: number;
    modifications: number;
    unchanged: number;
  };
}

export interface DataDiffResult {
  sourceTable: string;
  targetTable: string;
  matchedRows: number;
  insertedCount: number;
  updatedCount: number;
  deletedCount: number;
  errorCount: number;
}

interface ComparisonState {
  schemaDiff: SchemaDiffResult | null;
  dataDiff: DataDiffResult | null;
  isComparing: boolean;
  progress: number;
  error: string | null;

  setSchemaDiff: (diff: SchemaDiffResult | null) => void;
  setDataDiff: (diff: DataDiffResult | null) => void;
  setComparing: (comparing: boolean) => void;
  setProgress: (progress: number) => void;
  setError: (error: string | null) => void;
  reset: () => void;
}

export const useComparisonStore = create<ComparisonState>()((set) => ({
  schemaDiff: null,
  dataDiff: null,
  isComparing: false,
  progress: 0,
  error: null,

  setSchemaDiff: (schemaDiff) => set({ schemaDiff }),
  setDataDiff: (dataDiff) => set({ dataDiff }),
  setComparing: (isComparing) => set({ isComparing }),
  setProgress: (progress) => set({ progress }),
  setError: (error) => set({ error }),
  reset: () =>
    set({
      schemaDiff: null,
      dataDiff: null,
      isComparing: false,
      progress: 0,
      error: null,
    }),
}));
