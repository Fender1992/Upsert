import { useState, useCallback } from "react";
import {
  useConnectionStore,
  type ConnectionProfile,
} from "../../../stores/connectionStore";
import { useMigrationStore } from "../../../stores/migrationStore";
import {
  testConnection,
  connectDatabase,
  type ConnectionConfigDto,
} from "../../../lib/tauriCommands";

const statusColors: Record<ConnectionProfile["status"], string> = {
  connected: "bg-green-500",
  connecting: "bg-amber-500 animate-pulse",
  disconnected: "bg-neutral-400",
  error: "bg-red-500",
};

const statusLabels: Record<ConnectionProfile["status"], string> = {
  connected: "Connected",
  connecting: "Connecting...",
  disconnected: "Disconnected",
  error: "Error",
};

function buildConfigDto(conn: ConnectionProfile): ConnectionConfigDto {
  return {
    engine: conn.engine,
    host: conn.host,
    port: conn.port,
    database: conn.database,
    username: conn.username,
    password: conn.password,
    filePath: conn.filePath,
    readOnly: conn.readOnly,
  };
}

export default function SelectSource() {
  const { connections, setConnectionStatus } = useConnectionStore();
  const { sourceConnectionId, setSourceConnection } = useMigrationStore();
  const [testingId, setTestingId] = useState<string | null>(null);

  const handleTestConnection = useCallback(
    async (id: string) => {
      const conn = connections.find((c) => c.id === id);
      if (!conn) return;

      setTestingId(id);
      setConnectionStatus(id, "connecting");

      try {
        const config = buildConfigDto(conn);
        await testConnection(config);
        // Also register in the backend registry
        await connectDatabase(id, config);
        setConnectionStatus(id, "connected");
      } catch (err) {
        setConnectionStatus(id, "error", String(err));
      } finally {
        setTestingId(null);
      }
    },
    [connections, setConnectionStatus],
  );

  if (connections.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-16">
        <svg
          className="mb-3 h-12 w-12 text-neutral-300 dark:text-neutral-600"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
          strokeWidth={1.5}
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75"
          />
        </svg>
        <p className="text-sm font-medium text-neutral-500 dark:text-neutral-400">
          No connections available
        </p>
        <p className="mt-1 text-xs text-neutral-400 dark:text-neutral-500">
          Create a connection in the sidebar before starting a migration.
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div>
        <h3 className="text-sm font-semibold text-neutral-800 dark:text-neutral-100">
          Select Source Connection
        </h3>
        <p className="mt-1 text-xs text-neutral-500 dark:text-neutral-400">
          Choose the database you want to migrate data from.
        </p>
      </div>

      <div className="space-y-2">
        {connections.map((conn) => {
          const isSelected = conn.id === sourceConnectionId;
          const isTesting = testingId === conn.id;

          return (
            <div
              key={conn.id}
              onClick={() => setSourceConnection(conn.id)}
              className={`flex cursor-pointer items-center gap-3 rounded-lg border px-4 py-3 transition-colors ${
                isSelected
                  ? "border-blue-500 bg-blue-50 dark:border-blue-500 dark:bg-blue-950/30"
                  : "border-neutral-200 bg-white hover:border-neutral-300 hover:bg-neutral-50 dark:border-neutral-700 dark:bg-neutral-800 dark:hover:border-neutral-600 dark:hover:bg-neutral-750"
              }`}
            >
              {/* Radio indicator */}
              <div
                className={`flex h-4 w-4 shrink-0 items-center justify-center rounded-full border-2 ${
                  isSelected
                    ? "border-blue-500"
                    : "border-neutral-300 dark:border-neutral-600"
                }`}
              >
                {isSelected && (
                  <div className="h-2 w-2 rounded-full bg-blue-500" />
                )}
              </div>

              {/* Connection info */}
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="text-xs font-medium text-neutral-800 dark:text-neutral-100">
                    {conn.name}
                  </span>
                  <span className="rounded bg-neutral-100 px-1.5 py-0.5 text-[10px] font-medium text-neutral-500 dark:bg-neutral-700 dark:text-neutral-400">
                    {conn.engine}
                  </span>
                </div>
                <div className="mt-0.5 truncate text-[11px] text-neutral-400 dark:text-neutral-500">
                  {conn.engine === "Sqlite"
                    ? conn.filePath
                    : `${conn.host ?? ""}${conn.port ? `:${conn.port}` : ""}${conn.database ? ` / ${conn.database}` : ""}`}
                </div>
              </div>

              {/* Status indicator */}
              <div className="flex items-center gap-2">
                <div className="flex items-center gap-1.5">
                  <div
                    className={`h-2 w-2 rounded-full ${statusColors[conn.status]}`}
                  />
                  <span className="text-[10px] text-neutral-500 dark:text-neutral-400">
                    {statusLabels[conn.status]}
                  </span>
                </div>

                {/* Test connection button */}
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    handleTestConnection(conn.id);
                  }}
                  disabled={isTesting}
                  className="rounded border border-neutral-300 px-2 py-1 text-[10px] font-medium text-neutral-600 hover:bg-neutral-100 disabled:opacity-50 dark:border-neutral-600 dark:text-neutral-400 dark:hover:bg-neutral-700"
                >
                  {isTesting ? "Testing..." : "Test"}
                </button>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
