import { useEffect, useRef, useCallback } from "react";
import { useUiStore } from "../../stores/uiStore";

export default function BottomPanel() {
  const {
    bottomPanelVisible,
    setBottomPanelVisible,
    bottomPanelHeight,
    setBottomPanelHeight,
    outputLog,
    clearLog,
  } = useUiStore();

  const logEndRef = useRef<HTMLDivElement>(null);
  const dragRef = useRef<{ startY: number; startHeight: number } | null>(null);

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [outputLog.length]);

  const onMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      dragRef.current = { startY: e.clientY, startHeight: bottomPanelHeight };

      const onMouseMove = (ev: MouseEvent) => {
        if (!dragRef.current) return;
        const delta = dragRef.current.startY - ev.clientY;
        const newH = Math.max(80, Math.min(500, dragRef.current.startHeight + delta));
        setBottomPanelHeight(newH);
      };

      const onMouseUp = () => {
        dragRef.current = null;
        document.removeEventListener("mousemove", onMouseMove);
        document.removeEventListener("mouseup", onMouseUp);
      };

      document.addEventListener("mousemove", onMouseMove);
      document.addEventListener("mouseup", onMouseUp);
    },
    [bottomPanelHeight, setBottomPanelHeight],
  );

  if (!bottomPanelVisible) return null;

  return (
    <div
      data-tour="bottom-panel"
      className="flex flex-col border-t border-neutral-300 bg-white dark:border-neutral-700 dark:bg-neutral-900"
      style={{ height: bottomPanelHeight }}
    >
      {/* Drag handle */}
      <div
        className="flex h-1.5 cursor-ns-resize items-center justify-center bg-neutral-200 hover:bg-neutral-300 dark:bg-neutral-800 dark:hover:bg-neutral-700"
        onMouseDown={onMouseDown}
      >
        <div className="h-0.5 w-8 rounded-full bg-neutral-400 dark:bg-neutral-600" />
      </div>
      {/* Header */}
      <div className="flex h-7 items-center justify-between border-b border-neutral-200 px-3 dark:border-neutral-700">
        <span className="text-xs font-medium text-neutral-600 dark:text-neutral-400">
          Output
        </span>
        <div className="flex items-center gap-1">
          <button
            onClick={clearLog}
            className="rounded px-1.5 py-0.5 text-[10px] text-neutral-500 hover:bg-neutral-200 dark:hover:bg-neutral-700"
            title="Clear log"
          >
            Clear
          </button>
          <button
            onClick={() => setBottomPanelVisible(false)}
            className="rounded px-1.5 py-0.5 text-[10px] text-neutral-500 hover:bg-neutral-200 dark:hover:bg-neutral-700"
            title="Hide panel"
          >
            Hide
          </button>
        </div>
      </div>
      {/* Log content */}
      <div className="flex-1 overflow-y-auto p-2 font-mono text-xs text-neutral-700 dark:text-neutral-300">
        {outputLog.length === 0 ? (
          <span className="text-neutral-400">No output yet.</span>
        ) : (
          outputLog.map((line, i) => (
            <div key={i} className="whitespace-pre-wrap py-px">
              {line}
            </div>
          ))
        )}
        <div ref={logEndRef} />
      </div>
    </div>
  );
}
