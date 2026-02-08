import type { Job, JobExecution } from "./JobList";

interface Props {
  jobs: Job[];
  onNewJob: () => void;
  onNewMigration: () => void;
  onNewComparison: () => void;
}

function formatTime(ts: number): string {
  return new Date(ts).toLocaleString();
}

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  const remainder = s % 60;
  return `${m}m ${remainder}s`;
}

const statusColors: Record<JobExecution["status"], string> = {
  Completed: "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400",
  Failed: "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400",
  Running: "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400",
  Cancelled: "bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400",
};

export default function Dashboard({
  jobs,
  onNewJob,
  onNewMigration,
  onNewComparison,
}: Props) {
  const totalJobs = jobs.length;
  const activeJobs = jobs.filter((j) => j.enabled).length;

  const allExecutions: (JobExecution & { jobName: string })[] = jobs
    .flatMap((j) => j.history.map((e) => ({ ...e, jobName: j.name })))
    .sort((a, b) => b.startTime - a.startTime);

  const twentyFourHoursAgo = Date.now() - 24 * 60 * 60 * 1000;
  const recentExecutions = allExecutions.filter((e) => e.startTime >= twentyFourHoursAgo);
  const last24hRuns = recentExecutions.length;
  const failedRuns = recentExecutions.filter((e) => e.status === "Failed").length;

  const recentFeed = allExecutions.slice(0, 10);

  return (
    <div className="flex h-full flex-col overflow-y-auto px-6 py-5">
      <h2 className="mb-5 text-base font-semibold text-neutral-800 dark:text-neutral-100">
        Dashboard
      </h2>

      {/* Summary cards */}
      <div className="mb-6 grid grid-cols-2 gap-3 sm:grid-cols-4">
        <SummaryCard label="Total Jobs" value={totalJobs} color="blue" />
        <SummaryCard label="Active Jobs" value={activeJobs} color="green" />
        <SummaryCard label="Last 24h Runs" value={last24hRuns} color="purple" />
        <SummaryCard label="Failed Runs" value={failedRuns} color="red" />
      </div>

      {/* Recent activity feed */}
      <div className="mb-6">
        <h3 className="mb-3 text-xs font-semibold text-neutral-600 dark:text-neutral-400">
          Recent Activity
        </h3>
        {recentFeed.length === 0 ? (
          <p className="text-xs text-neutral-400 dark:text-neutral-500">
            No recent executions.
          </p>
        ) : (
          <div className="space-y-1">
            {recentFeed.map((exec) => (
              <div
                key={exec.id}
                className="flex items-center justify-between rounded px-3 py-2 text-xs hover:bg-neutral-50 dark:hover:bg-neutral-800/50"
              >
                <div className="flex items-center gap-3">
                  <span
                    className={`inline-block rounded-full px-2 py-0.5 text-[10px] font-medium ${statusColors[exec.status]}`}
                  >
                    {exec.status}
                  </span>
                  <span className="font-medium text-neutral-700 dark:text-neutral-300">
                    {exec.jobName}
                  </span>
                  <span className="text-neutral-400 dark:text-neutral-500">
                    {exec.summary}
                  </span>
                </div>
                <div className="flex items-center gap-3 text-neutral-400 dark:text-neutral-500">
                  <span>{formatDuration(exec.durationMs)}</span>
                  <span>{formatTime(exec.startTime)}</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Quick actions */}
      <div>
        <h3 className="mb-3 text-xs font-semibold text-neutral-600 dark:text-neutral-400">
          Quick Actions
        </h3>
        <div className="flex flex-wrap gap-2">
          <button
            onClick={onNewJob}
            className="rounded bg-blue-600 px-4 py-2 text-xs font-medium text-white hover:bg-blue-700"
          >
            New Job
          </button>
          <button
            onClick={onNewMigration}
            className="rounded border border-neutral-300 px-4 py-2 text-xs font-medium text-neutral-700 hover:bg-neutral-100 dark:border-neutral-600 dark:text-neutral-300 dark:hover:bg-neutral-700"
          >
            New Migration
          </button>
          <button
            onClick={onNewComparison}
            className="rounded border border-neutral-300 px-4 py-2 text-xs font-medium text-neutral-700 hover:bg-neutral-100 dark:border-neutral-600 dark:text-neutral-300 dark:hover:bg-neutral-700"
          >
            New Comparison
          </button>
        </div>
      </div>
    </div>
  );
}

function SummaryCard({
  label,
  value,
  color,
}: {
  label: string;
  value: number;
  color: "blue" | "green" | "purple" | "red";
}) {
  const colorStyles = {
    blue: "border-blue-200 bg-blue-50 dark:border-blue-800 dark:bg-blue-900/20",
    green: "border-green-200 bg-green-50 dark:border-green-800 dark:bg-green-900/20",
    purple: "border-purple-200 bg-purple-50 dark:border-purple-800 dark:bg-purple-900/20",
    red: "border-red-200 bg-red-50 dark:border-red-800 dark:bg-red-900/20",
  };

  const valueStyles = {
    blue: "text-blue-700 dark:text-blue-400",
    green: "text-green-700 dark:text-green-400",
    purple: "text-purple-700 dark:text-purple-400",
    red: "text-red-700 dark:text-red-400",
  };

  return (
    <div className={`rounded-lg border p-4 ${colorStyles[color]}`}>
      <p className="text-[10px] font-medium text-neutral-500 dark:text-neutral-400">{label}</p>
      <p className={`mt-1 text-2xl font-bold ${valueStyles[color]}`}>{value}</p>
    </div>
  );
}
