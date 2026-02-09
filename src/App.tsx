import { useEffect, useState, useCallback } from "react";
import { useUiStore } from "./stores/uiStore";
import { useSettingsStore } from "./stores/settingsStore";
import ConnectionSidebar from "./components/connections/ConnectionSidebar";
import ConnectionDialog from "./components/connections/ConnectionDialog";
import MigrationWizard from "./components/migration/MigrationWizard";
import TabBar from "./components/shared/TabBar";
import BottomPanel from "./components/shared/BottomPanel";
import CommandPalette from "./components/shared/CommandPalette";
import StatusBar from "./components/shared/StatusBar";
import OnboardingWizard from "./components/onboarding/OnboardingWizard";
import JobList from "./components/jobs/JobList";
import Dashboard from "./components/jobs/Dashboard";
import AppTour from "./components/tour/AppTour";
import ChatDrawer from "./components/chat/ChatDrawer";
import { useTourStore } from "./stores/tourStore";
import { useChatStore } from "./stores/chatStore";
import { listen } from "@tauri-apps/api/event";
import { useConnectionStore } from "./stores/connectionStore";
import { indexAppContext, indexConnectionContext } from "./lib/tauriCommands";

function useDbHydration() {
  const hydrateSettings = useSettingsStore((s) => s.hydrate);
  const hydrateConnections = useConnectionStore((s) => s.hydrate);
  const hydrateChat = useChatStore((s) => s.hydrate);

  useEffect(() => {
    hydrateSettings();
    hydrateConnections();
    hydrateChat();
  }, [hydrateSettings, hydrateConnections, hydrateChat]);

  // On models-ready, index app context and re-index connected DBs
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<boolean>("models-ready", () => {
      // Index global app info
      indexAppContext().catch(() => {});
      // Re-index any already-connected databases
      const { connections } = useConnectionStore.getState();
      for (const conn of connections) {
        if (conn.status === "connected") {
          indexConnectionContext(conn.id, conn.name, conn.engine).catch(() => {});
        }
      }
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, []);
}

function useThemeSync() {
  const theme = useUiStore((s) => s.theme);

  useEffect(() => {
    const apply = (resolved: "dark" | "light") => {
      document.documentElement.classList.toggle("dark", resolved === "dark");
    };

    if (theme === "system") {
      const mq = window.matchMedia("(prefers-color-scheme: dark)");
      apply(mq.matches ? "dark" : "light");
      const handler = (e: MediaQueryListEvent) =>
        apply(e.matches ? "dark" : "light");
      mq.addEventListener("change", handler);
      return () => mq.removeEventListener("change", handler);
    } else {
      apply(theme);
    }
  }, [theme]);
}

function useGlobalShortcuts() {
  const {
    setCommandPaletteOpen,
    commandPaletteOpen,
    toggleSidebar,
    bottomPanelVisible,
    setBottomPanelVisible,
    activeTabId,
    removeTab,
  } = useUiStore();

  const toggleChatDrawer = useChatStore((s) => s.toggleDrawer);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const ctrl = e.ctrlKey || e.metaKey;

      if (ctrl && e.key === "k") {
        e.preventDefault();
        setCommandPaletteOpen(!commandPaletteOpen);
      } else if (ctrl && e.key === "b") {
        e.preventDefault();
        toggleSidebar();
      } else if (ctrl && e.key === "`") {
        e.preventDefault();
        setBottomPanelVisible(!bottomPanelVisible);
      } else if (ctrl && e.key === "w") {
        e.preventDefault();
        if (activeTabId) removeTab(activeTabId);
      } else if (ctrl && e.key === "l") {
        e.preventDefault();
        toggleChatDrawer();
      }
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [
    commandPaletteOpen,
    setCommandPaletteOpen,
    toggleSidebar,
    bottomPanelVisible,
    setBottomPanelVisible,
    activeTabId,
    removeTab,
    toggleChatDrawer,
  ]);
}

function App() {
  useDbHydration();
  useThemeSync();
  useGlobalShortcuts();

  const { tabs, activeTabId } = useUiStore();
  const hasCompletedOnboarding = useSettingsStore((s) => s.hasCompletedOnboarding);
  const startTour = useTourStore((s) => s.startTour);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editConnectionId, setEditConnectionId] = useState<string | null>(null);
  const [migrationWizardOpen, setMigrationWizardOpen] = useState(false);
  const [dashboardOpen, setDashboardOpen] = useState(false);
  const [jobListOpen, setJobListOpen] = useState(false);

  const handleNewConnection = useCallback(() => {
    setEditConnectionId(null);
    setDialogOpen(true);
  }, []);

  const handleEditConnection = useCallback((id: string) => {
    setEditConnectionId(id);
    setDialogOpen(true);
  }, []);

  const handleCloseDialog = useCallback(() => {
    setDialogOpen(false);
    setEditConnectionId(null);
  }, []);

  const activeTab = tabs.find((t) => t.id === activeTabId);

  if (!hasCompletedOnboarding) {
    return <OnboardingWizard />;
  }

  return (
    <div className="flex h-screen w-screen flex-col bg-white text-neutral-900 dark:bg-neutral-900 dark:text-neutral-100">
      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar */}
        <ConnectionSidebar
          onNewConnection={handleNewConnection}
          onEditConnection={handleEditConnection}
        />

        {/* Main area */}
        <div className="flex flex-1 flex-col overflow-hidden">
          {/* Tab bar */}
          <TabBar />

          {/* Tab content */}
          <div data-tour="main-content" className="flex flex-1 items-center justify-center overflow-auto">
            {migrationWizardOpen ? (
              <div className="h-full w-full">
                <MigrationWizard onClose={() => setMigrationWizardOpen(false)} />
              </div>
            ) : jobListOpen ? (
              <div className="h-full w-full">
                <JobList onClose={() => setJobListOpen(false)} />
              </div>
            ) : dashboardOpen ? (
              <div className="h-full w-full">
                <Dashboard
                  jobs={[]}
                  onNewJob={() => {
                    setDashboardOpen(false);
                    setJobListOpen(true);
                  }}
                  onNewMigration={() => {
                    setDashboardOpen(false);
                    setMigrationWizardOpen(true);
                  }}
                  onNewComparison={() => {
                    setDashboardOpen(false);
                  }}
                />
              </div>
            ) : activeTab ? (
              <div className="text-sm text-neutral-500">
                {activeTab.type}: {activeTab.title}
              </div>
            ) : (
              <div className="text-center">
                <button
                  onClick={() => setDashboardOpen(true)}
                  className="text-xl font-bold text-neutral-400 hover:text-blue-500 dark:text-neutral-600 dark:hover:text-blue-400"
                  title="Open Dashboard"
                >
                  Upsert
                </button>
                <p className="mt-1 text-xs text-neutral-400 dark:text-neutral-600">
                  Click the title to open the Dashboard, or get started below.
                </p>
                <div className="mt-3 flex justify-center">
                  <button
                    onClick={startTour}
                    className="rounded-full border border-blue-200 bg-blue-50 px-4 py-1.5 text-xs font-medium text-blue-600 hover:bg-blue-100 dark:border-blue-800 dark:bg-blue-950/40 dark:text-blue-400 dark:hover:bg-blue-900/50"
                  >
                    Take a Tour
                  </button>
                </div>
                <div className="mt-4 flex justify-center gap-2">
                  <button
                    data-tour="new-migration-btn"
                    onClick={() => setMigrationWizardOpen(true)}
                    className="rounded bg-blue-600 px-4 py-2 text-xs font-medium text-white hover:bg-blue-700"
                  >
                    New Migration
                  </button>
                  <button
                    data-tour="jobs-btn"
                    onClick={() => setJobListOpen(true)}
                    className="rounded border border-neutral-300 px-4 py-2 text-xs font-medium text-neutral-700 hover:bg-neutral-100 dark:border-neutral-600 dark:text-neutral-300 dark:hover:bg-neutral-700"
                  >
                    Jobs
                  </button>
                  <button
                    data-tour="dashboard-btn"
                    onClick={() => setDashboardOpen(true)}
                    className="rounded border border-neutral-300 px-4 py-2 text-xs font-medium text-neutral-700 hover:bg-neutral-100 dark:border-neutral-600 dark:text-neutral-300 dark:hover:bg-neutral-700"
                  >
                    Dashboard
                  </button>
                </div>
              </div>
            )}
          </div>

          {/* Bottom panel */}
          <BottomPanel />
        </div>
      </div>

      {/* Status bar */}
      <StatusBar />

      {/* Command palette overlay */}
      <CommandPalette />

      {/* AI Chat drawer */}
      <ChatDrawer />

      {/* Guided tour overlay */}
      <AppTour />

      {/* Connection dialog */}
      {dialogOpen && (
        <ConnectionDialog editId={editConnectionId} onClose={handleCloseDialog} />
      )}
    </div>
  );
}

export default App;
