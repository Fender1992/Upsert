import { create } from "zustand";

interface SettingsState {
  defaultBatchSize: number;
  defaultTimeout: number;
  defaultReadOnly: boolean;
  autoSaveJobs: boolean;
  confirmDestructiveOps: boolean;
  maxRecentConnections: number;
  hasCompletedOnboarding: boolean;

  setSetting: <K extends keyof SettingsState>(
    key: K,
    value: SettingsState[K],
  ) => void;
  setHasCompletedOnboarding: (value: boolean) => void;
}

export const useSettingsStore = create<SettingsState>()((set) => ({
  defaultBatchSize: 1000,
  defaultTimeout: 30,
  defaultReadOnly: true,
  autoSaveJobs: true,
  confirmDestructiveOps: true,
  maxRecentConnections: 10,
  hasCompletedOnboarding: false,

  setSetting: (key, value) => set({ [key]: value }),
  setHasCompletedOnboarding: (value) => set({ hasCompletedOnboarding: value }),
}));
