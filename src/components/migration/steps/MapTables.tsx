import { useState, useEffect } from "react";
import { useMigrationStore, type TableMapping } from "../../../stores/migrationStore";
import { useConnectionStore } from "../../../stores/connectionStore";
import {
  getTables,
  getRowCount,
  getTableInfo,
  connectDatabase,
  type ConnectionConfigDto,
} from "../../../lib/tauriCommands";

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
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [targetTables, setTargetTables] = useState<string[]>([]);
  // Store detected key columns per source table
  const [keyColumnsMap, setKeyColumnsMap] = useState<Record<string, string[]>>({});

  // Ensure a connection is registered in the backend before querying it
  const ensureConnected = async (connId: string) => {
    const conn = connections.find((c) => c.id === connId);
    if (!conn) throw new Error(`Connection profile '${connId}' not found`);
    const config: ConnectionConfigDto = {
      engine: conn.engine,
      host: conn.host,
      port: conn.port,
      database: conn.database,
      username: conn.username,
      password: conn.password,
      filePath: conn.filePath,
      readOnly: conn.readOnly,
    };
    await connectDatabase(connId, config);
  };

  // Fetch real tables from both connections
  useEffect(() => {
    if (!sourceConnectionId || !targetConnectionId || tableMappings.length > 0) return;

    let cancelled = false;
    setLoading(true);
    setError(null);

    (async () => {
      try {
        // Auto-connect both databases if not already registered
        await Promise.all([
          ensureConnected(sourceConnectionId),
          ensureConnected(targetConnectionId),
        ]);

        const [sourceTables, tgtTables] = await Promise.all([
          getTables(sourceConnectionId),
          getTables(targetConnectionId),
        ]);

        if (cancelled) return;
        setTargetTables(tgtTables);

        // Fetch row counts and key columns for source tables in parallel
        const rowCountPromises = sourceTables.map((t) =>
          getRowCount(sourceConnectionId, t).catch(() => 0),
        );
        const tableInfoPromises = sourceTables.map((t) =>
          getTableInfo(sourceConnectionId, t).catch(() => null),
        );

        const [rowCounts, tableInfos] = await Promise.all([
          Promise.all(rowCountPromises),
          Promise.all(tableInfoPromises),
        ]);

        if (cancelled) return;

        // Build key columns map from PK detection
        const keyCols: Record<string, string[]> = {};
        tableInfos.forEach((info, idx) => {
          if (!info) return;
          const pks = info.columns
            .filter((c) => c.isPrimaryKey)
            .map((c) => c.name);
          keyCols[sourceTables[idx]] = pks.length > 0 ? pks : ["id"];
        });
        setKeyColumnsMap(keyCols);

        const mappings: TableMapping[] = sourceTables.map((st, idx) => {
          const autoMatch = tgtTables.find(
            (tt) => tt.toLowerCase() === st.toLowerCase(),
          );
          return {
            id: crypto.randomUUID(),
            sourceTable: st,
            targetTable: autoMatch ?? "",
            included: !!autoMatch,
            estimatedRows: Number(rowCounts[idx]) || 0,
          };
        });
        setTableMappings(mappings);
      } catch (err) {
        if (!cancelled) {
          setError(String(err));
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();

    return () => {
      cancelled = true;
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sourceConnectionId, targetConnectionId, tableMappings.length, setTableMappings]);

  // Store key columns on the migration store for later use
  // We expose it via a module-level getter that DryRun/Execute can access
  // For now, store it in window for cross-component access
  useEffect(() => {
    (window as any).__upsert_key_columns = keyColumnsMap;
  }, [keyColumnsMap]);

  if (loading) {
    return (
      <div className="flex flex-col items-center justify-center py-16">
        <div className="h-6 w-6 animate-spin rounded-full border-2 border-blue-500 border-t-transparent" />
        <p className="mt-3 text-xs text-neutral-500 dark:text-neutral-400">
          Fetching tables from databases...
        </p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center py-16">
        <p className="text-sm font-medium text-red-500">{error}</p>
        <p className="mt-1 text-xs text-neutral-400">
          Make sure both connections are tested and connected.
        </p>
      </div>
    );
  }

  if (tableMappings.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-16">
        <p className="text-sm font-medium text-neutral-500 dark:text-neutral-400">
          No tables found
        </p>
        <p className="mt-1 text-xs text-neutral-400 dark:text-neutral-500">
          Make sure connections are established and databases contain tables.
        </p>
      </div>
    );
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
              targetTables={targetTables}
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
