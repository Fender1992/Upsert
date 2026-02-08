import { useState, useCallback } from "react";
import { useMigrationStore, type TableMapping } from "../../../stores/migrationStore";
import { useConnectionStore } from "../../../stores/connectionStore";

// Simulated table lists -- in production, fetched from the Tauri backend
const DEMO_SOURCE_TABLES = [
  "users",
  "orders",
  "products",
  "categories",
  "order_items",
  "reviews",
  "inventory",
  "shipping_addresses",
];
const DEMO_TARGET_TABLES = [
  "users",
  "orders",
  "products",
  "categories",
  "order_items",
  "reviews",
  "stock",
  "addresses",
];

export default function MapTables() {
  const {
    tableMappings,
    setTableMappings,
    updateTableMapping,
    sourceConnectionId,
    targetConnectionId,
  } = useMigrationStore();
  const { connections } = useConnectionStore();

  const sourceConn = connections.find((c) => c.id === sourceConnectionId);
  const targetConn = connections.find((c) => c.id === targetConnectionId);

  const [search, setSearch] = useState("");

  // Auto-generate mappings on first load if empty
  const initMappings = useCallback(() => {
    const mappings: TableMapping[] = DEMO_SOURCE_TABLES.map((st) => {
      const autoMatch = DEMO_TARGET_TABLES.find(
        (tt) => tt.toLowerCase() === st.toLowerCase(),
      );
      return {
        id: crypto.randomUUID(),
        sourceTable: st,
        targetTable: autoMatch ?? "",
        included: !!autoMatch,
        estimatedRows: Math.floor(Math.random() * 50000) + 100,
      };
    });
    setTableMappings(mappings);
  }, [setTableMappings]);

  // Initialize if empty
  if (tableMappings.length === 0) {
    initMappings();
    return null;
  }

  const filtered = search
    ? tableMappings.filter(
        (m) =>
          m.sourceTable.toLowerCase().includes(search.toLowerCase()) ||
          m.targetTable.toLowerCase().includes(search.toLowerCase()),
      )
    : tableMappings;

  const mappedCount = tableMappings.filter(
    (m) => m.included && m.targetTable,
  ).length;
  const unmappedCount = tableMappings.filter((m) => !m.targetTable).length;

  return (
    <div className="space-y-4">
      <div>
        <h3 className="text-sm font-semibold text-neutral-800 dark:text-neutral-100">
          Map Tables
        </h3>
        <p className="mt-1 text-xs text-neutral-500 dark:text-neutral-400">
          Map source tables ({sourceConn?.name ?? "Source"}) to target tables (
          {targetConn?.name ?? "Target"}). Tables are auto-matched by name.
        </p>
      </div>

      {/* Summary badges */}
      <div className="flex items-center gap-3">
        <span className="rounded-full bg-green-100 px-2.5 py-0.5 text-[10px] font-medium text-green-700 dark:bg-green-900/40 dark:text-green-400">
          {mappedCount} mapped
        </span>
        {unmappedCount > 0 && (
          <span className="rounded-full bg-amber-100 px-2.5 py-0.5 text-[10px] font-medium text-amber-700 dark:bg-amber-900/40 dark:text-amber-400">
            {unmappedCount} unmapped
          </span>
        )}
        <span className="rounded-full bg-neutral-100 px-2.5 py-0.5 text-[10px] font-medium text-neutral-600 dark:bg-neutral-700 dark:text-neutral-400">
          {tableMappings.length} total
        </span>

        <div className="ml-auto">
          <input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Filter tables..."
            className="input-field w-48"
          />
        </div>
      </div>

      {/* Mapping table */}
      <div className="overflow-hidden rounded-lg border border-neutral-200 dark:border-neutral-700">
        {/* Header */}
        <div className="flex items-center gap-2 border-b border-neutral-200 bg-neutral-50 px-4 py-2 text-[10px] font-semibold uppercase tracking-wider text-neutral-500 dark:border-neutral-700 dark:bg-neutral-800/50 dark:text-neutral-400">
          <div className="w-8" />
          <div className="flex-1">Source Table</div>
          <div className="w-8 text-center">
            <svg
              className="mx-auto h-3 w-3 text-neutral-400"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={2}
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                d="M13.5 4.5L21 12m0 0l-7.5 7.5M21 12H3"
              />
            </svg>
          </div>
          <div className="flex-1">Target Table</div>
          <div className="w-24 text-right">Est. Rows</div>
        </div>

        {/* Rows */}
        <div className="max-h-[400px] overflow-y-auto">
          {filtered.map((mapping) => (
            <MappingRow
              key={mapping.id}
              mapping={mapping}
              targetTables={DEMO_TARGET_TABLES}
              onToggle={() =>
                updateTableMapping(mapping.id, {
                  included: !mapping.included,
                })
              }
              onTargetChange={(target) =>
                updateTableMapping(mapping.id, { targetTable: target })
              }
            />
          ))}
        </div>

        {filtered.length === 0 && (
          <div className="px-4 py-8 text-center text-xs text-neutral-400">
            No tables match the current filter.
          </div>
        )}
      </div>
    </div>
  );
}

function MappingRow({
  mapping,
  targetTables,
  onToggle,
  onTargetChange,
}: {
  mapping: TableMapping;
  targetTables: string[];
  onToggle: () => void;
  onTargetChange: (target: string) => void;
}) {
  const isMapped = !!mapping.targetTable;

  return (
    <div
      className={`flex items-center gap-2 border-b border-neutral-100 px-4 py-2 last:border-b-0 dark:border-neutral-800 ${
        !mapping.included ? "opacity-50" : ""
      }`}
    >
      {/* Include checkbox */}
      <div className="w-8">
        <input
          type="checkbox"
          checked={mapping.included}
          onChange={onToggle}
          className="rounded border-neutral-300 dark:border-neutral-600"
        />
      </div>

      {/* Source table */}
      <div className="flex-1">
        <span className="text-xs font-mono text-neutral-700 dark:text-neutral-300">
          {mapping.sourceTable}
        </span>
      </div>

      {/* Arrow */}
      <div className="w-8 text-center">
        <span
          className={`text-xs ${isMapped ? "text-green-500" : "text-neutral-300 dark:text-neutral-600"}`}
        >
          {isMapped ? "->" : "--"}
        </span>
      </div>

      {/* Target table dropdown */}
      <div className="flex-1">
        <select
          value={mapping.targetTable}
          onChange={(e) => onTargetChange(e.target.value)}
          className="input-field text-xs"
        >
          <option value="">(unmapped)</option>
          {targetTables.map((tt) => (
            <option key={tt} value={tt}>
              {tt}
            </option>
          ))}
        </select>
      </div>

      {/* Est. Rows */}
      <div className="w-24 text-right text-xs tabular-nums text-neutral-500 dark:text-neutral-400">
        {mapping.estimatedRows.toLocaleString()}
      </div>
    </div>
  );
}
