import { useState, useRef, useEffect, useCallback } from "react";
import {
  useConnectionStore,
  type ConnectionProfile,
} from "../../stores/connectionStore";
import { useUiStore } from "../../stores/uiStore";

const engineIcons: Record<string, string> = {
  SqlServer: "SQL",
  PostgreSql: "PG",
  MySql: "My",
  Sqlite: "SL",
  Oracle: "Or",
  MongoDb: "Mo",
  CosmosDb: "Co",
};

interface ContextMenuState {
  x: number;
  y: number;
  connectionId: string;
}

export default function ConnectionSidebar({
  onNewConnection,
  onEditConnection,
}: {
  onNewConnection: () => void;
  onEditConnection: (id: string) => void;
}) {
  const {
    connections,
    activeConnectionId,
    setActiveConnection,
    setConnectionStatus,
    removeConnection,
  } = useConnectionStore();
  const { sidebarCollapsed, toggleSidebar, appendLog } = useUiStore();
  const [filter, setFilter] = useState("");
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  const filtered = connections.filter((c) =>
    c.name.toLowerCase().includes(filter.toLowerCase()),
  );

  const handleConnect = useCallback(
    (conn: ConnectionProfile) => {
      setActiveConnection(conn.id);
      setConnectionStatus(conn.id, "connected");
      appendLog(`Connected to ${conn.name}`);
    },
    [setActiveConnection, setConnectionStatus, appendLog],
  );

  const handleDisconnect = useCallback(
    (conn: ConnectionProfile) => {
      setConnectionStatus(conn.id, "disconnected");
      if (activeConnectionId === conn.id) setActiveConnection(null);
      appendLog(`Disconnected from ${conn.name}`);
    },
    [setConnectionStatus, activeConnectionId, setActiveConnection, appendLog],
  );

  useEffect(() => {
    const onClick = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setContextMenu(null);
      }
    };
    document.addEventListener("mousedown", onClick);
    return () => document.removeEventListener("mousedown", onClick);
  }, []);

  if (sidebarCollapsed) {
    return (
      <div className="flex w-10 flex-col items-center border-r border-neutral-300 bg-neutral-50 py-2 dark:border-neutral-700 dark:bg-neutral-900">
        <button
          onClick={toggleSidebar}
          className="rounded p-1 text-neutral-500 hover:bg-neutral-200 dark:hover:bg-neutral-700"
          title="Expand sidebar"
        >
          <svg className="h-4 w-4" viewBox="0 0 20 20" fill="currentColor">
            <path
              fillRule="evenodd"
              d="M7.21 14.77a.75.75 0 01.02-1.06L11.168 10 7.23 6.29a.75.75 0 111.04-1.08l4.5 4.25a.75.75 0 010 1.08l-4.5 4.25a.75.75 0 01-1.06-.02z"
              clipRule="evenodd"
            />
          </svg>
        </button>
      </div>
    );
  }

  return (
    <div data-tour="sidebar" className="flex w-[250px] flex-col border-r border-neutral-300 bg-neutral-50 dark:border-neutral-700 dark:bg-neutral-900">
      {/* Header */}
      <div className="flex h-9 items-center justify-between border-b border-neutral-200 px-3 dark:border-neutral-700">
        <span className="text-xs font-semibold text-neutral-700 dark:text-neutral-300">
          Connections
        </span>
        <button
          onClick={toggleSidebar}
          className="rounded p-0.5 text-neutral-500 hover:bg-neutral-200 dark:hover:bg-neutral-700"
          title="Collapse sidebar"
        >
          <svg className="h-4 w-4" viewBox="0 0 20 20" fill="currentColor">
            <path
              fillRule="evenodd"
              d="M12.79 5.23a.75.75 0 01-.02 1.06L8.832 10l3.938 3.71a.75.75 0 11-1.04 1.08l-4.5-4.25a.75.75 0 010-1.08l4.5-4.25a.75.75 0 011.06.02z"
              clipRule="evenodd"
            />
          </svg>
        </button>
      </div>

      {/* Search */}
      <div className="px-2 py-1.5">
        <input
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          placeholder="Filter..."
          className="w-full rounded border border-neutral-300 bg-white px-2 py-1 text-xs outline-none focus:border-blue-500 dark:border-neutral-600 dark:bg-neutral-800 dark:text-neutral-200"
        />
      </div>

      {/* New Connection button */}
      <div className="px-2 pb-1">
        <button
          data-tour="new-connection-btn"
          onClick={onNewConnection}
          className="w-full rounded bg-blue-600 px-2 py-1 text-xs font-medium text-white hover:bg-blue-700"
        >
          + New Connection
        </button>
      </div>

      {/* Connection list */}
      <div data-tour="connection-list" className="flex-1 overflow-y-auto">
        {filtered.length === 0 ? (
          <div className="px-3 py-4 text-center text-xs text-neutral-400">
            {connections.length === 0 ? "No connections yet." : "No matches."}
          </div>
        ) : (
          filtered.map((conn) => (
            <div
              key={conn.id}
              className={`group flex cursor-pointer items-center gap-2 px-3 py-1.5 text-xs ${
                conn.id === activeConnectionId
                  ? "bg-blue-600/10 text-blue-700 dark:text-blue-400"
                  : "text-neutral-700 hover:bg-neutral-200 dark:text-neutral-300 dark:hover:bg-neutral-800"
              }`}
              onClick={() => setActiveConnection(conn.id)}
              onDoubleClick={() => handleConnect(conn)}
              onContextMenu={(e) => {
                e.preventDefault();
                setContextMenu({ x: e.clientX, y: e.clientY, connectionId: conn.id });
              }}
            >
              <span className="shrink-0 rounded bg-neutral-200 px-1 py-0.5 font-mono text-[10px] text-neutral-600 dark:bg-neutral-700 dark:text-neutral-400">
                {engineIcons[conn.engine] ?? "DB"}
              </span>
              <span className="flex-1 truncate">{conn.name}</span>
              <span
                className={`h-2 w-2 shrink-0 rounded-full ${
                  conn.status === "connected"
                    ? "bg-green-500"
                    : conn.status === "error"
                      ? "bg-red-500"
                      : "bg-neutral-400"
                }`}
              />
            </div>
          ))
        )}
      </div>

      {/* Context menu */}
      {contextMenu && (
        <div
          ref={menuRef}
          className="fixed z-50 min-w-[140px] rounded border border-neutral-300 bg-white py-1 shadow-lg dark:border-neutral-600 dark:bg-neutral-800"
          style={{ left: contextMenu.x, top: contextMenu.y }}
        >
          {(() => {
            const conn = connections.find((c) => c.id === contextMenu.connectionId);
            if (!conn) return null;
            return (
              <>
                <button
                  className="block w-full px-3 py-1 text-left text-xs text-neutral-700 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-700"
                  onClick={() => {
                    handleConnect(conn);
                    setContextMenu(null);
                  }}
                >
                  Connect
                </button>
                <button
                  className="block w-full px-3 py-1 text-left text-xs text-neutral-700 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-700"
                  onClick={() => {
                    handleDisconnect(conn);
                    setContextMenu(null);
                  }}
                >
                  Disconnect
                </button>
                <button
                  className="block w-full px-3 py-1 text-left text-xs text-neutral-700 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-700"
                  onClick={() => {
                    onEditConnection(conn.id);
                    setContextMenu(null);
                  }}
                >
                  Edit
                </button>
                <hr className="my-1 border-neutral-200 dark:border-neutral-700" />
                <button
                  className="block w-full px-3 py-1 text-left text-xs text-red-600 hover:bg-neutral-100 dark:hover:bg-neutral-700"
                  onClick={() => {
                    removeConnection(conn.id);
                    appendLog(`Removed connection: ${conn.name}`);
                    setContextMenu(null);
                  }}
                >
                  Delete
                </button>
              </>
            );
          })()}
        </div>
      )}
    </div>
  );
}
