interface SummaryItem {
  label: string;
  count: number;
  color: string;
  active: boolean;
  onClick: () => void;
}

interface Props {
  items: SummaryItem[];
  title?: string;
}

export default function DiffSummaryBar({ items, title }: Props) {
  const total = items.reduce((acc, item) => acc + item.count, 0);

  return (
    <div className="flex items-center gap-3 border-b border-neutral-200 bg-neutral-50 px-3 py-1.5 dark:border-neutral-700 dark:bg-neutral-850">
      {title && (
        <span className="text-xs font-semibold text-neutral-600 dark:text-neutral-400">
          {title}
        </span>
      )}
      <span className="text-xs text-neutral-500">
        Total: <span className="font-medium text-neutral-700 dark:text-neutral-300">{total}</span>
      </span>
      <div className="h-3 w-px bg-neutral-300 dark:bg-neutral-600" />
      {items.map((item) => (
        <button
          key={item.label}
          onClick={item.onClick}
          className={`flex items-center gap-1 rounded px-1.5 py-0.5 text-xs transition-colors ${
            item.active
              ? "bg-neutral-200 dark:bg-neutral-700"
              : "hover:bg-neutral-100 dark:hover:bg-neutral-800"
          }`}
        >
          <span
            className="inline-block h-2 w-2 rounded-full"
            style={{ backgroundColor: item.color }}
          />
          <span className="text-neutral-600 dark:text-neutral-400">{item.label}:</span>
          <span className="font-medium text-neutral-800 dark:text-neutral-200">{item.count}</span>
        </button>
      ))}
    </div>
  );
}
