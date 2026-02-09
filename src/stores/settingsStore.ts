import { create } from "zustand";
import { getAllSettings, setSetting as dbSetSetting } from "../lib/tauriCommands";

interface SettingsState {
  defaultBatchSize: number;
  defaultTimeout: number;
  defaultReadOnly: boolean;
  autoSaveJobs: boolean;
  confirmDestructiveOps: boolean;
  maxRecentConnections: number;
  hasCompletedOnboarding: boolean;
  _hydrated: boolean;

  setSetting: <K extends keyof SettingsState>(
    key: K,
    value: SettingsState[K],
  ) => void;
  setHasCompletedOnboarding: (value: boolean) => void;
  hydrate: () => Promise<void>;
}

const PERSIST_KEYS = [
  "defaultBatchSize",
  "defaultTimeout",
  "defaultReadOnly",
  "autoSaveJobs",
  "confirmDestructiveOps",
  "maxRecentConnections",
  "hasCompletedOnboarding",
] as const;

function parseSettingValue(key: string, raw: string): unknown {
  if (raw === "true") return true;
  if (raw === "false") return false;
  const num = Number(raw);
  if (!isNaN(num) && key !== "hasCompletedOnboarding") return num;
  return raw;
}

export const useSettingsStore = create<SettingsState>()((set, get) => ({
  defaultBatchSize: 1000,
  defaultTimeout: 30,
  defaultReadOnly: true,
  autoSaveJobs: true,
  confirmDestructiveOps: true,
  maxRecentConnections: 10,
  hasCompletedOnboarding: false,
  _hydrated: false,

  setSetting: (key, value) => {
    set({ [key]: value });
    // Persist to DB (fire-and-forget)
    if (PERSIST_KEYS.includes(key as (typeof PERSIST_KEYS)[number])) {
      dbSetSetting(key as string, String(value)).catch((e) =>
        console.error("Failed to persist setting:", e),
      );
    }
  },

  setHasCompletedOnboarding: (value) => {
    set({ hasCompletedOnboarding: value });
    dbSetSetting("hasCompletedOnboarding", String(value)).catch((e) =>
      console.error("Failed to persist onboarding:", e),
    );
  },

  hydrate: async () => {
    if (get()._hydrated) return;
    try {
      const all = await getAllSettings();
      const updates: Partial<SettingsState> = {};
      for (const key of PERSIST_KEYS) {
        if (key in all) {
          (updates as Record<string, unknown>)[key] = parseSettingValue(
            key,
            all[key],
          );
        }
      }
      set({ ...updates, _hydrated: true });
    } catch (e) {
      console.error("Failed to hydrate settings:", e);
      set({ _hydrated: true });
    }
  },
}));
