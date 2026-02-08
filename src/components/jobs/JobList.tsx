import { useState, useCallback } from "react";
import JobEditor from "./JobEditor";
import JobHistory from "./JobHistory";

export type JobType = "Comparison" | "Migration";
export type JobStatus = "active" | "disabled" | "failed";

export interface JobSchedule {
  preset: "hourly" | "daily" | "weekly" | "custom";
  cron: string;
  timezone: "UTC" | "Local";
}

export interface JobExecution {
  id: string;
  jobId: string;
  startTime: number;
  durationMs: number;
  status: "Completed" | "Failed" | "Running" | "Cancelled";
  summary: string;
  error?: string;
}

export interface Job {
  id: string;
  name: string;
  type: JobType;
  enabled: boolean;
  schedule: JobSchedule;
  status: JobStatus;
  lastRun?: number;
  nextRun?: number;
  history: JobExecution[];
}

function formatDate(ts?: number): string {
  if (!ts) return "--";
  return new Date(ts).toLocaleString();
}

function cronToLabel(schedule: JobSchedule): string {
  switch (schedule.preset) {
    case "hourly":
      return "Every hour";
    case "daily":
      return "Every day";
    case "weekly":
      return "Every week";
    case "custom":
      return schedule.cron;
    default:
      return schedule.cron;
  }
}

export default function JobList({ onClose }: { onClose: () => void }) {
  const [jobs, setJobs] = useState<Job[]>([]);
  const [editorOpen, setEditorOpen] = useState(false);
  const [editingJob, setEditingJob] = useState<Job | null>(null);
  const [historyJobId, setHistoryJobId] = useState<string | null>(null);

  const handleNewJob = useCallback(() => {
    setEditingJob(null);
    setEditorOpen(true);
  }, []);

  const handleEditJob = useCallback((job: Job) => {
    setEditingJob(job);
    setEditorOpen(true);
  }, []);

  const handleSaveJob = useCallback(
    (data: { name: string; type: JobType; enabled: boolean; schedule: JobSchedule }) => {
      if (editingJob) {
        setJobs((prev) =>
          prev.map((j) =>
            j.id === editingJob.id
              ? { ...j, ...data, status: data.enabled ? "active" : "disabled" }
              : j,
          ),
        );
      } else {
        const newJob: Job = {
          id: crypto.randomUUID(),
          ...data,
          status: data.enabled ? "active" : "disabled",
          history: [],
        };
        setJobs((prev) => [...prev, newJob]);
      }
      setEditorOpen(false);
      setEditingJob(null);
    },
    [editingJob],
  );

  const handleDeleteJob = useCallback((id: string) => {
    setJobs((prev) => prev.filter((j) => j.id !== id));
  }, []);

  const handleToggleEnabled = useCallback((id: string) => {
    setJobs((prev) =>
      prev.map((j) => {
        if (j.id !== id) return j;
        const enabled = !j.enabled;
        return { ...j, enabled, status: enabled ? "active" : "disabled" };
      }),
    );
  }, []);

  const handleRunNow = useCallback((id: string) => {
    const execution: JobExecution = {
      id: crypto.randomUUID(),
      jobId: id,
      startTime: Date.now(),
      durationMs: Math.floor(Math.random() * 5000) + 500,
      status: "Completed",
      summary: "Executed successfully",
    };
    setJobs((prev) =>
      prev.map((j) =>
        j.id === id
          ? { ...j, lastRun: execution.startTime, history: [execution, ...j.history] }
          : j,
      ),
    );
  }, []);

  const historyJob = historyJobId ? jobs.find((j) => j.id === historyJobId) : null;

  if (historyJob) {
    return (
      <JobHistory
        job={historyJob}
        onBack={() => setHistoryJobId(null)}
        onClearHistory={() => {
          setJobs((prev) =>
            prev.map((j) => (j.id === historyJob.id ? { ...j, history: [] } : j)),
          );
        }}
      />
    );
  }

  return (
    <div className="flex h-full flex-col overflow-hidden">
      {/* Header */}
      <div className="flex shrink-0 items-center justify-between border-b border-neutral-200 bg-neutral-50 px-6 py-3 dark:border-neutral-700 dark:bg-neutral-850">
        <div className="flex items-center gap-3">
          <button
            onClick={onClose}
            className="rounded p-1 text-neutral-500 hover:bg-neutral-200 dark:hover:bg-neutral-700"
            title="Back"
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
            Jobs
          </h2>
        </div>
        <button
          onClick={handleNewJob}
          className="rounded bg-blue-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-blue-700"
        >
          + New Job
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto px-6 py-4">
        {jobs.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-16 text-center">
            <div className="mb-3 rounded-full bg-neutral-100 p-4 dark:bg-neutral-800">
              <svg
                className="h-8 w-8 text-neutral-400"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                strokeWidth={1.5}
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M20.25 14.15v4.25c0 1.094-.787 2.036-1.872 2.18-2.087.277-4.216.42-6.378.42s-4.291-.143-6.378-.42c-1.085-.144-1.872-1.086-1.872-2.18v-4.25m16.5 0a2.18 2.18 0 00.75-1.661V8.706c0-1.081-.768-2.015-1.837-2.175a48.114 48.114 0 00-3.413-.387m4.5 8.006c-.194.165-.42.295-.673.38A23.978 23.978 0 0112 15.75c-2.648 0-5.195-.429-7.577-1.22a2.016 2.016 0 01-.673-.38m0 0A2.18 2.18 0 013 12.489V8.706c0-1.081.768-2.015 1.837-2.175a48.111 48.111 0 013.413-.387m7.5 0V5.25A2.25 2.25 0 0013.5 3h-3a2.25 2.25 0 00-2.25 2.25v.894m7.5 0a48.667 48.667 0 00-7.5 0"
                />
              </svg>
            </div>
            <p className="text-sm font-medium text-neutral-600 dark:text-neutral-400">
              No jobs yet
            </p>
            <p className="mt-1 text-xs text-neutral-400 dark:text-neutral-500">
              Create a scheduled job to automate comparisons and migrations.
            </p>
            <button
              onClick={handleNewJob}
              className="mt-4 rounded bg-blue-600 px-4 py-2 text-xs font-medium text-white hover:bg-blue-700"
            >
              Create Your First Job
            </button>
          </div>
        ) : (
          <table className="w-full text-xs">
            <thead>
              <tr className="border-b border-neutral-200 text-left text-neutral-500 dark:border-neutral-700 dark:text-neutral-400">
                <th className="pb-2 pr-4 font-medium">Name</th>
                <th className="pb-2 pr-4 font-medium">Type</th>
                <th className="pb-2 pr-4 font-medium">Schedule</th>
                <th className="pb-2 pr-4 font-medium">Last Run</th>
                <th className="pb-2 pr-4 font-medium">Status</th>
                <th className="pb-2 font-medium">Actions</th>
              </tr>
            </thead>
            <tbody>
              {jobs.map((job) => (
                <tr
                  key={job.id}
                  className="border-b border-neutral-100 text-neutral-700 dark:border-neutral-800 dark:text-neutral-300"
                >
                  <td className="py-2.5 pr-4 font-medium">{job.name}</td>
                  <td className="py-2.5 pr-4">{job.type}</td>
                  <td className="py-2.5 pr-4">{cronToLabel(job.schedule)}</td>
                  <td className="py-2.5 pr-4">{formatDate(job.lastRun)}</td>
                  <td className="py-2.5 pr-4">
                    <span
                      className={`inline-flex items-center gap-1.5 rounded-full px-2 py-0.5 text-[10px] font-medium ${
                        job.status === "active"
                          ? "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400"
                          : job.status === "failed"
                            ? "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400"
                            : "bg-neutral-100 text-neutral-500 dark:bg-neutral-800 dark:text-neutral-400"
                      }`}
                    >
                      <span
                        className={`h-1.5 w-1.5 rounded-full ${
                          job.status === "active"
                            ? "bg-green-500"
                            : job.status === "failed"
                              ? "bg-red-500"
                              : "bg-neutral-400"
                        }`}
                      />
                      {job.status === "active"
                        ? "Active"
                        : job.status === "failed"
                          ? "Failed"
                          : "Disabled"}
                    </span>
                  </td>
                  <td className="py-2.5">
                    <div className="flex items-center gap-1">
                      <button
                        onClick={() => handleRunNow(job.id)}
                        className="rounded px-2 py-1 text-blue-600 hover:bg-blue-50 dark:text-blue-400 dark:hover:bg-blue-900/20"
                        title="Run Now"
                      >
                        Run
                      </button>
                      <button
                        onClick={() => handleEditJob(job)}
                        className="rounded px-2 py-1 text-neutral-600 hover:bg-neutral-100 dark:text-neutral-400 dark:hover:bg-neutral-800"
                        title="Edit"
                      >
                        Edit
                      </button>
                      <button
                        onClick={() => setHistoryJobId(job.id)}
                        className="rounded px-2 py-1 text-neutral-600 hover:bg-neutral-100 dark:text-neutral-400 dark:hover:bg-neutral-800"
                        title="History"
                      >
                        History
                      </button>
                      <button
                        onClick={() => handleToggleEnabled(job.id)}
                        className={`rounded px-2 py-1 ${
                          job.enabled
                            ? "text-amber-600 hover:bg-amber-50 dark:text-amber-400 dark:hover:bg-amber-900/20"
                            : "text-green-600 hover:bg-green-50 dark:text-green-400 dark:hover:bg-green-900/20"
                        }`}
                        title={job.enabled ? "Disable" : "Enable"}
                      >
                        {job.enabled ? "Disable" : "Enable"}
                      </button>
                      <button
                        onClick={() => handleDeleteJob(job.id)}
                        className="rounded px-2 py-1 text-red-600 hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-900/20"
                        title="Delete"
                      >
                        Delete
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {/* Job Editor dialog */}
      {editorOpen && (
        <JobEditor
          job={editingJob}
          onSave={handleSaveJob}
          onCancel={() => {
            setEditorOpen(false);
            setEditingJob(null);
          }}
        />
      )}
    </div>
  );
}
