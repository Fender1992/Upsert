import { useState, useCallback } from "react";
import {
  useMigrationStore,
  type DryRunResult,
} from "../../../stores/migrationStore";
import { useConnectionStore } from "../../../stores/connectionStore";

export default function DryRun() {
  const {
    tableMappings,
    config,
    sourceConnectionId,
    targetConnectionId,
    dryRunResult,
    setDryRunResult,
    setStatus,
    transformRules,
  } = useMigrationStore();
  const { connections } = useConnectionStore();

  const sourceConn = connections.find((c) => c.id === sourceConnectionId);
  const targetConn = connections.find((c) => c.id === targetConnectionId);
  const includedTables = tableMappings.filter(
    (m) => m.included && m.targetTable,
  );

  const [isRunning, setIsRunning] = useState(false);

  const totalEstimatedRows = includedTables.reduce(
    (sum, m) => sum + m.estimatedRows,
    0,
  );
  const rulesCount = transformRules.length;

  const handleRunDryRun = useCallback(async () => {
    setIsRunning(true);
    setStatus("dry-run");

    // Simulate dry run
    await new Promise((resolve) => setTimeout(resolve, 2000));

    const result: DryRunResult = {
      tableSummaries: includedTables.map((t) => {
        const rows = t.estimatedRows;
        const inserts = Math.floor(rows * 0.6);
        const updates = Math.floor(rows * 0.3);
        const deletes = config.mode === "Mirror" ? Math.floor(rows * 0.05) : 0;
        const skips = rows - inserts - updates - deletes;
        return {
          tableId: t.id,
          tableName: t.sourceTable,
          estimatedRows: rows,
          estimatedInserts: inserts,
          estimatedUpdates: updates,
          estimatedDeletes: deletes,
          estimatedSkips: skips,
        };
      }),
      warnings: [
        includedTables.length > 5
          ? `Large migration: ${includedTables.length} tables selected`
          : "",
        totalEstimatedRows > 10000
          ? `High row count: ${totalEstimatedRows.toLocaleString()} estimated rows`
          : "",
        config.transactionMode === "None"
          ? "No transaction wrapping -- partial failures cannot be rolled back"
          : "",
        sourceConnectionId === targetConnectionId
          ? "Source and target are the same connection"
          : "",
      ].filter(Boolean),
      errors: [],
      totalEstimatedTime: Math.ceil(totalEstimatedRows / config.batchSize) * 50,
    };

    setDryRunResult(result);
    setStatus("configuring");
    setIsRunning(false);
  }, [
    includedTables,
    config,
    totalEstimatedRows,
    sourceConnectionId,
    targetConnectionId,
    setDryRunResult,
    setStatus,
  ]);

  return (
    <div className="space-y-4">
      <div>
        <h3 className="text-sm font-semibold text-neutral-800 dark:text-neutral-100">
          Dry Run Preview
        </h3>
        <p className="mt-1 text-xs text-neutral-500 dark:text-neutral-400">
          Review the migration plan and run a simulation to validate before
          executing.
        </p>
      </div>

      {/* Summary cards */}
      <div className="grid grid-cols-4 gap-3">
        <SummaryCard
          label="Tables"
          value={String(includedTables.length)}
          sublabel="to migrate"
        />
        <SummaryCard
          label="Estimated Rows"
          value={totalEstimatedRows.toLocaleString()}
          sublabel="total"
        />
        <SummaryCard label="Mode" value={config.mode} sublabel={config.conflictResolution} />
        <SummaryCard
          label="Transform Rules"
          value={String(rulesCount)}
          sublabel="configured"
        />
      </div>

      {/* Connection summary */}
      <div className="flex items-center gap-3 rounded-lg border border-neutral-200 bg-white px-4 py-3 dark:border-neutral-700 dark:bg-neutral-800">
        <div className="flex-1">
          <div className="text-[10px] uppercase tracking-wider text-neutral-400">
            Source
          </div>
          <div className="text-xs font-medium text-neutral-800 dark:text-neutral-100">
            {sourceConn?.name ?? "Not selected"}
          </div>
          <div className="text-[11px] text-neutral-400">
            {sourceConn?.engine}
          </div>
        </div>
        <svg
          className="h-5 w-5 text-neutral-300 dark:text-neutral-600"
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
        <div className="flex-1 text-right">
          <div className="text-[10px] uppercase tracking-wider text-neutral-400">
            Target
          </div>
          <div className="text-xs font-medium text-neutral-800 dark:text-neutral-100">
            {targetConn?.name ?? "Not selected"}
          </div>
          <div className="text-[11px] text-neutral-400">
            {targetConn?.engine}
          </div>
        </div>
      </div>

      {/* Table list */}
      <div className="overflow-hidden rounded-lg border border-neutral-200 dark:border-neutral-700">
        <div className="flex items-center border-b border-neutral-200 bg-neutral-50 px-4 py-2 text-[10px] font-semibold uppercase tracking-wider text-neutral-500 dark:border-neutral-700 dark:bg-neutral-800/50 dark:text-neutral-400">
          <div className="flex-1">Table</div>
          <div className="w-24 text-right">Est. Rows</div>
          {dryRunResult && (
            <>
              <div className="w-20 text-right">Inserts</div>
              <div className="w-20 text-right">Updates</div>
              <div className="w-20 text-right">Deletes</div>
              <div className="w-20 text-right">Skips</div>
            </>
          )}
        </div>
        <div className="max-h-[250px] overflow-y-auto">
          {includedTables.map((t) => {
            const summary = dryRunResult?.tableSummaries.find(
              (s) => s.tableId === t.id,
            );
            return (
              <div
                key={t.id}
                className="flex items-center border-b border-neutral-100 px-4 py-1.5 text-xs last:border-b-0 dark:border-neutral-800"
              >
                <div className="flex-1 font-mono text-neutral-700 dark:text-neutral-300">
                  {t.sourceTable}
                  <span className="mx-1 text-neutral-300 dark:text-neutral-600">
                    -&gt;
                  </span>
                  <span className="text-neutral-500 dark:text-neutral-400">
                    {t.targetTable}
                  </span>
                </div>
                <div className="w-24 text-right tabular-nums text-neutral-500 dark:text-neutral-400">
                  {t.estimatedRows.toLocaleString()}
                </div>
                {summary && (
                  <>
                    <div className="w-20 text-right tabular-nums text-green-600 dark:text-green-400">
                      +{summary.estimatedInserts.toLocaleString()}
                    </div>
                    <div className="w-20 text-right tabular-nums text-amber-600 dark:text-amber-400">
                      ~{summary.estimatedUpdates.toLocaleString()}
                    </div>
                    <div className="w-20 text-right tabular-nums text-red-600 dark:text-red-400">
                      -{summary.estimatedDeletes.toLocaleString()}
                    </div>
                    <div className="w-20 text-right tabular-nums text-neutral-400">
                      {summary.estimatedSkips.toLocaleString()}
                    </div>
                  </>
                )}
              </div>
            );
          })}
        </div>
      </div>

      {/* Warnings */}
      {dryRunResult && dryRunResult.warnings.length > 0 && (
        <div className="space-y-1">
          <h4 className="text-xs font-semibold text-amber-600 dark:text-amber-400">
            Warnings ({dryRunResult.warnings.length})
          </h4>
          {dryRunResult.warnings.map((w, i) => (
            <div
              key={i}
              className="flex items-start gap-2 rounded border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-700 dark:border-amber-800 dark:bg-amber-950/20 dark:text-amber-400"
            >
              <svg
                className="mt-0.5 h-3.5 w-3.5 shrink-0"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                strokeWidth={2}
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126ZM12 15.75h.007v.008H12v-.008Z"
                />
              </svg>
              {w}
            </div>
          ))}
        </div>
      )}

      {/* Errors */}
      {dryRunResult && dryRunResult.errors.length > 0 && (
        <div className="space-y-1">
          <h4 className="text-xs font-semibold text-red-600 dark:text-red-400">
            Errors ({dryRunResult.errors.length})
          </h4>
          {dryRunResult.errors.map((err, i) => (
            <div
              key={i}
              className="flex items-start gap-2 rounded border border-red-200 bg-red-50 px-3 py-2 text-xs text-red-700 dark:border-red-800 dark:bg-red-950/20 dark:text-red-400"
            >
              <svg
                className="mt-0.5 h-3.5 w-3.5 shrink-0"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                strokeWidth={2}
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126ZM12 15.75h.007v.008H12v-.008Z"
                />
              </svg>
              {err}
            </div>
          ))}
        </div>
      )}

      {/* Estimated time */}
      {dryRunResult && (
        <div className="text-xs text-neutral-500 dark:text-neutral-400">
          Estimated migration time:{" "}
          <span className="font-medium text-neutral-700 dark:text-neutral-300">
            {dryRunResult.totalEstimatedTime < 1000
              ? `${dryRunResult.totalEstimatedTime}ms`
              : `${(dryRunResult.totalEstimatedTime / 1000).toFixed(1)}s`}
          </span>
        </div>
      )}

      {/* Run Dry Run button */}
      <button
        onClick={handleRunDryRun}
        disabled={isRunning || includedTables.length === 0}
        className="rounded bg-blue-600 px-4 py-2 text-xs font-medium text-white hover:bg-blue-700 disabled:opacity-50"
      >
        {isRunning
          ? "Running Dry Run..."
          : dryRunResult
            ? "Re-run Dry Run"
            : "Run Dry Run"}
      </button>
    </div>
  );
}

function SummaryCard({
  label,
  value,
  sublabel,
}: {
  label: string;
  value: string;
  sublabel: string;
}) {
  return (
    <div className="rounded-lg border border-neutral-200 bg-white px-4 py-3 dark:border-neutral-700 dark:bg-neutral-800">
      <div className="text-[10px] uppercase tracking-wider text-neutral-400">
        {label}
      </div>
      <div className="mt-1 text-lg font-bold tabular-nums text-neutral-800 dark:text-neutral-100">
        {value}
      </div>
      <div className="text-[11px] text-neutral-400">{sublabel}</div>
    </div>
  );
}
