export type ObjectType = "Table" | "Column" | "Index" | "Constraint";
export type ChangeType = "Added" | "Removed" | "Modified" | "Unchanged";

export interface SchemaChange {
  objectType: ObjectType;
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

export type RowStatus = "matched" | "inserted" | "updated" | "deleted" | "error";

export interface DataDiffRow {
  _rowId: string;
  _status: RowStatus;
  _changedColumns?: string[];
  [column: string]: unknown;
}

export interface DataDiffDataSet {
  columns: string[];
  rows: DataDiffRow[];
  totalRows: number;
}

export type ChangeTypeFilter = ChangeType | "all";
export type ObjectTypeFilter = ObjectType | "all";
export type RowStatusFilter = RowStatus | "all";
