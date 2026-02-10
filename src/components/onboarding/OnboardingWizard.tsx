import { useState } from "react";
import {
  useConnectionStore,
  type DatabaseEngine,
} from "../../stores/connectionStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { useTourStore } from "../../stores/tourStore";

const STEPS = [
  { number: 1, label: "Welcome" },
  { number: 2, label: "Connection" },
  { number: 3, label: "Done" },
] as const;

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

export default function OnboardingWizard() {
  const { addConnection } = useConnectionStore();
  const { setHasCompletedOnboarding } = useSettingsStore();
  const startTour = useTourStore((s) => s.startTour);

  const [step, setStep] = useState(1);
  const [engine, setEngine] = useState<DatabaseEngine>("PostgreSql");
  const [host, setHost] = useState("localhost");
  const [port, setPort] = useState<number | "">(5432);
  const [database, setDatabase] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [errors, setErrors] = useState<string[]>([]);

  const handleEngineChange = (e: DatabaseEngine) => {
    setEngine(e);
    const dp = defaultPorts[e];
    if (dp) setPort(dp);
  };

  const handleCreateConnection = () => {
    const errs: string[] = [];
    if (engine !== "Sqlite") {
      if (!host.trim()) errs.push("Host is required.");
      if (!port) errs.push("Port is required.");
    }
    if (!database.trim()) errs.push("Database name is required.");

    if (errs.length > 0) {
      setErrors(errs);
      return;
    }

    addConnection({
      name: database.trim() || `${engine} Connection`,
      engine,
      host: host.trim() || undefined,
      port: port || undefined,
      database: database.trim() || undefined,
      username: username.trim() || undefined,
      password: password || undefined,
      readOnly: true,
    });

    setStep(3);
  };

  const handleFinish = () => {
    setHasCompletedOnboarding(true);
    setTimeout(() => startTour(), 300);
  };

  const handleSkip = () => {
    setHasCompletedOnboarding(true);
    setTimeout(() => startTour(), 300);
  };

  return (
    <div className="flex h-screen w-screen items-center justify-center bg-neutral-100 dark:bg-neutral-900">
      <div className="w-full max-w-lg rounded-xl border border-neutral-200 bg-white p-8 shadow-lg dark:border-neutral-700 dark:bg-neutral-800">
        {/* Stepper */}
        <div className="mb-8 flex items-center justify-center">
          {STEPS.map((s, idx) => {
            const isActive = step === s.number;
            const isCompleted = step > s.number;
            const isLast = idx === STEPS.length - 1;

            return (
              <div key={s.number} className="flex items-center">
                <div className="flex items-center gap-2">
                  <div
                    className={`flex h-7 w-7 items-center justify-center rounded-full text-xs font-bold transition-colors ${
                      isActive
                        ? "bg-blue-600 text-white"
                        : isCompleted
                          ? "bg-green-500 text-white"
                          : "bg-neutral-200 text-neutral-500 dark:bg-neutral-700 dark:text-neutral-400"
                    }`}
                  >
                    {isCompleted ? (
                      <svg
                        className="h-3.5 w-3.5"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                        strokeWidth={3}
                      >
                        <path
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          d="M4.5 12.75l6 6 9-13.5"
                        />
                      </svg>
                    ) : (
                      s.number
                    )}
                  </div>
                  <span
                    className={`text-xs font-medium ${
                      isActive
                        ? "text-blue-600 dark:text-blue-400"
                        : isCompleted
                          ? "text-green-600 dark:text-green-400"
                          : "text-neutral-400 dark:text-neutral-500"
                    }`}
                  >
                    {s.label}
                  </span>
                </div>
                {!isLast && (
                  <div
                    className={`mx-4 h-px w-12 ${
                      isCompleted
                        ? "bg-green-400 dark:bg-green-600"
                        : "bg-neutral-200 dark:bg-neutral-700"
                    }`}
                  />
                )}
              </div>
            );
          })}
        </div>

        {/* Step 1: Welcome */}
        {step === 1 && (
          <div className="text-center">
            <h1 className="mb-2 text-xl font-bold text-neutral-800 dark:text-neutral-100">
              Welcome to Upsert
            </h1>
            <p className="mb-6 text-sm text-neutral-500 dark:text-neutral-400">
              Upsert helps you compare, migrate, and synchronize databases with ease.
              Set up your first connection to get started.
            </p>
            <div className="flex justify-center gap-3">
              <button
                onClick={handleSkip}
                className="rounded border border-neutral-300 px-4 py-2 text-xs font-medium text-neutral-700 hover:bg-neutral-100 dark:border-neutral-600 dark:text-neutral-300 dark:hover:bg-neutral-700"
              >
                Skip Setup
              </button>
              <button
                onClick={() => setStep(2)}
                className="rounded bg-blue-600 px-6 py-2 text-xs font-medium text-white hover:bg-blue-700"
              >
                Get Started
              </button>
            </div>
          </div>
        )}

        {/* Step 2: Create Connection */}
        {step === 2 && (
          <div>
            <h2 className="mb-1 text-sm font-semibold text-neutral-800 dark:text-neutral-100">
              Create Your First Connection
            </h2>
            <p className="mb-4 text-xs text-neutral-500 dark:text-neutral-400">
              Enter your database details below. You can add more connections later.
            </p>

            {errors.length > 0 && (
              <div className="mb-3 rounded border border-red-300 bg-red-50 px-3 py-2 text-xs text-red-700 dark:border-red-700 dark:bg-red-900/30 dark:text-red-300">
                {errors.map((err, i) => (
                  <div key={i}>{err}</div>
                ))}
              </div>
            )}

            <div className="space-y-3">
              <div>
                <label className="mb-1 block text-xs font-medium text-neutral-600 dark:text-neutral-400">
                  Engine
                </label>
                <select
                  value={engine}
                  onChange={(e) => handleEngineChange(e.target.value as DatabaseEngine)}
                  className="input-field"
                >
                  {engines.map((eng) => (
                    <option key={eng} value={eng}>
                      {eng}
                    </option>
                  ))}
                </select>
              </div>

              {engine !== "Sqlite" && (
                <>
                  <div className="flex gap-2">
                    <div className="flex-1">
                      <label className="mb-1 block text-xs font-medium text-neutral-600 dark:text-neutral-400">
                        Host
                      </label>
                      <input
                        value={host}
                        onChange={(e) => setHost(e.target.value)}
                        className="input-field"
                        placeholder="localhost"
                      />
                    </div>
                    <div className="w-24">
                      <label className="mb-1 block text-xs font-medium text-neutral-600 dark:text-neutral-400">
                        Port
                      </label>
                      <input
                        type="number"
                        value={port}
                        onChange={(e) =>
                          setPort(e.target.value ? Number(e.target.value) : "")
                        }
                        className="input-field"
                      />
                    </div>
                  </div>
                </>
              )}

              <div>
                <label className="mb-1 block text-xs font-medium text-neutral-600 dark:text-neutral-400">
                  Database
                </label>
                <input
                  value={database}
                  onChange={(e) => setDatabase(e.target.value)}
                  className="input-field"
                  placeholder="my_database"
                />
              </div>

              <div>
                <label className="mb-1 block text-xs font-medium text-neutral-600 dark:text-neutral-400">
                  Username
                </label>
                <input
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                  className="input-field"
                  placeholder="postgres"
                />
              </div>

              <div>
                <label className="mb-1 block text-xs font-medium text-neutral-600 dark:text-neutral-400">
                  Password
                </label>
                <input
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  className="input-field"
                  placeholder="Optional"
                />
              </div>
            </div>

            <div className="mt-5 flex justify-between">
              <button
                onClick={() => setStep(1)}
                className="rounded border border-neutral-300 px-3 py-1.5 text-xs text-neutral-700 hover:bg-neutral-100 dark:border-neutral-600 dark:text-neutral-300 dark:hover:bg-neutral-700"
              >
                Back
              </button>
              <div className="flex gap-2">
                <button
                  onClick={handleSkip}
                  className="rounded border border-neutral-300 px-3 py-1.5 text-xs text-neutral-700 hover:bg-neutral-100 dark:border-neutral-600 dark:text-neutral-300 dark:hover:bg-neutral-700"
                >
                  Skip
                </button>
                <button
                  onClick={handleCreateConnection}
                  className="rounded bg-blue-600 px-4 py-1.5 text-xs font-medium text-white hover:bg-blue-700"
                >
                  Create Connection
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Step 3: Done */}
        {step === 3 && (
          <div className="text-center">
            <div className="mb-4 flex justify-center">
              <div className="rounded-full bg-green-100 p-3 dark:bg-green-900/30">
                <svg
                  className="h-8 w-8 text-green-600 dark:text-green-400"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                  strokeWidth={2}
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    d="M9 12.75L11.25 15 15 9.75M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                  />
                </svg>
              </div>
            </div>
            <h2 className="mb-2 text-lg font-bold text-neutral-800 dark:text-neutral-100">
              You're All Set!
            </h2>
            <p className="mb-6 text-sm text-neutral-500 dark:text-neutral-400">
              Your connection has been created. You can now start comparing and migrating
              databases.
            </p>
            <button
              onClick={handleFinish}
              className="rounded bg-blue-600 px-6 py-2 text-xs font-medium text-white hover:bg-blue-700"
            >
              Open Dashboard
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
