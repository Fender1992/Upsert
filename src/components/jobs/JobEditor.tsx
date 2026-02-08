import { useState, useEffect } from "react";
import type { Job, JobType, JobSchedule } from "./JobList";

const SCHEDULE_PRESETS: Array<{ value: JobSchedule["preset"]; label: string; cron: string }> = [
  { value: "hourly", label: "Every hour", cron: "0 * * * *" },
  { value: "daily", label: "Every day", cron: "0 0 * * *" },
  { value: "weekly", label: "Every week", cron: "0 0 * * 0" },
  { value: "custom", label: "Custom", cron: "" },
];

interface Props {
  job: Job | null;
  onSave: (data: { name: string; type: JobType; enabled: boolean; schedule: JobSchedule }) => void;
  onCancel: () => void;
}

export default function JobEditor({ job, onSave, onCancel }: Props) {
  const [name, setName] = useState("");
  const [type, setType] = useState<JobType>("Comparison");
  const [enabled, setEnabled] = useState(true);
  const [preset, setPreset] = useState<JobSchedule["preset"]>("daily");
  const [cron, setCron] = useState("0 0 * * *");
  const [timezone, setTimezone] = useState<"UTC" | "Local">("UTC");
  const [errors, setErrors] = useState<string[]>([]);

  useEffect(() => {
    if (job) {
      setName(job.name);
      setType(job.type);
      setEnabled(job.enabled);
      setPreset(job.schedule.preset);
      setCron(job.schedule.cron);
      setTimezone(job.schedule.timezone);
    }
  }, [job]);

  const handlePresetChange = (value: JobSchedule["preset"]) => {
    setPreset(value);
    const found = SCHEDULE_PRESETS.find((p) => p.value === value);
    if (found && value !== "custom") {
      setCron(found.cron);
    }
  };

  const validate = (): string[] => {
    const errs: string[] = [];
    if (!name.trim()) errs.push("Job name is required.");
    if (!cron.trim()) errs.push("Cron expression is required.");
    if (preset === "custom") {
      const parts = cron.trim().split(/\s+/);
      if (parts.length < 5) errs.push("Cron expression must have at least 5 fields.");
    }
    return errs;
  };

  const handleSave = () => {
    const errs = validate();
    if (errs.length > 0) {
      setErrors(errs);
      return;
    }
    onSave({
      name: name.trim(),
      type,
      enabled,
      schedule: { preset, cron: cron.trim(), timezone },
    });
  };

  return (
    <div
      className="fixed inset-0 z-40 flex items-center justify-center bg-black/40"
      onClick={onCancel}
    >
      <div
        className="w-full max-w-md rounded-lg border border-neutral-300 bg-white p-5 shadow-xl dark:border-neutral-600 dark:bg-neutral-800"
        onClick={(e) => e.stopPropagation()}
      >
        <h2 className="mb-4 text-sm font-semibold text-neutral-800 dark:text-neutral-100">
          {job ? "Edit Job" : "New Job"}
        </h2>

        {errors.length > 0 && (
          <div className="mb-3 rounded border border-red-300 bg-red-50 px-3 py-2 text-xs text-red-700 dark:border-red-700 dark:bg-red-900/30 dark:text-red-300">
            {errors.map((err, i) => (
              <div key={i}>{err}</div>
            ))}
          </div>
        )}

        <div className="space-y-3">
          {/* Job Name */}
          <div>
            <label className="mb-1 block text-xs font-medium text-neutral-600 dark:text-neutral-400">
              Job Name
            </label>
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="input-field"
              placeholder="My Scheduled Job"
            />
          </div>

          {/* Job Type */}
          <div>
            <label className="mb-1 block text-xs font-medium text-neutral-600 dark:text-neutral-400">
              Job Type
            </label>
            <select
              value={type}
              onChange={(e) => setType(e.target.value as JobType)}
              className="input-field"
            >
              <option value="Comparison">Comparison</option>
              <option value="Migration">Migration</option>
            </select>
          </div>

          {/* Enable / Disable Toggle */}
          <label className="flex items-center gap-2 text-xs text-neutral-700 dark:text-neutral-300">
            <input
              type="checkbox"
              checked={enabled}
              onChange={(e) => setEnabled(e.target.checked)}
              className="rounded border-neutral-300"
            />
            Enabled
          </label>

          {/* Schedule Preset */}
          <div>
            <label className="mb-1 block text-xs font-medium text-neutral-600 dark:text-neutral-400">
              Schedule
            </label>
            <div className="flex flex-wrap gap-2">
              {SCHEDULE_PRESETS.map((p) => (
                <button
                  key={p.value}
                  onClick={() => handlePresetChange(p.value)}
                  className={`rounded border px-3 py-1 text-xs ${
                    preset === p.value
                      ? "border-blue-500 bg-blue-50 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400"
                      : "border-neutral-300 text-neutral-700 hover:bg-neutral-100 dark:border-neutral-600 dark:text-neutral-300 dark:hover:bg-neutral-700"
                  }`}
                >
                  {p.label}
                </button>
              ))}
            </div>
          </div>

          {/* Custom Cron Input */}
          {preset === "custom" && (
            <div>
              <label className="mb-1 block text-xs font-medium text-neutral-600 dark:text-neutral-400">
                Cron Expression
              </label>
              <input
                value={cron}
                onChange={(e) => setCron(e.target.value)}
                className="input-field font-mono"
                placeholder="* * * * *"
              />
              <p className="mt-1 text-[10px] text-neutral-400 dark:text-neutral-500">
                Format: minute hour day-of-month month day-of-week (e.g., 0 9 * * 1-5 = weekdays
                at 9am)
              </p>
            </div>
          )}

          {/* Timezone */}
          <div>
            <label className="mb-1 block text-xs font-medium text-neutral-600 dark:text-neutral-400">
              Timezone
            </label>
            <select
              value={timezone}
              onChange={(e) => setTimezone(e.target.value as "UTC" | "Local")}
              className="input-field"
            >
              <option value="UTC">UTC</option>
              <option value="Local">Local</option>
            </select>
          </div>
        </div>

        {/* Footer */}
        <div className="mt-5 flex justify-end gap-2">
          <button
            onClick={onCancel}
            className="rounded border border-neutral-300 px-3 py-1.5 text-xs text-neutral-700 hover:bg-neutral-100 dark:border-neutral-600 dark:text-neutral-300 dark:hover:bg-neutral-700"
          >
            Cancel
          </button>
          <button
            onClick={handleSave}
            className="rounded bg-blue-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-blue-700"
          >
            {job ? "Save Changes" : "Create Job"}
          </button>
        </div>
      </div>
    </div>
  );
}
