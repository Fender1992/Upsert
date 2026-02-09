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
  ]);
}

function App() {
  useThemeSync();
  useGlobalShortcuts();

  const { tabs, activeTabId } = useUiStore();
  const hasCompletedOnboarding = useSettingsStore((s) => s.hasCompletedOnboarding);
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
