import { useState, useEffect, useRef, useCallback } from "react";
import { useUiStore } from "../../stores/uiStore";
import { useChatStore } from "../../stores/chatStore";

interface Command {
  id: string;
  label: string;
  action: () => void;
}

export default function CommandPalette() {
  const {
    commandPaletteOpen,
    setCommandPaletteOpen,
    toggleSidebar,
    setTheme,
    theme,
    clearLog,
    bottomPanelVisible,
    setBottomPanelVisible,
  } = useUiStore();

  const toggleChatDrawer = useChatStore((s) => s.toggleDrawer);

  const [search, setSearch] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const commands: Command[] = [
    {
      id: "toggle-sidebar",
      label: "Toggle Sidebar",
      action: toggleSidebar,
    },
    {
      id: "toggle-theme",
      label: "Toggle Theme",
      action: () => {
        if (theme === "light") setTheme("dark");
        else if (theme === "dark") setTheme("system");
        else setTheme("light");
      },
    },
    {
      id: "toggle-bottom-panel",
      label: "Toggle Bottom Panel",
      action: () => setBottomPanelVisible(!bottomPanelVisible),
    },
    {
      id: "clear-log",
      label: "Clear Output Log",
      action: clearLog,
    },
    {
      id: "toggle-ai-chat",
      label: "Toggle AI Chat",
      action: toggleChatDrawer,
    },
  ];

  const filtered = commands.filter((c) =>
    c.label.toLowerCase().includes(search.toLowerCase()),
  );

  const runCommand = useCallback(
    (cmd: Command) => {
      cmd.action();
      setCommandPaletteOpen(false);
      setSearch("");
    },
    [setCommandPaletteOpen],
  );

  useEffect(() => {
    if (commandPaletteOpen) {
      setSearch("");
      setSelectedIndex(0);
      requestAnimationFrame(() => inputRef.current?.focus());
    }
  }, [commandPaletteOpen]);

  useEffect(() => {
    setSelectedIndex(0);
  }, [search]);

  const onKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIndex((i) => Math.min(i + 1, filtered.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIndex((i) => Math.max(i - 1, 0));
      } else if (e.key === "Enter") {
        e.preventDefault();
        const cmd = filtered[selectedIndex];
        if (cmd) runCommand(cmd);
      } else if (e.key === "Escape") {
        setCommandPaletteOpen(false);
        setSearch("");
      }
    },
    [filtered, selectedIndex, runCommand, setCommandPaletteOpen],
  );

  if (!commandPaletteOpen) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center bg-black/40 pt-[20vh]"
      onClick={() => setCommandPaletteOpen(false)}
    >
      <div
        className="w-full max-w-md rounded-lg border border-neutral-300 bg-white shadow-xl dark:border-neutral-600 dark:bg-neutral-800"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={onKeyDown}
      >
        <input
          ref={inputRef}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Type a command..."
          className="w-full rounded-t-lg border-b border-neutral-200 bg-transparent px-4 py-3 text-sm outline-none dark:border-neutral-700 dark:text-neutral-100"
        />
        <div className="max-h-64 overflow-y-auto py-1">
          {filtered.length === 0 ? (
            <div className="px-4 py-2 text-sm text-neutral-500">
              No matching commands.
            </div>
          ) : (
            filtered.map((cmd, i) => (
              <div
                key={cmd.id}
                className={`cursor-pointer px-4 py-2 text-sm ${
                  i === selectedIndex
                    ? "bg-blue-600 text-white"
                    : "text-neutral-800 hover:bg-neutral-100 dark:text-neutral-200 dark:hover:bg-neutral-700"
                }`}
                onClick={() => runCommand(cmd)}
                onMouseEnter={() => setSelectedIndex(i)}
              >
                {cmd.label}
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
