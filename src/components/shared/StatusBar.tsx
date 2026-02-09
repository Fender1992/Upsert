import { useUiStore } from "../../stores/uiStore";
import { useConnectionStore } from "../../stores/connectionStore";
import { useTourStore } from "../../stores/tourStore";
import { useChatStore } from "../../stores/chatStore";

export default function StatusBar() {
  const { theme, setTheme, notifications } = useUiStore();
  const startTour = useTourStore((s) => s.startTour);
  const toggleChatDrawer = useChatStore((s) => s.toggleDrawer);
  const { connections, activeConnectionId } = useConnectionStore();

  const activeConnection = connections.find((c) => c.id === activeConnectionId);
  const unreadCount = notifications.filter((n) => !n.read).length;

  const resolvedTheme =
    theme === "system"
      ? window.matchMedia("(prefers-color-scheme: dark)").matches
        ? "dark"
        : "light"
      : theme;

  const cycleTheme = () => {
    if (theme === "light") setTheme("dark");
    else if (theme === "dark") setTheme("system");
    else setTheme("light");
  };

  return (
    <div data-tour="status-bar" className="flex h-6 items-center justify-between border-t border-neutral-300 bg-neutral-100 px-3 text-xs dark:border-neutral-700 dark:bg-neutral-900 dark:text-neutral-400">
      <div className="flex items-center gap-2">
        {activeConnection ? (
          <>
            <span
              className={`inline-block h-2 w-2 rounded-full ${
                activeConnection.status === "connected"
                  ? "bg-green-500"
                  : activeConnection.status === "error"
                    ? "bg-red-500"
                    : "bg-neutral-400"
              }`}
            />
            <span>{activeConnection.name}</span>
            <span className="text-neutral-500">({activeConnection.engine})</span>
          </>
        ) : (
          <span className="text-neutral-500">No connection</span>
        )}
      </div>
      <div className="flex items-center gap-3">
        <button
          onClick={toggleChatDrawer}
          className="rounded px-1.5 py-0.5 text-[10px] font-medium text-neutral-500 hover:bg-neutral-200 hover:text-neutral-700 dark:hover:bg-neutral-700 dark:hover:text-neutral-300"
          title="AI Chat (Ctrl+L)"
        >
          AI Chat
        </button>
        <button
          onClick={startTour}
          className="rounded px-1.5 py-0.5 text-[10px] font-medium text-neutral-500 hover:bg-neutral-200 hover:text-neutral-700 dark:hover:bg-neutral-700 dark:hover:text-neutral-300"
          title="Start guided tour"
        >
          Tour
        </button>
        {unreadCount > 0 && (
          <span className="rounded-full bg-blue-600 px-1.5 text-[10px] font-medium text-white">
            {unreadCount}
          </span>
        )}
        <button
          onClick={cycleTheme}
          className="hover:text-neutral-900 dark:hover:text-neutral-100"
          title={`Theme: ${theme}`}
        >
          {resolvedTheme === "dark" ? (
            <svg
              className="h-3.5 w-3.5"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={2}
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                d="M12 3v1m0 16v1m8.66-13.66l-.71.71M4.05 19.95l-.71.71M21 12h-1M4 12H3m16.66 7.66l-.71-.71M4.05 4.05l-.71-.71M16 12a4 4 0 11-8 0 4 4 0 018 0z"
              />
            </svg>
          ) : (
            <svg
              className="h-3.5 w-3.5"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={2}
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                d="M21 12.79A9 9 0 1111.21 3 7 7 0 0021 12.79z"
              />
            </svg>
          )}
        </button>
      </div>
    </div>
  );
}
