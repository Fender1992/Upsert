import { useState, useMemo, useCallback } from "react";
import type {
  SchemaDiffResult,
  SchemaChange,
  ChangeType,
  ObjectType,
  ChangeTypeFilter,
  ObjectTypeFilter,
} from "../../types/comparison";
import DiffSummaryBar from "./DiffSummaryBar";

// -- Helpers ------------------------------------------------------------------

const changeColor: Record<ChangeType, string> = {
  Added: "#22c55e",
  Removed: "#ef4444",
  Modified: "#f59e0b",
  Unchanged: "#9ca3af",
};

const changeTailwind: Record<ChangeType, string> = {
  Added: "text-green-600 dark:text-green-400",
  Removed: "text-red-600 dark:text-red-400",
  Modified: "text-amber-600 dark:text-amber-400",
  Unchanged: "text-neutral-500 dark:text-neutral-500",
};

const changeBg: Record<ChangeType, string> = {
  Added: "bg-green-50 dark:bg-green-950/30",
  Removed: "bg-red-50 dark:bg-red-950/30",
  Modified: "bg-amber-50 dark:bg-amber-950/30",
  Unchanged: "",
};

const changeSymbol: Record<ChangeType, string> = {
  Added: "+",
  Removed: "-",
  Modified: "~",
  Unchanged: "=",
};

interface TreeNode {
  key: string;
  label: string;
  objectType?: ObjectType;
  changeType?: ChangeType;
  children: TreeNode[];
  change?: SchemaChange;
  diffCount: number;
}

function buildTree(changes: SchemaChange[]): TreeNode[] {
  const tableMap = new Map<string, SchemaChange[]>();

  for (const c of changes) {
    // Group by parent table name: for Table changes use objectName, for
    // Column/Index/Constraint extract the table portion (before the dot).
    let tableName: string;
    if (c.objectType === "Table") {
      tableName = c.objectName;
    } else {
      const dotIdx = c.objectName.indexOf(".");
      tableName = dotIdx >= 0 ? c.objectName.substring(0, dotIdx) : c.objectName;
    }

    if (!tableMap.has(tableName)) tableMap.set(tableName, []);
    tableMap.get(tableName)!.push(c);
  }

  const tables: TreeNode[] = [];

  for (const [tableName, tableChanges] of tableMap) {
    const tableChange = tableChanges.find(
      (c) => c.objectType === "Table" && c.objectName === tableName,
    );

    const childChanges = tableChanges.filter(
      (c) => !(c.objectType === "Table" && c.objectName === tableName),
    );

    // Group children by objectType
    const grouped = new Map<ObjectType, SchemaChange[]>();
    for (const c of childChanges) {
      if (!grouped.has(c.objectType as ObjectType))
        grouped.set(c.objectType as ObjectType, []);
      grouped.get(c.objectType as ObjectType)!.push(c);
    }

    const groupNodes: TreeNode[] = [];
    for (const [objectType, items] of grouped) {
      const leafNodes: TreeNode[] = items.map((c) => {
        const shortName = c.objectName.includes(".")
          ? c.objectName.substring(c.objectName.indexOf(".") + 1)
          : c.objectName;
        return {
          key: `${tableName}.${objectType}.${c.objectName}`,
          label: shortName,
          objectType: c.objectType as ObjectType,
          changeType: c.changeType as ChangeType,
          children: [],
          change: c,
          diffCount: c.changeType !== "Unchanged" ? 1 : 0,
        };
      });

      const diffCount = leafNodes.reduce((s, n) => s + n.diffCount, 0);

      groupNodes.push({
        key: `${tableName}.${objectType}`,
        label: `${objectType}s`,
        objectType: objectType,
        children: leafNodes,
        diffCount,
      });
    }

    const tableDiffCount =
      (tableChange && tableChange.changeType !== "Unchanged" ? 1 : 0) +
      groupNodes.reduce((s, n) => s + n.diffCount, 0);

    tables.push({
      key: tableName,
      label: tableName,
      objectType: "Table",
      changeType: tableChange?.changeType as ChangeType | undefined,
      children: groupNodes,
      change: tableChange,
      diffCount: tableDiffCount,
    });
  }

  return tables.sort((a, b) => a.label.localeCompare(b.label));
}

// -- Sub-components -----------------------------------------------------------

function TreeNodeRow({
  node,
  depth,
  expanded,
  onToggle,
  onSelect,
  selected,
}: {
  node: TreeNode;
  depth: number;
  expanded: boolean;
  onToggle: (key: string) => void;
  onSelect: (node: TreeNode) => void;
  selected: boolean;
}) {
  const hasChildren = node.children.length > 0;
  const ct = node.changeType;

  return (
    <div
      className={`flex cursor-pointer items-center gap-1 py-0.5 pr-2 text-xs hover:bg-neutral-100 dark:hover:bg-neutral-800 ${
        selected ? "bg-blue-50 dark:bg-blue-950/30" : ""
      } ${ct ? changeBg[ct] : ""}`}
      style={{ paddingLeft: depth * 16 + 8 }}
      onClick={() => {
        if (hasChildren) onToggle(node.key);
        onSelect(node);
      }}
    >
      {/* Expand arrow */}
      {hasChildren ? (
        <svg
          className={`h-3 w-3 shrink-0 text-neutral-400 transition-transform ${expanded ? "rotate-90" : ""}`}
          viewBox="0 0 20 20"
          fill="currentColor"
        >
          <path
            fillRule="evenodd"
            d="M7.21 14.77a.75.75 0 01.02-1.06L11.168 10 7.23 6.29a.75.75 0 111.04-1.08l4.5 4.25a.75.75 0 010 1.08l-4.5 4.25a.75.75 0 01-1.06-.02z"
            clipRule="evenodd"
          />
        </svg>
      ) : (
        <span className="w-3 shrink-0" />
      )}

      {/* Change symbol */}
      {ct && (
        <span className={`w-3 shrink-0 text-center font-mono font-bold ${changeTailwind[ct]}`}>
          {changeSymbol[ct]}
        </span>
      )}

      {/* Label */}
      <span
        className={`flex-1 truncate ${ct ? changeTailwind[ct] : "text-neutral-700 dark:text-neutral-300"}`}
      >
        {node.label}
      </span>

      {/* Badge */}
      {node.diffCount > 0 && (
        <span className="rounded-full bg-amber-200 px-1.5 text-[10px] font-medium text-amber-800 dark:bg-amber-900/50 dark:text-amber-300">
          {node.diffCount}
        </span>
      )}
    </div>
  );
}

function DetailPanel({ change }: { change: SchemaChange }) {
  return (
    <div className="flex flex-col gap-2 p-3">
      <div className="flex items-center gap-2">
        <span
          className={`rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase ${changeTailwind[change.changeType]}`}
        >
          {change.changeType}
        </span>
        <span className="text-xs font-medium text-neutral-700 dark:text-neutral-300">
          {change.objectType}: {change.objectName}
        </span>
      </div>

      {change.details.length > 0 && (
        <table className="w-full text-xs">
          <thead>
            <tr className="border-b border-neutral-200 dark:border-neutral-700">
              <th className="py-1 pr-2 text-left font-medium text-neutral-500">Property</th>
              <th className="py-1 pr-2 text-left font-medium text-neutral-500">Source</th>
              <th className="py-1 text-left font-medium text-neutral-500">Target</th>
            </tr>
          </thead>
          <tbody>
            {change.details.map((d, i) => (
              <tr
                key={i}
                className="border-b border-neutral-100 dark:border-neutral-800"
              >
                <td className="py-1 pr-2 text-neutral-600 dark:text-neutral-400">
                  {d.property}
                </td>
                <td className="py-1 pr-2 font-mono text-red-600 dark:text-red-400">
                  {d.sourceValue ?? <span className="text-neutral-300">--</span>}
                </td>
                <td className="py-1 font-mono text-green-600 dark:text-green-400">
                  {d.targetValue ?? <span className="text-neutral-300">--</span>}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {change.details.length === 0 && (
        <span className="text-xs text-neutral-400">No property differences.</span>
      )}
    </div>
  );
}

// -- Main component -----------------------------------------------------------

interface Props {
  diff: SchemaDiffResult;
}

export default function SchemaDiffViewer({ diff }: Props) {
  const [changeFilter, setChangeFilter] = useState<ChangeTypeFilter>("all");
  const [objectFilter, setObjectFilter] = useState<ObjectTypeFilter>("all");
  const [search, setSearch] = useState("");
  const [expandedKeys, setExpandedKeys] = useState<Set<string>>(new Set());
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [diffNavIndex, setDiffNavIndex] = useState(0);

  // Filter changes
  const filtered = useMemo(() => {
    return diff.changes.filter((c) => {
      if (changeFilter !== "all" && c.changeType !== changeFilter) return false;
      if (objectFilter !== "all" && c.objectType !== objectFilter) return false;
      if (search) {
        const q = search.toLowerCase();
        if (
          !c.objectName.toLowerCase().includes(q) &&
          !c.objectType.toLowerCase().includes(q)
        )
          return false;
      }
      return true;
    });
  }, [diff.changes, changeFilter, objectFilter, search]);

  const tree = useMemo(() => buildTree(filtered), [filtered]);

  // All diff nodes (non-Unchanged) for navigation
  const diffNodes = useMemo(() => {
    const result: SchemaChange[] = [];
    for (const c of filtered) {
      if (c.changeType !== "Unchanged") result.push(c);
    }
    return result;
  }, [filtered]);

  const toggleExpanded = useCallback((key: string) => {
    setExpandedKeys((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  }, []);

  const selectNode = useCallback((node: TreeNode) => {
    setSelectedKey(node.key);
  }, []);

  const navigateDiff = useCallback(
    (direction: 1 | -1) => {
      if (diffNodes.length === 0) return;
      const next = (diffNavIndex + direction + diffNodes.length) % diffNodes.length;
      setDiffNavIndex(next);
      const target = diffNodes[next];
      if (target) {
        setSelectedKey(
          `${target.objectType === "Table" ? target.objectName : target.objectName.split(".")[0]}.${target.objectType}.${target.objectName}`,
        );
        // Expand parents
        const tableName =
          target.objectType === "Table"
            ? target.objectName
            : target.objectName.substring(0, target.objectName.indexOf("."));
        setExpandedKeys((prev) => {
          const next = new Set(prev);
          next.add(tableName);
          next.add(`${tableName}.${target.objectType}`);
          return next;
        });
      }
    },
    [diffNavIndex, diffNodes],
  );

  // Render tree recursively
  const renderNodes = (nodes: TreeNode[], depth: number): React.ReactNode[] => {
    const result: React.ReactNode[] = [];
    for (const node of nodes) {
      const isExpanded = expandedKeys.has(node.key);
      result.push(
        <TreeNodeRow
          key={node.key}
          node={node}
          depth={depth}
          expanded={isExpanded}
          onToggle={toggleExpanded}
          onSelect={selectNode}
          selected={selectedKey === node.key}
        />,
      );
      if (isExpanded && node.children.length > 0) {
        result.push(...renderNodes(node.children, depth + 1));
      }
    }
    return result;
  };

  // Find the selected change for detail panel
  const selectedChange = useMemo(() => {
    if (!selectedKey) return null;
    for (const c of filtered) {
      const key =
        c.objectType === "Table"
          ? c.objectName
          : `${c.objectName.split(".")[0]}.${c.objectType}.${c.objectName}`;
      if (key === selectedKey) return c;
    }
    return null;
  }, [selectedKey, filtered]);

  return (
    <div className="flex h-full flex-col overflow-hidden">
      {/* Summary bar */}
      <DiffSummaryBar
        title="Schema Diff"
        items={[
          {
            label: "Added",
            count: diff.summary.additions,
            color: changeColor.Added,
            active: changeFilter === "Added",
            onClick: () => setChangeFilter(changeFilter === "Added" ? "all" : "Added"),
          },
          {
            label: "Removed",
            count: diff.summary.removals,
            color: changeColor.Removed,
            active: changeFilter === "Removed",
            onClick: () => setChangeFilter(changeFilter === "Removed" ? "all" : "Removed"),
          },
          {
            label: "Modified",
            count: diff.summary.modifications,
            color: changeColor.Modified,
            active: changeFilter === "Modified",
            onClick: () => setChangeFilter(changeFilter === "Modified" ? "all" : "Modified"),
          },
          {
            label: "Unchanged",
            count: diff.summary.unchanged,
            color: changeColor.Unchanged,
            active: changeFilter === "Unchanged",
            onClick: () => setChangeFilter(changeFilter === "Unchanged" ? "all" : "Unchanged"),
          },
        ]}
      />

      {/* Filter bar */}
      <div className="flex items-center gap-2 border-b border-neutral-200 px-3 py-1.5 dark:border-neutral-700">
        <input
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Search objects..."
          className="w-48 rounded border border-neutral-300 bg-white px-2 py-1 text-xs outline-none focus:border-blue-500 dark:border-neutral-600 dark:bg-neutral-800 dark:text-neutral-200"
        />
        <select
          value={objectFilter}
          onChange={(e) => setObjectFilter(e.target.value as ObjectTypeFilter)}
          className="rounded border border-neutral-300 bg-white px-2 py-1 text-xs outline-none dark:border-neutral-600 dark:bg-neutral-800 dark:text-neutral-200"
        >
          <option value="all">All Types</option>
          <option value="Table">Tables</option>
          <option value="Column">Columns</option>
          <option value="Index">Indexes</option>
          <option value="Constraint">Constraints</option>
        </select>

        <div className="ml-auto flex items-center gap-1">
          <span className="text-[10px] text-neutral-400">
            {diffNodes.length > 0
              ? `${diffNavIndex + 1} / ${diffNodes.length} diffs`
              : "No diffs"}
          </span>
          <button
            onClick={() => navigateDiff(-1)}
            disabled={diffNodes.length === 0}
            className="rounded p-0.5 text-neutral-500 hover:bg-neutral-200 disabled:opacity-30 dark:hover:bg-neutral-700"
            title="Previous diff"
          >
            <svg className="h-3.5 w-3.5" viewBox="0 0 20 20" fill="currentColor">
              <path
                fillRule="evenodd"
                d="M14.77 12.79a.75.75 0 01-1.06-.02L10 8.832l-3.71 3.938a.75.75 0 11-1.08-1.04l4.25-4.5a.75.75 0 011.08 0l4.25 4.5a.75.75 0 01-.02 1.06z"
                clipRule="evenodd"
              />
            </svg>
          </button>
          <button
            onClick={() => navigateDiff(1)}
            disabled={diffNodes.length === 0}
            className="rounded p-0.5 text-neutral-500 hover:bg-neutral-200 disabled:opacity-30 dark:hover:bg-neutral-700"
            title="Next diff"
          >
            <svg className="h-3.5 w-3.5" viewBox="0 0 20 20" fill="currentColor">
              <path
                fillRule="evenodd"
                d="M5.23 7.21a.75.75 0 011.06.02L10 11.168l3.71-3.938a.75.75 0 111.08 1.04l-4.25 4.5a.75.75 0 01-1.08 0l-4.25-4.5a.75.75 0 01.02-1.06z"
                clipRule="evenodd"
              />
            </svg>
          </button>
        </div>
      </div>

      {/* Main content: tree + detail pane */}
      <div className="flex flex-1 overflow-hidden">
        {/* Tree pane */}
        <div className="w-1/2 overflow-y-auto border-r border-neutral-200 dark:border-neutral-700">
          {/* Database headers */}
          <div className="flex border-b border-neutral-200 text-[10px] uppercase tracking-wider text-neutral-400 dark:border-neutral-700">
            <div className="flex-1 px-3 py-1">
              Source: <span className="font-medium text-neutral-600 dark:text-neutral-300">{diff.sourceDatabase}</span>
            </div>
          </div>
          <div className="py-1">{renderNodes(tree, 0)}</div>
          {tree.length === 0 && (
            <div className="px-3 py-6 text-center text-xs text-neutral-400">
              No changes match the current filters.
            </div>
          )}
        </div>

        {/* Detail pane */}
        <div className="w-1/2 overflow-y-auto">
          <div className="flex border-b border-neutral-200 text-[10px] uppercase tracking-wider text-neutral-400 dark:border-neutral-700">
            <div className="flex-1 px-3 py-1">
              Target: <span className="font-medium text-neutral-600 dark:text-neutral-300">{diff.targetDatabase}</span>
            </div>
          </div>
          {selectedChange ? (
            <DetailPanel change={selectedChange} />
          ) : (
            <div className="flex h-40 items-center justify-center text-xs text-neutral-400">
              Select an item to view details.
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
