import { useUiStore } from "../../stores/uiStore";

const typeIcons: Record<string, string> = {
  comparison: "Diff",
  migration: "Mig",
  job: "Job",
  query: "SQL",
};

export default function TabBar() {
  const { tabs, activeTabId, setActiveTab, removeTab } = useUiStore();

  if (tabs.length === 0) {
    return null;
  }

  return (
    <div data-tour="tab-bar" className="flex h-9 shrink-0 items-stretch overflow-x-auto border-b border-neutral-300 bg-neutral-100 dark:border-neutral-700 dark:bg-neutral-900">
      {tabs.map((tab) => (
        <div
          key={tab.id}
          className={`group flex cursor-pointer items-center gap-1.5 border-r border-neutral-300 px-3 text-xs dark:border-neutral-700 ${
            tab.id === activeTabId
              ? "bg-white text-neutral-900 dark:bg-neutral-800 dark:text-neutral-100"
              : "text-neutral-600 hover:bg-neutral-200 dark:text-neutral-400 dark:hover:bg-neutral-800"
          }`}
          onClick={() => setActiveTab(tab.id)}
        >
          <span className="font-mono text-[10px] text-neutral-400">
            {typeIcons[tab.type] ?? "Tab"}
          </span>
          <span className="max-w-[120px] truncate whitespace-nowrap">
            {tab.title}
            {tab.isDirty && <span className="ml-0.5 text-orange-400">*</span>}
          </span>
          <button
            className="ml-1 rounded p-0.5 opacity-0 hover:bg-neutral-300 group-hover:opacity-100 dark:hover:bg-neutral-600"
            onClick={(e) => {
              e.stopPropagation();
              removeTab(tab.id);
            }}
            title="Close tab"
          >
            <svg className="h-3 w-3" viewBox="0 0 12 12" fill="currentColor">
              <path d="M3.05 3.05a.5.5 0 01.7 0L6 5.29l2.25-2.24a.5.5 0 01.7.7L6.71 6l2.24 2.25a.5.5 0 01-.7.7L6 6.71 3.75 8.95a.5.5 0 01-.7-.7L5.29 6 3.05 3.75a.5.5 0 010-.7z" />
            </svg>
          </button>
        </div>
      ))}
    </div>
  );
}
