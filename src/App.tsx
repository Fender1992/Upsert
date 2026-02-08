import { useEffect, useState, useCallback } from "react";
import { useUiStore } from "./stores/uiStore";
import ConnectionSidebar from "./components/connections/ConnectionSidebar";
import ConnectionDialog from "./components/connections/ConnectionDialog";
import TabBar from "./components/shared/TabBar";
import BottomPanel from "./components/shared/BottomPanel";
import CommandPalette from "./components/shared/CommandPalette";
import StatusBar from "./components/shared/StatusBar";

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
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editConnectionId, setEditConnectionId] = useState<string | null>(null);

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
          <div className="flex flex-1 items-center justify-center overflow-auto">
            {activeTab ? (
              <div className="text-sm text-neutral-500">
                {activeTab.type}: {activeTab.title}
              </div>
            ) : (
              <div className="text-center">
                <h1 className="text-xl font-bold text-neutral-400 dark:text-neutral-600">
                  Upsert
                </h1>
                <p className="mt-1 text-xs text-neutral-400 dark:text-neutral-600">
                  Open a connection or create a new tab to get started.
                </p>
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

      {/* Connection dialog */}
      {dialogOpen && (
        <ConnectionDialog editId={editConnectionId} onClose={handleCloseDialog} />
      )}
    </div>
  );
}

export default App;
