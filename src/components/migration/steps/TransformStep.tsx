import { useState, useMemo } from "react";
import {
  useMigrationStore,
  type TransformRule,
  type TransformRuleType,
} from "../../../stores/migrationStore";

// Simulated column data -- in production fetched from the backend
const DEMO_COLUMNS: Record<string, Array<{ name: string; type: string }>> = {
  users: [
    { name: "id", type: "int" },
    { name: "name", type: "varchar(255)" },
    { name: "email", type: "varchar(255)" },
    { name: "created_at", type: "datetime" },
    { name: "is_active", type: "bit" },
  ],
  orders: [
    { name: "id", type: "int" },
    { name: "user_id", type: "int" },
    { name: "total", type: "decimal(10,2)" },
    { name: "status", type: "varchar(50)" },
    { name: "order_date", type: "datetime" },
  ],
  products: [
    { name: "id", type: "int" },
    { name: "name", type: "varchar(255)" },
    { name: "price", type: "decimal(10,2)" },
    { name: "category_id", type: "int" },
    { name: "sku", type: "varchar(50)" },
  ],
};

const ruleTypeLabels: Record<TransformRuleType, string> = {
  rename: "Rename Column",
  type_cast: "Type Cast",
  value_map: "Value Map",
  default_for_null: "Default for NULL",
  drop_column: "Drop Column",
};

const ruleTypeColors: Record<TransformRuleType, string> = {
  rename: "bg-blue-100 text-blue-700 dark:bg-blue-900/40 dark:text-blue-400",
  type_cast:
    "bg-purple-100 text-purple-700 dark:bg-purple-900/40 dark:text-purple-400",
  value_map:
    "bg-green-100 text-green-700 dark:bg-green-900/40 dark:text-green-400",
  default_for_null:
    "bg-amber-100 text-amber-700 dark:bg-amber-900/40 dark:text-amber-400",
  drop_column: "bg-red-100 text-red-700 dark:bg-red-900/40 dark:text-red-400",
};

export default function TransformStep() {
  const {
    tableMappings,
    transformRules,
    addTransformRule,
    removeTransformRule,
    reorderTransformRule,
  } = useMigrationStore();

  const includedTables = tableMappings.filter((m) => m.included && m.targetTable);
  const [selectedTableId, setSelectedTableId] = useState<string>(
    includedTables[0]?.id ?? "",
  );
  const [addingRule, setAddingRule] = useState(false);

  // New rule form state
  const [newRuleType, setNewRuleType] = useState<TransformRuleType>("rename");
  const [newSourceCol, setNewSourceCol] = useState("");
  const [newTargetCol, setNewTargetCol] = useState("");
  const [newConfigValue, setNewConfigValue] = useState("");

  const selectedTable = includedTables.find((t) => t.id === selectedTableId);

  const columns = useMemo(() => {
    if (!selectedTable) return [];
    return DEMO_COLUMNS[selectedTable.sourceTable] ?? [];
  }, [selectedTable]);

  const tableRules = useMemo(
    () =>
      transformRules
        .filter((r) => r.tableId === selectedTableId)
        .sort((a, b) => a.order - b.order),
    [transformRules, selectedTableId],
  );

  const handleAddRule = () => {
    if (!newSourceCol) return;

    const rule: TransformRule = {
      id: crypto.randomUUID(),
      tableId: selectedTableId,
      sourceColumn: newSourceCol,
      targetColumn: newTargetCol || newSourceCol,
      ruleType: newRuleType,
      config: newConfigValue ? { value: newConfigValue } : {},
      order: tableRules.length,
    };
    addTransformRule(rule);
    setAddingRule(false);
    setNewSourceCol("");
    setNewTargetCol("");
    setNewConfigValue("");
  };

  const handleMoveRule = (id: string, direction: -1 | 1) => {
    const rule = tableRules.find((r) => r.id === id);
    if (!rule) return;
    const newOrder = rule.order + direction;
    if (newOrder < 0 || newOrder >= tableRules.length) return;

    // Swap with neighbor
    const neighbor = tableRules.find((r) => r.order === newOrder);
    if (neighbor) {
      reorderTransformRule(neighbor.id, rule.order);
    }
    reorderTransformRule(id, newOrder);
  };

  if (includedTables.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-16">
        <p className="text-sm font-medium text-neutral-500 dark:text-neutral-400">
          No tables mapped
        </p>
        <p className="mt-1 text-xs text-neutral-400 dark:text-neutral-500">
          Go back to Map Tables to configure table mappings first.
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div>
        <h3 className="text-sm font-semibold text-neutral-800 dark:text-neutral-100">
          Column Transformations
        </h3>
        <p className="mt-1 text-xs text-neutral-500 dark:text-neutral-400">
          Add transformation rules to modify data during migration.
        </p>
      </div>

      {/* Table selector */}
      <div className="flex items-center gap-2">
        <label className="text-xs font-medium text-neutral-600 dark:text-neutral-400">
          Table:
        </label>
        <select
          value={selectedTableId}
          onChange={(e) => setSelectedTableId(e.target.value)}
          className="input-field w-64"
        >
          {includedTables.map((t) => (
            <option key={t.id} value={t.id}>
              {t.sourceTable} -&gt; {t.targetTable}
            </option>
          ))}
        </select>
      </div>

      {/* Column mapping overview */}
      <div className="overflow-hidden rounded-lg border border-neutral-200 dark:border-neutral-700">
        <div className="flex items-center border-b border-neutral-200 bg-neutral-50 px-4 py-2 text-[10px] font-semibold uppercase tracking-wider text-neutral-500 dark:border-neutral-700 dark:bg-neutral-800/50 dark:text-neutral-400">
          <div className="flex-1">Source Column</div>
          <div className="w-32 text-center">Type</div>
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
          <div className="flex-1">Target Column</div>
        </div>
        <div className="max-h-[200px] overflow-y-auto">
          {columns.map((col) => (
            <div
              key={col.name}
              className="flex items-center border-b border-neutral-100 px-4 py-1.5 text-xs last:border-b-0 dark:border-neutral-800"
            >
              <div className="flex-1 font-mono text-neutral-700 dark:text-neutral-300">
                {col.name}
              </div>
              <div className="w-32 text-center font-mono text-[11px] text-neutral-400 dark:text-neutral-500">
                {col.type}
              </div>
              <div className="w-8 text-center text-neutral-300 dark:text-neutral-600">
                -&gt;
              </div>
              <div className="flex-1 font-mono text-neutral-700 dark:text-neutral-300">
                {col.name}
              </div>
            </div>
          ))}
          {columns.length === 0 && (
            <div className="px-4 py-4 text-center text-xs text-neutral-400">
              No column info available for this table.
            </div>
          )}
        </div>
      </div>

      {/* Transform rules list */}
      <div>
        <div className="flex items-center justify-between">
          <h4 className="text-xs font-semibold text-neutral-700 dark:text-neutral-300">
            Transformation Rules ({tableRules.length})
          </h4>
          {!addingRule && (
            <button
              onClick={() => setAddingRule(true)}
              className="rounded bg-blue-600 px-2.5 py-1 text-[10px] font-medium text-white hover:bg-blue-700"
            >
              + Add Rule
            </button>
          )}
        </div>

        {/* Add rule form */}
        {addingRule && (
          <div className="mt-2 space-y-2 rounded-lg border border-blue-200 bg-blue-50 p-3 dark:border-blue-800 dark:bg-blue-950/20">
            <div className="flex gap-2">
              <div className="flex-1">
                <label className="mb-1 block text-[10px] font-medium text-neutral-600 dark:text-neutral-400">
                  Rule Type
                </label>
                <select
                  value={newRuleType}
                  onChange={(e) =>
                    setNewRuleType(e.target.value as TransformRuleType)
                  }
                  className="input-field"
                >
                  {(Object.keys(ruleTypeLabels) as TransformRuleType[]).map(
                    (rt) => (
                      <option key={rt} value={rt}>
                        {ruleTypeLabels[rt]}
                      </option>
                    ),
                  )}
                </select>
              </div>
              <div className="flex-1">
                <label className="mb-1 block text-[10px] font-medium text-neutral-600 dark:text-neutral-400">
                  Source Column
                </label>
                <select
                  value={newSourceCol}
                  onChange={(e) => setNewSourceCol(e.target.value)}
                  className="input-field"
                >
                  <option value="">Select column...</option>
                  {columns.map((col) => (
                    <option key={col.name} value={col.name}>
                      {col.name}
                    </option>
                  ))}
                </select>
              </div>
            </div>

            <div className="flex gap-2">
              {newRuleType !== "drop_column" && (
                <div className="flex-1">
                  <label className="mb-1 block text-[10px] font-medium text-neutral-600 dark:text-neutral-400">
                    Target Column
                  </label>
                  <input
                    value={newTargetCol}
                    onChange={(e) => setNewTargetCol(e.target.value)}
                    placeholder="Same as source"
                    className="input-field"
                  />
                </div>
              )}
              {(newRuleType === "type_cast" ||
                newRuleType === "default_for_null" ||
                newRuleType === "value_map") && (
                <div className="flex-1">
                  <label className="mb-1 block text-[10px] font-medium text-neutral-600 dark:text-neutral-400">
                    {newRuleType === "type_cast"
                      ? "Target Type"
                      : newRuleType === "default_for_null"
                        ? "Default Value"
                        : "Mapping (JSON)"}
                  </label>
                  <input
                    value={newConfigValue}
                    onChange={(e) => setNewConfigValue(e.target.value)}
                    className="input-field"
                    placeholder={
                      newRuleType === "type_cast"
                        ? "e.g. varchar(100)"
                        : newRuleType === "default_for_null"
                          ? "e.g. N/A"
                          : '{"old": "new"}'
                    }
                  />
                </div>
              )}
            </div>

            <div className="flex justify-end gap-2">
              <button
                onClick={() => {
                  setAddingRule(false);
                  setNewSourceCol("");
                  setNewTargetCol("");
                  setNewConfigValue("");
                }}
                className="rounded border border-neutral-300 px-2.5 py-1 text-[10px] text-neutral-600 hover:bg-neutral-100 dark:border-neutral-600 dark:text-neutral-400 dark:hover:bg-neutral-700"
              >
                Cancel
              </button>
              <button
                onClick={handleAddRule}
                disabled={!newSourceCol}
                className="rounded bg-blue-600 px-2.5 py-1 text-[10px] font-medium text-white hover:bg-blue-700 disabled:opacity-50"
              >
                Add
              </button>
            </div>
          </div>
        )}

        {/* Existing rules */}
        {tableRules.length > 0 ? (
          <div className="mt-2 space-y-1">
            {tableRules.map((rule) => (
              <div
                key={rule.id}
                className="flex items-center gap-2 rounded-lg border border-neutral-200 bg-white px-3 py-2 dark:border-neutral-700 dark:bg-neutral-800"
              >
                {/* Reorder buttons */}
                <div className="flex flex-col gap-0.5">
                  <button
                    onClick={() => handleMoveRule(rule.id, -1)}
                    disabled={rule.order === 0}
                    className="rounded p-0.5 text-neutral-400 hover:bg-neutral-100 disabled:opacity-30 dark:hover:bg-neutral-700"
                  >
                    <svg
                      className="h-3 w-3"
                      viewBox="0 0 20 20"
                      fill="currentColor"
                    >
                      <path
                        fillRule="evenodd"
                        d="M14.77 12.79a.75.75 0 01-1.06-.02L10 8.832l-3.71 3.938a.75.75 0 11-1.08-1.04l4.25-4.5a.75.75 0 011.08 0l4.25 4.5a.75.75 0 01-.02 1.06z"
                        clipRule="evenodd"
                      />
                    </svg>
                  </button>
                  <button
                    onClick={() => handleMoveRule(rule.id, 1)}
                    disabled={rule.order === tableRules.length - 1}
                    className="rounded p-0.5 text-neutral-400 hover:bg-neutral-100 disabled:opacity-30 dark:hover:bg-neutral-700"
                  >
                    <svg
                      className="h-3 w-3"
                      viewBox="0 0 20 20"
                      fill="currentColor"
                    >
                      <path
                        fillRule="evenodd"
                        d="M5.23 7.21a.75.75 0 011.06.02L10 11.168l3.71-3.938a.75.75 0 111.08 1.04l-4.25 4.5a.75.75 0 01-1.08 0l-4.25-4.5a.75.75 0 01.02-1.06z"
                        clipRule="evenodd"
                      />
                    </svg>
                  </button>
                </div>

                {/* Rule badge */}
                <span
                  className={`shrink-0 rounded px-1.5 py-0.5 text-[10px] font-semibold ${ruleTypeColors[rule.ruleType]}`}
                >
                  {ruleTypeLabels[rule.ruleType]}
                </span>

                {/* Rule details */}
                <div className="flex-1 text-xs text-neutral-700 dark:text-neutral-300">
                  <span className="font-mono">{rule.sourceColumn}</span>
                  {rule.ruleType !== "drop_column" && (
                    <>
                      <span className="mx-1 text-neutral-400">-&gt;</span>
                      <span className="font-mono">{rule.targetColumn}</span>
                    </>
                  )}
                  {rule.config.value && (
                    <span className="ml-2 text-neutral-400">
                      ({rule.config.value})
                    </span>
                  )}
                </div>

                {/* Delete button */}
                <button
                  onClick={() => removeTransformRule(rule.id)}
                  className="rounded p-1 text-neutral-400 hover:bg-red-50 hover:text-red-500 dark:hover:bg-red-900/20"
                  title="Remove rule"
                >
                  <svg
                    className="h-3.5 w-3.5"
                    viewBox="0 0 20 20"
                    fill="currentColor"
                  >
                    <path
                      fillRule="evenodd"
                      d="M8.75 1A2.75 2.75 0 006 3.75v.443c-.795.077-1.584.176-2.365.298a.75.75 0 10.23 1.482l.149-.022.841 10.518A2.75 2.75 0 007.596 19h4.807a2.75 2.75 0 002.742-2.53l.841-10.52.149.023a.75.75 0 00.23-1.482A41.03 41.03 0 0014 4.193V3.75A2.75 2.75 0 0011.25 1h-2.5zM10 4c.84 0 1.673.025 2.5.075V3.75c0-.69-.56-1.25-1.25-1.25h-2.5c-.69 0-1.25.56-1.25 1.25v.325C8.327 4.025 9.16 4 10 4zM8.58 7.72a.75.75 0 00-1.5.06l.3 7.5a.75.75 0 101.5-.06l-.3-7.5zm4.34.06a.75.75 0 10-1.5-.06l-.3 7.5a.75.75 0 101.5.06l.3-7.5z"
                      clipRule="evenodd"
                    />
                  </svg>
                </button>
              </div>
            ))}
          </div>
        ) : (
          !addingRule && (
            <div className="mt-3 rounded-lg border border-dashed border-neutral-300 px-4 py-6 text-center text-xs text-neutral-400 dark:border-neutral-700 dark:text-neutral-500">
              No transformation rules for this table. Data will be copied
              as-is.
            </div>
          )
        )}
      </div>
    </div>
  );
}
