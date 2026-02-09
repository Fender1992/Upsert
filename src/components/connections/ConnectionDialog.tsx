import { useState, useEffect } from "react";
import {
  useConnectionStore,
  type DatabaseEngine,
  type ConnectionProfile,
} from "../../stores/connectionStore";
import { useUiStore } from "../../stores/uiStore";

const engines: DatabaseEngine[] = [
  "SqlServer",
  "PostgreSql",
  "MySql",
  "Sqlite",
  "Oracle",
  "MongoDb",
  "CosmosDb",
];

const defaultPorts: Partial<Record<DatabaseEngine, number>> = {
  SqlServer: 1433,
  PostgreSql: 5432,
  MySql: 3306,
  Oracle: 1521,
  MongoDb: 27017,
  CosmosDb: 443,
};

interface Props {
  editId?: string | null;
  onClose: () => void;
}

export default function ConnectionDialog({ editId, onClose }: Props) {
  const { connections, addConnection, updateConnection } = useConnectionStore();
  const { appendLog } = useUiStore();

  const existing = editId ? connections.find((c) => c.id === editId) : null;

  const [name, setName] = useState("");
  const [engine, setEngine] = useState<DatabaseEngine>("SqlServer");
  const [host, setHost] = useState("");
  const [port, setPort] = useState<number | "">(1433);
  const [database, setDatabase] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [filePath, setFilePath] = useState("");
  const [readOnly, setReadOnly] = useState(false);
  const [errors, setErrors] = useState<string[]>([]);

  useEffect(() => {
    if (existing) {
      setName(existing.name);
      setEngine(existing.engine);
      setHost(existing.host ?? "");
      setPort(existing.port ?? defaultPorts[existing.engine] ?? "");
      setDatabase(existing.database ?? "");
      setUsername(existing.username ?? "");
      setFilePath(existing.filePath ?? "");
      setReadOnly(existing.readOnly);
    }
  }, [existing]);

  const isSqlite = engine === "Sqlite";

  const validate = (): string[] => {
    const errs: string[] = [];
    if (!name.trim()) errs.push("Name is required.");
    if (isSqlite) {
      if (!filePath.trim()) errs.push("File path is required for SQLite.");
    } else {
      if (!host.trim()) errs.push("Host is required.");
      if (!port) errs.push("Port is required.");
    }
    return errs;
  };

  const handleSave = () => {
    const errs = validate();
    if (errs.length > 0) {
      setErrors(errs);
      return;
    }

    const profile: Omit<ConnectionProfile, "id" | "status"> = {
      name: name.trim(),
      engine,
      host: isSqlite ? undefined : host.trim(),
      port: isSqlite ? undefined : (port as number),
      database: database.trim() || undefined,
      username: username.trim() || undefined,
      password: password || undefined,
      filePath: isSqlite ? filePath.trim() : undefined,
      readOnly,
    };

    if (editId && existing) {
      updateConnection(editId, profile);
      appendLog(`Updated connection: ${name}`);
    } else {
      addConnection(profile);
      appendLog(`Created connection: ${name}`);
    }
    onClose();
  };

  const handleEngineChange = (e: DatabaseEngine) => {
    setEngine(e);
    const dp = defaultPorts[e];
    if (dp) setPort(dp);
  };

  return (
    <div
      className="fixed inset-0 z-40 flex items-center justify-center bg-black/40"
      onClick={onClose}
    >
      <div
        className="w-full max-w-md rounded-lg border border-neutral-300 bg-white p-5 shadow-xl dark:border-neutral-600 dark:bg-neutral-800"
        onClick={(e) => e.stopPropagation()}
      >
        <h2 className="mb-4 text-sm font-semibold text-neutral-800 dark:text-neutral-100">
          {editId ? "Edit Connection" : "New Connection"}
        </h2>

        {errors.length > 0 && (
          <div className="mb-3 rounded border border-red-300 bg-red-50 px-3 py-2 text-xs text-red-700 dark:border-red-700 dark:bg-red-900/30 dark:text-red-300">
            {errors.map((err, i) => (
              <div key={i}>{err}</div>
            ))}
          </div>
        )}

        <div className="space-y-3">
          <Field label="Name">
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="input-field"
              placeholder="My Database"
            />
          </Field>

          <Field label="Engine">
            <select
              value={engine}
              onChange={(e) =>
                handleEngineChange(e.target.value as DatabaseEngine)
              }
              className="input-field"
            >
              {engines.map((eng) => (
                <option key={eng} value={eng}>
                  {eng}
                </option>
              ))}
            </select>
          </Field>

          {isSqlite ? (
            <Field label="File Path">
              <input
                value={filePath}
                onChange={(e) => setFilePath(e.target.value)}
                className="input-field"
                placeholder="/path/to/db.sqlite"
              />
            </Field>
          ) : (
            <>
              <div className="flex gap-2">
                <Field label="Host" className="flex-1">
                  <input
                    value={host}
                    onChange={(e) => setHost(e.target.value)}
                    className="input-field"
                    placeholder="localhost"
                  />
                </Field>
                <Field label="Port" className="w-24">
                  <input
                    type="number"
                    value={port}
                    onChange={(e) =>
                      setPort(e.target.value ? Number(e.target.value) : "")
                    }
                    className="input-field"
                  />
                </Field>
              </div>

              <Field label="Database">
                <input
                  value={database}
                  onChange={(e) => setDatabase(e.target.value)}
                  className="input-field"
                  placeholder="my_database"
                />
              </Field>

              <div className="flex gap-2">
                <Field label="Username" className="flex-1">
                  <input
                    value={username}
                    onChange={(e) => setUsername(e.target.value)}
                    className="input-field"
                    placeholder="sa"
                  />
                </Field>
                <Field label="Password" className="flex-1">
                  <input
                    type="password"
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                    className="input-field"
                    placeholder="********"
                  />
                </Field>
              </div>
            </>
          )}

          <label className="flex items-center gap-2 text-xs text-neutral-700 dark:text-neutral-300">
            <input
              type="checkbox"
              checked={readOnly}
              onChange={(e) => setReadOnly(e.target.checked)}
              className="rounded border-neutral-300"
            />
            Read Only
          </label>
        </div>

        <div className="mt-5 flex justify-end gap-2">
          <button
            onClick={onClose}
            className="rounded border border-neutral-300 px-3 py-1.5 text-xs text-neutral-700 hover:bg-neutral-100 dark:border-neutral-600 dark:text-neutral-300 dark:hover:bg-neutral-700"
          >
            Cancel
          </button>
          <button
            onClick={handleSave}
            className="rounded bg-blue-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-blue-700"
          >
            {editId ? "Save Changes" : "Create"}
          </button>
        </div>
      </div>
    </div>
  );
}

function Field({
  label,
  children,
  className,
}: {
  label: string;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div className={className}>
      <label className="mb-1 block text-xs font-medium text-neutral-600 dark:text-neutral-400">
        {label}
      </label>
      {children}
    </div>
  );
}
