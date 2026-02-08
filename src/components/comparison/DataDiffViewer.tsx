import { useState, useMemo, useCallback, useRef, useEffect } from "react";
import {
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
  createColumnHelper,
  flexRender,
  type SortingState,
  type ColumnDef,
} from "@tanstack/react-table";
import { useVirtualizer } from "@tanstack/react-virtual";
import type {
  DataDiffDataSet,
  DataDiffRow,
  DataDiffResult,
  RowStatus,
  RowStatusFilter,
} from "../../types/comparison";
import DiffSummaryBar from "./DiffSummaryBar";

// -- Helpers ------------------------------------------------------------------

const statusSymbol: Record<RowStatus, string> = {
  matched: "=",
  inserted: "+",
  updated: "~",
  deleted: "-",
  error: "!",
};

const statusColor: Record<RowStatus, string> = {
  matched: "#9ca3af",
  inserted: "#22c55e",
  updated: "#f59e0b",
  deleted: "#ef4444",
  error: "#dc2626",
};

const statusRowBg: Record<RowStatus, string> = {
  matched: "",
  inserted: "bg-green-50/50 dark:bg-green-950/20",
  updated: "bg-amber-50/50 dark:bg-amber-950/20",
  deleted: "bg-red-50/50 dark:bg-red-950/20",
  error: "bg-red-100/50 dark:bg-red-950/30",
};

const statusTextColor: Record<RowStatus, string> = {
  matched: "text-neutral-400",
  inserted: "text-green-600 dark:text-green-400",
  updated: "text-amber-600 dark:text-amber-400",
  deleted: "text-red-600 dark:text-red-400",
  error: "text-red-700 dark:text-red-300",
};

const ROW_HEIGHT = 28;
const PAGE_SIZE = 100;

// -- Main component -----------------------------------------------------------

interface Props {
  summary: DataDiffResult;
  data: DataDiffDataSet;
}

export default function DataDiffViewer({ summary, data }: Props) {
  const [statusFilter, setStatusFilter] = useState<RowStatusFilter>("all");
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchText, setSearchText] = useState("");
  const [sorting, setSorting] = useState<SortingState>([]);
  const [page, setPage] = useState(0);

  const searchInputRef = useRef<HTMLInputElement>(null);
  const parentRef = useRef<HTMLDivElement>(null);

  // Ctrl+F to open inline search
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "f") {
        e.preventDefault();
        setSearchOpen(true);
        requestAnimationFrame(() => searchInputRef.current?.focus());
      }
      if (e.key === "Escape" && searchOpen) {
        setSearchOpen(false);
        setSearchText("");
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [searchOpen]);

  // Filter rows by status
  const filteredRows = useMemo(() => {
    let rows = data.rows;
    if (statusFilter !== "all") {
      rows = rows.filter((r) => r._status === statusFilter);
    }
    if (searchText) {
      const q = searchText.toLowerCase();
      rows = rows.filter((r) =>
        data.columns.some((col) => {
          const val = r[col];
          return val != null && String(val).toLowerCase().includes(q);
        }),
      );
    }
    return rows;
  }, [data.rows, data.columns, statusFilter, searchText]);

  // Pagination
  const totalPages = Math.max(1, Math.ceil(filteredRows.length / PAGE_SIZE));
  const safeCurrentPage = Math.min(page, totalPages - 1);
  const pagedRows = useMemo(
    () =>
      filteredRows.slice(
        safeCurrentPage * PAGE_SIZE,
        (safeCurrentPage + 1) * PAGE_SIZE,
      ),
    [filteredRows, safeCurrentPage],
  );

  // Build columns
  const columnHelper = createColumnHelper<DataDiffRow>();

  const columns = useMemo<ColumnDef<DataDiffRow, unknown>[]>(() => {
    const statusCol = columnHelper.display({
      id: "_status_display",
      header: "",
      size: 32,
      cell: ({ row }) => {
        const s = row.original._status;
        return (
          <span
            className={`font-mono font-bold ${statusTextColor[s]}`}
            title={s}
          >
            {statusSymbol[s]}
          </span>
        );
      },
    });

    const dataCols = data.columns.map((colName) =>
      columnHelper.accessor((row) => row[colName], {
        id: colName,
        header: colName,
        size: 150,
        cell: ({ row, getValue }) => {
          const val = getValue();
          const isChanged = row.original._changedColumns?.includes(colName);
          return (
            <span
              className={
                isChanged
                  ? "rounded bg-amber-200/60 px-0.5 dark:bg-amber-800/40"
                  : ""
              }
            >
              {val == null ? (
                <span className="text-neutral-300 dark:text-neutral-600">NULL</span>
              ) : (
                String(val)
              )}
            </span>
          );
        },
      }),
    );

    return [statusCol, ...dataCols] as ColumnDef<DataDiffRow, unknown>[];
  }, [data.columns, columnHelper]);

  const table = useReactTable({
    data: pagedRows,
    columns,
    state: { sorting },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    columnResizeMode: "onChange",
  });

  const { rows: tableRows } = table.getRowModel();

  const virtualizer = useVirtualizer({
    count: tableRows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 20,
  });

  const toggleFilter = useCallback(
    (status: RowStatus) => {
      setStatusFilter((prev) => (prev === status ? "all" : status));
      setPage(0);
    },
    [],
  );

  return (
    <div className="flex h-full flex-col overflow-hidden">
      {/* Summary bar */}
      <DiffSummaryBar
        title="Data Diff"
        items={[
          {
            label: "Matched",
            count: summary.matchedRows,
            color: statusColor.matched,
            active: statusFilter === "matched",
            onClick: () => toggleFilter("matched"),
          },
          {
            label: "Inserted",
            count: summary.insertedCount,
            color: statusColor.inserted,
            active: statusFilter === "inserted",
            onClick: () => toggleFilter("inserted"),
          },
          {
            label: "Updated",
            count: summary.updatedCount,
            color: statusColor.updated,
            active: statusFilter === "updated",
            onClick: () => toggleFilter("updated"),
          },
          {
            label: "Deleted",
            count: summary.deletedCount,
            color: statusColor.deleted,
            active: statusFilter === "deleted",
            onClick: () => toggleFilter("deleted"),
          },
          {
            label: "Errors",
            count: summary.errorCount,
            color: statusColor.error,
            active: statusFilter === "error",
            onClick: () => toggleFilter("error"),
          },
        ]}
      />

      {/* Inline search */}
      {searchOpen && (
        <div className="flex items-center gap-2 border-b border-neutral-200 bg-neutral-50 px-3 py-1 dark:border-neutral-700 dark:bg-neutral-850">
          <svg className="h-3.5 w-3.5 text-neutral-400" viewBox="0 0 20 20" fill="currentColor">
            <path
              fillRule="evenodd"
              d="M9 3.5a5.5 5.5 0 100 11 5.5 5.5 0 000-11zM2 9a7 7 0 1112.452 4.391l3.328 3.329a.75.75 0 11-1.06 1.06l-3.329-3.328A7 7 0 012 9z"
              clipRule="evenodd"
            />
          </svg>
          <input
            ref={searchInputRef}
            value={searchText}
            onChange={(e) => {
              setSearchText(e.target.value);
              setPage(0);
            }}
            placeholder="Find in data..."
            className="flex-1 bg-transparent text-xs outline-none dark:text-neutral-200"
          />
          <span className="text-[10px] text-neutral-400">
            {filteredRows.length} results
          </span>
          <button
            onClick={() => {
              setSearchOpen(false);
              setSearchText("");
            }}
            className="text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-200"
          >
            <svg className="h-3.5 w-3.5" viewBox="0 0 20 20" fill="currentColor">
              <path d="M6.28 5.22a.75.75 0 00-1.06 1.06L8.94 10l-3.72 3.72a.75.75 0 101.06 1.06L10 11.06l3.72 3.72a.75.75 0 101.06-1.06L11.06 10l3.72-3.72a.75.75 0 00-1.06-1.06L10 8.94 6.28 5.22z" />
            </svg>
          </button>
        </div>
      )}

      {/* Table header + source/target labels */}
      <div className="flex items-center gap-2 border-b border-neutral-200 px-3 py-1 text-[10px] uppercase tracking-wider text-neutral-400 dark:border-neutral-700">
        <span>
          Source: <span className="font-medium text-neutral-600 dark:text-neutral-300">{summary.sourceTable}</span>
        </span>
        <span className="text-neutral-300 dark:text-neutral-600">vs</span>
        <span>
          Target: <span className="font-medium text-neutral-600 dark:text-neutral-300">{summary.targetTable}</span>
        </span>
      </div>

      {/* Virtualized table */}
      <div ref={parentRef} className="flex-1 overflow-auto">
        <table className="w-full text-xs" style={{ tableLayout: "fixed" }}>
          <thead className="sticky top-0 z-10 bg-neutral-100 dark:bg-neutral-800">
            {table.getHeaderGroups().map((hg) => (
              <tr key={hg.id}>
                {hg.headers.map((header) => (
                  <th
                    key={header.id}
                    className="relative border-b border-r border-neutral-200 px-2 py-1.5 text-left font-medium text-neutral-600 dark:border-neutral-700 dark:text-neutral-400"
                    style={{ width: header.getSize() }}
                  >
                    {header.isPlaceholder ? null : (
                      <div
                        className={`flex items-center gap-1 ${header.column.getCanSort() ? "cursor-pointer select-none" : ""}`}
                        onClick={header.column.getToggleSortingHandler()}
                      >
                        {flexRender(header.column.columnDef.header, header.getContext())}
                        {{
                          asc: " \u2191",
                          desc: " \u2193",
                        }[header.column.getIsSorted() as string] ?? null}
                      </div>
                    )}
                    {/* Resize handle */}
                    <div
                      onMouseDown={header.getResizeHandler()}
                      onTouchStart={header.getResizeHandler()}
                      className="absolute right-0 top-0 h-full w-1 cursor-col-resize hover:bg-blue-500"
                    />
                  </th>
                ))}
              </tr>
            ))}
          </thead>
          <tbody>
            {virtualizer.getVirtualItems().length === 0 ? (
              <tr>
                <td
                  colSpan={columns.length}
                  className="py-8 text-center text-neutral-400"
                >
                  No rows match the current filters.
                </td>
              </tr>
            ) : (
              <>
                {/* Top spacer */}
                {virtualizer.getVirtualItems()[0]?.start > 0 && (
                  <tr>
                    <td
                      colSpan={columns.length}
                      style={{ height: virtualizer.getVirtualItems()[0].start }}
                    />
                  </tr>
                )}
                {virtualizer.getVirtualItems().map((vRow) => {
                  const row = tableRows[vRow.index];
                  const status = row.original._status;
                  return (
                    <tr
                      key={row.id}
                      className={`border-b border-neutral-100 dark:border-neutral-800 ${statusRowBg[status]}`}
                      style={{ height: ROW_HEIGHT }}
                    >
                      {row.getVisibleCells().map((cell) => (
                        <td
                          key={cell.id}
                          className="overflow-hidden truncate whitespace-nowrap border-r border-neutral-100 px-2 dark:border-neutral-800"
                          style={{ width: cell.column.getSize() }}
                        >
                          {flexRender(cell.column.columnDef.cell, cell.getContext())}
                        </td>
                      ))}
                    </tr>
                  );
                })}
                {/* Bottom spacer */}
                {(() => {
                  const items = virtualizer.getVirtualItems();
                  const lastItem = items[items.length - 1];
                  const remaining = lastItem
                    ? virtualizer.getTotalSize() - lastItem.end
                    : 0;
                  return remaining > 0 ? (
                    <tr>
                      <td colSpan={columns.length} style={{ height: remaining }} />
                    </tr>
                  ) : null;
                })()}
              </>
            )}
          </tbody>
        </table>
      </div>

      {/* Pagination footer */}
      <div className="flex items-center justify-between border-t border-neutral-200 bg-neutral-50 px-3 py-1 text-xs dark:border-neutral-700 dark:bg-neutral-850">
        <span className="text-neutral-500">
          Showing {safeCurrentPage * PAGE_SIZE + 1}-
          {Math.min((safeCurrentPage + 1) * PAGE_SIZE, filteredRows.length)} of{" "}
          {filteredRows.length} rows
        </span>
        <div className="flex items-center gap-1">
          <button
            onClick={() => setPage(0)}
            disabled={safeCurrentPage === 0}
            className="rounded px-1.5 py-0.5 text-neutral-500 hover:bg-neutral-200 disabled:opacity-30 dark:hover:bg-neutral-700"
          >
            First
          </button>
          <button
            onClick={() => setPage((p) => Math.max(0, p - 1))}
            disabled={safeCurrentPage === 0}
            className="rounded px-1.5 py-0.5 text-neutral-500 hover:bg-neutral-200 disabled:opacity-30 dark:hover:bg-neutral-700"
          >
            Prev
          </button>
          <span className="px-2 text-neutral-600 dark:text-neutral-400">
            Page {safeCurrentPage + 1} / {totalPages}
          </span>
          <button
            onClick={() => setPage((p) => Math.min(totalPages - 1, p + 1))}
            disabled={safeCurrentPage >= totalPages - 1}
            className="rounded px-1.5 py-0.5 text-neutral-500 hover:bg-neutral-200 disabled:opacity-30 dark:hover:bg-neutral-700"
          >
            Next
          </button>
          <button
            onClick={() => setPage(totalPages - 1)}
            disabled={safeCurrentPage >= totalPages - 1}
            className="rounded px-1.5 py-0.5 text-neutral-500 hover:bg-neutral-200 disabled:opacity-30 dark:hover:bg-neutral-700"
          >
            Last
          </button>
        </div>
      </div>
    </div>
  );
}
