import { create } from "zustand";

export type Theme = "light" | "dark" | "system";

export interface Tab {
  id: string;
  title: string;
  type: "comparison" | "migration" | "job" | "query";
  icon?: string;
  isDirty?: boolean;
}

export interface Notification {
  id: string;
  type: "info" | "success" | "warning" | "error";
  title: string;
  message: string;
  timestamp: number;
  read: boolean;
}

interface UiState {
  theme: Theme;
  sidebarCollapsed: boolean;
  bottomPanelHeight: number;
  bottomPanelVisible: boolean;
  tabs: Tab[];
  activeTabId: string | null;
  commandPaletteOpen: boolean;
  notifications: Notification[];
  outputLog: string[];

  setTheme: (theme: Theme) => void;
  toggleSidebar: () => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  setBottomPanelHeight: (height: number) => void;
  setBottomPanelVisible: (visible: boolean) => void;
  addTab: (tab: Omit<Tab, "id">) => string;
  removeTab: (id: string) => void;
  setActiveTab: (id: string | null) => void;
  setCommandPaletteOpen: (open: boolean) => void;
  addNotification: (
    notification: Omit<Notification, "id" | "timestamp" | "read">,
  ) => void;
  markNotificationRead: (id: string) => void;
  appendLog: (message: string) => void;
  clearLog: () => void;
}

export const useUiStore = create<UiState>()((set) => ({
  theme: "system",
  sidebarCollapsed: false,
  bottomPanelHeight: 200,
  bottomPanelVisible: true,
  tabs: [],
  activeTabId: null,
  commandPaletteOpen: false,
  notifications: [],
  outputLog: [],

  setTheme: (theme) => set({ theme }),
  toggleSidebar: () =>
    set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),
  setSidebarCollapsed: (sidebarCollapsed) => set({ sidebarCollapsed }),
  setBottomPanelHeight: (bottomPanelHeight) => set({ bottomPanelHeight }),
  setBottomPanelVisible: (bottomPanelVisible) => set({ bottomPanelVisible }),

  addTab: (tab) => {
    const id = crypto.randomUUID();
    set((state) => ({
      tabs: [...state.tabs, { ...tab, id }],
      activeTabId: id,
    }));
    return id;
  },

  removeTab: (id) =>
    set((state) => {
      const tabs = state.tabs.filter((t) => t.id !== id);
      return {
        tabs,
        activeTabId:
          state.activeTabId === id
            ? (tabs[tabs.length - 1]?.id ?? null)
            : state.activeTabId,
      };
    }),

  setActiveTab: (activeTabId) => set({ activeTabId }),
  setCommandPaletteOpen: (commandPaletteOpen) => set({ commandPaletteOpen }),

  addNotification: (notification) =>
    set((state) => ({
      notifications: [
        {
          ...notification,
          id: crypto.randomUUID(),
          timestamp: Date.now(),
          read: false,
        },
        ...state.notifications,
      ],
    })),

  markNotificationRead: (id) =>
    set((state) => ({
      notifications: state.notifications.map((n) =>
        n.id === id ? { ...n, read: true } : n,
      ),
    })),

  appendLog: (message) =>
    set((state) => ({
      outputLog: [...state.outputLog, `[${new Date().toISOString()}] ${message}`],
    })),

  clearLog: () => set({ outputLog: [] }),
}));
