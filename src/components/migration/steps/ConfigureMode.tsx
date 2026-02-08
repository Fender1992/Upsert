import {
  useMigrationStore,
  type MigrationMode,
  type ConflictResolution,
} from "../../../stores/migrationStore";

const modeDescriptions: Record<MigrationMode, string> = {
  Upsert:
    "Insert new rows, update existing rows based on primary key. No deletes.",
  Mirror:
    "Make the target an exact copy of the source. Inserts, updates, and deletes rows to match.",
  AppendOnly:
    "Insert new rows only. Skip any rows that already exist in the target.",
  Merge:
    "Merge source data into target using a custom merge strategy with conflict resolution rules.",
  SchemaOnly:
    "Migrate table schemas only (columns, indexes, constraints). No data is transferred.",
};

const conflictOptions: Array<{ value: ConflictResolution; label: string }> = [
  { value: "SourceWins", label: "Source Wins" },
  { value: "TargetWins", label: "Target Wins" },
  { value: "NewestWins", label: "Newest Wins (by timestamp)" },
  { value: "ManualReview", label: "Flag for Manual Review" },
];

const transactionModes: Array<{
  value: "PerBatch" | "WholeMigration" | "None";
  label: string;
  description: string;
}> = [
  {
    value: "PerBatch",
    label: "Per Batch",
    description: "Commit after each batch. Good balance of safety and speed.",
  },
  {
    value: "WholeMigration",
    label: "Whole Migration",
    description: "Single transaction. All or nothing -- rolls back on any error.",
  },
  {
    value: "None",
    label: "None",
    description: "No transaction wrapping. Fastest, but no rollback capability.",
  },
];

export default function ConfigureMode() {
  const { config, setConfig } = useMigrationStore();

  return (
    <div className="space-y-6">
      <div>
        <h3 className="text-sm font-semibold text-neutral-800 dark:text-neutral-100">
          Configure Migration Mode
        </h3>
        <p className="mt-1 text-xs text-neutral-500 dark:text-neutral-400">
          Choose how the migration should handle data transfer and conflicts.
        </p>
      </div>

      {/* Migration Mode */}
      <section className="space-y-2">
        <label className="block text-xs font-medium text-neutral-600 dark:text-neutral-400">
          Migration Mode
        </label>
        <div className="space-y-2">
          {(Object.keys(modeDescriptions) as MigrationMode[]).map((mode) => {
            const isSelected = config.mode === mode;
            return (
              <label
                key={mode}
                className={`flex cursor-pointer items-start gap-3 rounded-lg border px-4 py-3 transition-colors ${
                  isSelected
                    ? "border-blue-500 bg-blue-50 dark:border-blue-500 dark:bg-blue-950/30"
                    : "border-neutral-200 bg-white hover:border-neutral-300 dark:border-neutral-700 dark:bg-neutral-800 dark:hover:border-neutral-600"
                }`}
              >
                <input
                  type="radio"
                  name="migrationMode"
                  value={mode}
                  checked={isSelected}
                  onChange={() => setConfig({ mode })}
                  className="mt-0.5"
                />
                <div>
                  <span className="text-xs font-medium text-neutral-800 dark:text-neutral-100">
                    {mode}
                  </span>
                  <p className="mt-0.5 text-[11px] text-neutral-500 dark:text-neutral-400">
                    {modeDescriptions[mode]}
                  </p>
                </div>
              </label>
            );
          })}
        </div>
      </section>

      {/* Conflict Resolution */}
      <section className="space-y-2">
        <label className="block text-xs font-medium text-neutral-600 dark:text-neutral-400">
          Conflict Resolution
        </label>
        <select
          value={config.conflictResolution}
          onChange={(e) =>
            setConfig({
              conflictResolution: e.target.value as ConflictResolution,
            })
          }
          className="input-field"
        >
          {conflictOptions.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
      </section>

      {/* Batch Size */}
      <section className="space-y-2">
        <label className="block text-xs font-medium text-neutral-600 dark:text-neutral-400">
          Batch Size
        </label>
        <div className="flex items-center gap-3">
          <input
            type="number"
            value={config.batchSize}
            onChange={(e) =>
              setConfig({
                batchSize: Math.max(1, Number(e.target.value) || 1),
              })
            }
            min={1}
            max={100000}
            className="input-field w-32"
          />
          <span className="text-[11px] text-neutral-400 dark:text-neutral-500">
            rows per batch (1 - 100,000)
          </span>
        </div>
      </section>

      {/* Transaction Mode */}
      <section className="space-y-2">
        <label className="block text-xs font-medium text-neutral-600 dark:text-neutral-400">
          Transaction Mode
        </label>
        <div className="space-y-2">
          {transactionModes.map((tm) => {
            const isSelected = config.transactionMode === tm.value;
            return (
              <label
                key={tm.value}
                className={`flex cursor-pointer items-start gap-3 rounded-lg border px-4 py-2.5 transition-colors ${
                  isSelected
                    ? "border-blue-500 bg-blue-50 dark:border-blue-500 dark:bg-blue-950/30"
                    : "border-neutral-200 bg-white hover:border-neutral-300 dark:border-neutral-700 dark:bg-neutral-800 dark:hover:border-neutral-600"
                }`}
              >
                <input
                  type="radio"
                  name="transactionMode"
                  value={tm.value}
                  checked={isSelected}
                  onChange={() => setConfig({ transactionMode: tm.value })}
                  className="mt-0.5"
                />
                <div>
                  <span className="text-xs font-medium text-neutral-800 dark:text-neutral-100">
                    {tm.label}
                  </span>
                  <p className="mt-0.5 text-[11px] text-neutral-500 dark:text-neutral-400">
                    {tm.description}
                  </p>
                </div>
              </label>
            );
          })}
        </div>
      </section>

      {/* Dry Run Toggle */}
      <section className="space-y-2">
        <label className="flex items-center gap-2 text-xs text-neutral-700 dark:text-neutral-300">
          <input
            type="checkbox"
            checked={config.dryRun}
            onChange={(e) => setConfig({ dryRun: e.target.checked })}
            className="rounded border-neutral-300 dark:border-neutral-600"
          />
          <span className="font-medium">Dry Run</span>
          <span className="text-neutral-400 dark:text-neutral-500">
            -- Preview changes without executing
          </span>
        </label>
      </section>
    </div>
  );
}
