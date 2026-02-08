import { useState } from "react";
import type { Job, JobExecution } from "./JobList";

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  const remainder = s % 60;
  return `${m}m ${remainder}s`;
}

function formatTime(ts: number): string {
  return new Date(ts).toLocaleString();
}

const statusStyles: Record<JobExecution["status"], string> = {
  Completed: "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400",
  Failed: "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400",
  Running: "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400",
  Cancelled: "bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400",
};

interface Props {
  job: Job;
  onBack: () => void;
  onClearHistory: () => void;
}

export default function JobHistory({ job, onBack, onClearHistory }: Props) {
  const [selectedExecution, setSelectedExecution] = useState<JobExecution | null>(null);

  return (
    <div className="flex h-full flex-col overflow-hidden">
      {/* Header */}
      <div className="flex shrink-0 items-center justify-between border-b border-neutral-200 bg-neutral-50 px-6 py-3 dark:border-neutral-700 dark:bg-neutral-850">
        <div className="flex items-center gap-3">
          <button
            onClick={onBack}
            className="rounded p-1 text-neutral-500 hover:bg-neutral-200 dark:hover:bg-neutral-700"
            title="Back to Jobs"
          >
            <svg className="h-4 w-4" viewBox="0 0 20 20" fill="currentColor">
              <path
                fillRule="evenodd"
                d="M12.79 5.23a.75.75 0 01-.02 1.06L8.832 10l3.938 3.71a.75.75 0 11-1.04 1.08l-4.5-4.25a.75.75 0 010-1.08l4.5-4.25a.75.75 0 011.06.02z"
                clipRule="evenodd"
              />
            </svg>
          </button>
          <h2 className="text-sm font-semibold text-neutral-800 dark:text-neutral-100">
            History: {job.name}
          </h2>
        </div>
        {job.history.length > 0 && (
          <button
            onClick={onClearHistory}
            className="rounded border border-neutral-300 px-3 py-1.5 text-xs text-neutral-700 hover:bg-neutral-100 dark:border-neutral-600 dark:text-neutral-300 dark:hover:bg-neutral-700"
          >
            Clear History
          </button>
        )}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto px-6 py-4">
        {job.history.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-16 text-center">
            <p className="text-sm text-neutral-500 dark:text-neutral-400">
              No execution history yet.
            </p>
            <p className="mt-1 text-xs text-neutral-400 dark:text-neutral-500">
              Run the job to see results here.
            </p>
          </div>
        ) : (
          <table className="w-full text-xs">
            <thead>
              <tr className="border-b border-neutral-200 text-left text-neutral-500 dark:border-neutral-700 dark:text-neutral-400">
                <th className="pb-2 pr-4 font-medium">Start Time</th>
                <th className="pb-2 pr-4 font-medium">Duration</th>
                <th className="pb-2 pr-4 font-medium">Status</th>
                <th className="pb-2 pr-4 font-medium">Summary</th>
                <th className="pb-2 font-medium">Error</th>
              </tr>
            </thead>
            <tbody>
              {job.history.map((exec) => (
                <tr
                  key={exec.id}
                  className="cursor-pointer border-b border-neutral-100 text-neutral-700 hover:bg-neutral-50 dark:border-neutral-800 dark:text-neutral-300 dark:hover:bg-neutral-800/50"
                  onClick={() => setSelectedExecution(exec)}
                >
                  <td className="py-2.5 pr-4">{formatTime(exec.startTime)}</td>
                  <td className="py-2.5 pr-4">{formatDuration(exec.durationMs)}</td>
                  <td className="py-2.5 pr-4">
                    <span
                      className={`inline-block rounded-full px-2 py-0.5 text-[10px] font-medium ${statusStyles[exec.status]}`}
                    >
                      {exec.status}
                    </span>
                  </td>
                  <td className="py-2.5 pr-4">{exec.summary}</td>
                  <td className="py-2.5 truncate text-red-600 dark:text-red-400">
                    {exec.error ?? "--"}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {/* Detail overlay */}
      {selectedExecution && (
        <div
          className="fixed inset-0 z-40 flex items-center justify-center bg-black/40"
          onClick={() => setSelectedExecution(null)}
        >
          <div
            className="w-full max-w-lg rounded-lg border border-neutral-300 bg-white p-5 shadow-xl dark:border-neutral-600 dark:bg-neutral-800"
            onClick={(e) => e.stopPropagation()}
          >
            <h3 className="mb-3 text-sm font-semibold text-neutral-800 dark:text-neutral-100">
              Execution Details
            </h3>
            <div className="space-y-2 text-xs text-neutral-700 dark:text-neutral-300">
              <div className="flex justify-between">
                <span className="font-medium text-neutral-500 dark:text-neutral-400">
                  Start Time
                </span>
                <span>{formatTime(selectedExecution.startTime)}</span>
              </div>
              <div className="flex justify-between">
                <span className="font-medium text-neutral-500 dark:text-neutral-400">
                  Duration
                </span>
                <span>{formatDuration(selectedExecution.durationMs)}</span>
              </div>
              <div className="flex justify-between">
                <span className="font-medium text-neutral-500 dark:text-neutral-400">Status</span>
                <span
                  className={`rounded-full px-2 py-0.5 text-[10px] font-medium ${statusStyles[selectedExecution.status]}`}
                >
                  {selectedExecution.status}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="font-medium text-neutral-500 dark:text-neutral-400">Summary</span>
                <span>{selectedExecution.summary}</span>
              </div>
              {selectedExecution.error && (
                <div>
                  <span className="font-medium text-neutral-500 dark:text-neutral-400">Error</span>
                  <p className="mt-1 rounded border border-red-200 bg-red-50 p-2 text-red-700 dark:border-red-800 dark:bg-red-900/20 dark:text-red-400">
                    {selectedExecution.error}
                  </p>
                </div>
              )}
            </div>
            <div className="mt-4 flex justify-end">
              <button
                onClick={() => setSelectedExecution(null)}
                className="rounded border border-neutral-300 px-3 py-1.5 text-xs text-neutral-700 hover:bg-neutral-100 dark:border-neutral-600 dark:text-neutral-300 dark:hover:bg-neutral-700"
              >
                Close
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
