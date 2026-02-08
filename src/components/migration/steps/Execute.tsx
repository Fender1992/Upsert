import { useState, useEffect, useCallback, useRef } from "react";
import { useMigrationStore } from "../../../stores/migrationStore";
import { useUiStore } from "../../../stores/uiStore";

export default function Execute() {
  const {
    status,
    progress,
    tableProgress,
    startMigration,
    cancelMigration,
    setStatus,
    setProgress,
    updateTableProgress,
    setElapsedMs,
    elapsedMs,
    config,
    tableMappings,
  } = useMigrationStore();
  const { appendLog, addNotification } = useUiStore();

  const [showErrors, setShowErrors] = useState(false);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const simulationRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const isRunning = status === "running";
  const isCompleted = status === "completed";
  const isFailed = status === "failed";
  const isCancelled = status === "cancelled";
  const isIdle = status === "idle" || status === "configuring";

  const includedTables = tableMappings.filter(
    (m) => m.included && m.targetTable,
  );

  // Elapsed time timer
  useEffect(() => {
    if (isRunning) {
      const start = Date.now() - elapsedMs;
      timerRef.current = setInterval(() => {
        setElapsedMs(Date.now() - start);
      }, 100);
    }
    return () => {
      if (timerRef.current) clearInterval(timerRef.current);
    };
  }, [isRunning, elapsedMs, setElapsedMs]);

  // Simulate migration progress
  useEffect(() => {
    if (!isRunning || !progress) return;

    simulationRef.current = setInterval(() => {
      const currentTableIdx = tableProgress.findIndex(
        (tp) => tp.status === "running" || tp.status === "pending",
      );

      if (currentTableIdx === -1) {
        // All done
        if (simulationRef.current) clearInterval(simulationRef.current);
        setStatus("completed");
        addNotification({
          type: "success",
          title: "Migration Complete",
          message: `Successfully migrated ${progress.processedRows.toLocaleString()} rows.`,
        });
        appendLog("Migration completed successfully.");
        return;
      }

      const currentTable = tableProgress[currentTableIdx];

      if (currentTable.status === "pending") {
        updateTableProgress(currentTable.tableId, { status: "running" });
        appendLog(`Migrating table: ${currentTable.tableName}...`);
        return;
      }

      // Simulate processing
      const increment = Math.min(
        config.batchSize,
        currentTable.totalRows - currentTable.processedRows,
      );
      const newProcessed = currentTable.processedRows + increment;

      if (newProcessed >= currentTable.totalRows) {
        updateTableProgress(currentTable.tableId, {
          processedRows: currentTable.totalRows,
          status: "completed",
        });
        appendLog(`Completed table: ${currentTable.tableName}`);

        setProgress({
          ...progress,
          processedRows: progress.processedRows + increment,
          insertedRows: progress.insertedRows + Math.floor(increment * 0.6),
          updatedRows: progress.updatedRows + Math.floor(increment * 0.3),
          skippedRows: progress.skippedRows + Math.floor(increment * 0.1),
        });
      } else {
        updateTableProgress(currentTable.tableId, {
          processedRows: newProcessed,
        });
        setProgress({
          ...progress,
          processedRows: progress.processedRows + increment,
          insertedRows: progress.insertedRows + Math.floor(increment * 0.6),
          updatedRows: progress.updatedRows + Math.floor(increment * 0.3),
          skippedRows: progress.skippedRows + Math.floor(increment * 0.1),
        });
      }
    }, 300);

    return () => {
      if (simulationRef.current) clearInterval(simulationRef.current);
    };
  }, [
    isRunning,
    progress,
    tableProgress,
    config.batchSize,
    setStatus,
    setProgress,
    updateTableProgress,
    addNotification,
    appendLog,
  ]);

  const handleStart = useCallback(() => {
    startMigration();
    appendLog("Migration started.");
  }, [startMigration, appendLog]);

  const handleCancel = useCallback(() => {
    cancelMigration();
    if (simulationRef.current) clearInterval(simulationRef.current);
    if (timerRef.current) clearInterval(timerRef.current);
    appendLog("Migration cancelled by user.");
    addNotification({
      type: "warning",
      title: "Migration Cancelled",
      message: "The migration was cancelled by the user.",
    });
  }, [cancelMigration, appendLog, addNotification]);

  const formatTime = (ms: number): string => {
    const totalSec = Math.floor(ms / 1000);
    const minutes = Math.floor(totalSec / 60);
    const seconds = totalSec % 60;
    if (minutes > 0) return `${minutes}m ${seconds}s`;
    return `${seconds}s`;
  };

  const overallPercent =
    progress && progress.totalRows > 0
      ? Math.round((progress.processedRows / progress.totalRows) * 100)
      : 0;

  const allErrors = tableProgress.flatMap((tp) => tp.errors);

  return (
    <div className="space-y-4">
      <div>
        <h3 className="text-sm font-semibold text-neutral-800 dark:text-neutral-100">
          Execute Migration
        </h3>
        <p className="mt-1 text-xs text-neutral-500 dark:text-neutral-400">
          {isIdle && `Ready to migrate ${includedTables.length} tables.`}
          {isRunning && "Migration in progress..."}
          {isCompleted && "Migration completed successfully."}
          {isFailed && "Migration failed. See errors below."}
          {isCancelled && "Migration was cancelled."}
        </p>
      </div>

      {/* Start / Cancel buttons */}
      {isIdle && (
        <button
          onClick={handleStart}
          disabled={includedTables.length === 0}
          className="rounded bg-blue-600 px-5 py-2.5 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
        >
          Start Migration
        </button>
      )}

      {isRunning && (
        <button
          onClick={handleCancel}
          className="rounded border border-red-300 bg-red-50 px-4 py-2 text-xs font-medium text-red-700 hover:bg-red-100 dark:border-red-700 dark:bg-red-950/30 dark:text-red-400 dark:hover:bg-red-900/40"
        >
          Cancel Migration
        </button>
      )}

      {/* Overall progress */}
      {(isRunning || isCompleted || isFailed || isCancelled) && progress && (
        <div className="space-y-3">
          {/* Overall progress bar */}
          <div>
            <div className="mb-1 flex items-center justify-between text-xs">
              <span className="font-medium text-neutral-700 dark:text-neutral-300">
                Overall Progress
              </span>
              <span className="tabular-nums text-neutral-500 dark:text-neutral-400">
                {overallPercent}% -- {progress.processedRows.toLocaleString()} /{" "}
                {progress.totalRows.toLocaleString()} rows
              </span>
            </div>
            <div className="h-2.5 overflow-hidden rounded-full bg-neutral-200 dark:bg-neutral-700">
              <div
                className={`h-full rounded-full transition-all duration-300 ${
                  isCompleted
                    ? "bg-green-500"
                    : isFailed || isCancelled
                      ? "bg-red-500"
                      : "bg-blue-500"
                }`}
                style={{ width: `${overallPercent}%` }}
              />
            </div>
          </div>

          {/* Stats row */}
          <div className="grid grid-cols-5 gap-3">
            <StatCard
              label="Elapsed"
              value={formatTime(elapsedMs)}
              color="text-neutral-700 dark:text-neutral-300"
            />
            <StatCard
              label="Inserted"
              value={progress.insertedRows.toLocaleString()}
              color="text-green-600 dark:text-green-400"
            />
            <StatCard
              label="Updated"
              value={progress.updatedRows.toLocaleString()}
              color="text-amber-600 dark:text-amber-400"
            />
            <StatCard
              label="Skipped"
              value={progress.skippedRows.toLocaleString()}
              color="text-neutral-500 dark:text-neutral-400"
            />
            <StatCard
              label="Errors"
              value={String(progress.errorCount)}
              color="text-red-600 dark:text-red-400"
            />
          </div>
        </div>
      )}

      {/* Per-table progress */}
      {tableProgress.length > 0 && (
        <div className="overflow-hidden rounded-lg border border-neutral-200 dark:border-neutral-700">
          <div className="flex items-center border-b border-neutral-200 bg-neutral-50 px-4 py-2 text-[10px] font-semibold uppercase tracking-wider text-neutral-500 dark:border-neutral-700 dark:bg-neutral-800/50 dark:text-neutral-400">
            <div className="flex-1">Table</div>
            <div className="w-20 text-center">Status</div>
            <div className="flex-1">Progress</div>
            <div className="w-24 text-right">Rows</div>
          </div>
          <div className="max-h-[300px] overflow-y-auto">
            {tableProgress.map((tp) => {
              const percent =
                tp.totalRows > 0
                  ? Math.round((tp.processedRows / tp.totalRows) * 100)
                  : 0;

              return (
                <div
                  key={tp.tableId}
                  className="flex items-center gap-2 border-b border-neutral-100 px-4 py-2 text-xs last:border-b-0 dark:border-neutral-800"
                >
                  <div className="flex-1 font-mono text-neutral-700 dark:text-neutral-300">
                    {tp.tableName}
                  </div>
                  <div className="w-20 text-center">
                    <StatusBadge status={tp.status} />
                  </div>
                  <div className="flex-1">
                    <div className="h-1.5 overflow-hidden rounded-full bg-neutral-200 dark:bg-neutral-700">
                      <div
                        className={`h-full rounded-full transition-all duration-300 ${
                          tp.status === "completed"
                            ? "bg-green-500"
                            : tp.status === "failed"
                              ? "bg-red-500"
                              : tp.status === "running"
                                ? "bg-blue-500"
                                : "bg-neutral-300 dark:bg-neutral-600"
                        }`}
                        style={{ width: `${percent}%` }}
                      />
                    </div>
                  </div>
                  <div className="w-24 text-right tabular-nums text-neutral-500 dark:text-neutral-400">
                    {tp.processedRows.toLocaleString()} /{" "}
                    {tp.totalRows.toLocaleString()}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Error panel */}
      {allErrors.length > 0 && (
        <div>
          <button
            onClick={() => setShowErrors(!showErrors)}
            className="flex items-center gap-1 text-xs font-medium text-red-600 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300"
          >
            <svg
              className={`h-3.5 w-3.5 transition-transform ${showErrors ? "rotate-90" : ""}`}
              viewBox="0 0 20 20"
              fill="currentColor"
            >
              <path
                fillRule="evenodd"
                d="M7.21 14.77a.75.75 0 01.02-1.06L11.168 10 7.23 6.29a.75.75 0 111.04-1.08l4.5 4.25a.75.75 0 010 1.08l-4.5 4.25a.75.75 0 01-1.06-.02z"
                clipRule="evenodd"
              />
            </svg>
            Errors ({allErrors.length})
          </button>
          {showErrors && (
            <div className="mt-2 max-h-[200px] space-y-1 overflow-y-auto">
              {allErrors.map((err) => (
                <div
                  key={err.id}
                  className="flex items-start gap-2 rounded border border-red-200 bg-red-50 px-3 py-2 text-xs dark:border-red-800 dark:bg-red-950/20"
                >
                  <span className="shrink-0 text-red-500">
                    <svg
                      className="h-3.5 w-3.5"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                      strokeWidth={2}
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z"
                      />
                    </svg>
                  </span>
                  <div className="flex-1">
                    <span className="text-red-700 dark:text-red-400">
                      {err.message}
                    </span>
                    {err.rowIndex !== undefined && (
                      <span className="ml-1 text-red-400 dark:text-red-500">
                        (row {err.rowIndex})
                      </span>
                    )}
                  </div>
                  <div className="flex shrink-0 gap-1">
                    <button className="rounded border border-red-200 px-1.5 py-0.5 text-[10px] text-red-600 hover:bg-red-100 dark:border-red-700 dark:text-red-400 dark:hover:bg-red-900/30">
                      Retry
                    </button>
                    <button className="rounded border border-neutral-200 px-1.5 py-0.5 text-[10px] text-neutral-500 hover:bg-neutral-100 dark:border-neutral-700 dark:text-neutral-400 dark:hover:bg-neutral-800">
                      Skip
                    </button>
                    <button className="rounded border border-red-300 px-1.5 py-0.5 text-[10px] text-red-700 hover:bg-red-100 dark:border-red-700 dark:text-red-400 dark:hover:bg-red-900/30">
                      Abort
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Completion summary */}
      {(isCompleted || isCancelled) && progress && (
        <div
          className={`rounded-lg border px-4 py-4 ${
            isCompleted
              ? "border-green-200 bg-green-50 dark:border-green-800 dark:bg-green-950/20"
              : "border-amber-200 bg-amber-50 dark:border-amber-800 dark:bg-amber-950/20"
          }`}
        >
          <h4
            className={`text-sm font-semibold ${
              isCompleted
                ? "text-green-700 dark:text-green-400"
                : "text-amber-700 dark:text-amber-400"
            }`}
          >
            {isCompleted ? "Migration Completed" : "Migration Cancelled"}
          </h4>
          <div className="mt-2 grid grid-cols-3 gap-4 text-xs">
            <div>
              <span className="text-neutral-500 dark:text-neutral-400">
                Total Processed:
              </span>
              <span className="ml-1 font-medium text-neutral-700 dark:text-neutral-300">
                {progress.processedRows.toLocaleString()} rows
              </span>
            </div>
            <div>
              <span className="text-neutral-500 dark:text-neutral-400">
                Elapsed Time:
              </span>
              <span className="ml-1 font-medium text-neutral-700 dark:text-neutral-300">
                {formatTime(elapsedMs)}
              </span>
            </div>
            <div>
              <span className="text-neutral-500 dark:text-neutral-400">
                Errors:
              </span>
              <span className="ml-1 font-medium text-neutral-700 dark:text-neutral-300">
                {progress.errorCount}
              </span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function StatusBadge({
  status,
}: {
  status: "pending" | "running" | "completed" | "failed" | "skipped";
}) {
  const styles: Record<string, string> = {
    pending:
      "bg-neutral-100 text-neutral-500 dark:bg-neutral-700 dark:text-neutral-400",
    running:
      "bg-blue-100 text-blue-700 dark:bg-blue-900/40 dark:text-blue-400",
    completed:
      "bg-green-100 text-green-700 dark:bg-green-900/40 dark:text-green-400",
    failed: "bg-red-100 text-red-700 dark:bg-red-900/40 dark:text-red-400",
    skipped:
      "bg-neutral-100 text-neutral-500 dark:bg-neutral-700 dark:text-neutral-400",
  };

  return (
    <span
      className={`inline-block rounded px-1.5 py-0.5 text-[10px] font-medium ${styles[status]}`}
    >
      {status}
    </span>
  );
}

function StatCard({
  label,
  value,
  color,
}: {
  label: string;
  value: string;
  color: string;
}) {
  return (
    <div className="rounded-lg border border-neutral-200 bg-white px-3 py-2 dark:border-neutral-700 dark:bg-neutral-800">
      <div className="text-[10px] uppercase tracking-wider text-neutral-400">
        {label}
      </div>
      <div className={`mt-0.5 text-sm font-bold tabular-nums ${color}`}>
        {value}
      </div>
    </div>
  );
}
