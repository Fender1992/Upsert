import { create } from "zustand";

export interface TourStep {
  id: string;
  title: string;
  description: string;
  /** CSS selector for the target element. If null, shows a centered modal. */
  target: string | null;
  /** Preferred tooltip position relative to the target */
  position: "top" | "bottom" | "left" | "right" | "center";
  /** Optional action to perform when this step becomes active */
  onEnter?: () => void;
}

interface TourState {
  isActive: boolean;
  currentStep: number;
  steps: TourStep[];

  startTour: () => void;
  endTour: () => void;
  nextStep: () => void;
  prevStep: () => void;
  goToStep: (index: number) => void;
}

const TOUR_STEPS: TourStep[] = [
  {
    id: "welcome",
    title: "Welcome to Upsert",
    description:
      "Upsert is a cross-platform database comparison and migration tool. It supports 7 database engines including SQL Server, PostgreSQL, MySQL, SQLite, MongoDB, Oracle, and CosmosDB. Let's take a quick tour of the application.",
    target: null,
    position: "center",
  },
  {
    id: "sidebar",
    title: "Connection Sidebar",
    description:
      "This is where you manage all your database connections. Each connection shows its engine type (SQL, PG, My, etc.), name, and status. Green means connected, gray means disconnected.",
    target: "[data-tour='sidebar']",
    position: "right",
  },
  {
    id: "new-connection",
    title: "Add a Connection",
    description:
      'Click "+ New Connection" to add a database. You\'ll choose the engine type, enter host/port/credentials, and give it a friendly name. You can test the connection before saving.',
    target: "[data-tour='new-connection-btn']",
    position: "right",
  },
  {
    id: "connection-list",
    title: "Your Connections",
    description:
      "Connected databases appear here. Right-click any connection for options like Connect, Disconnect, Edit, or Delete. Click a connection to select it as active.",
    target: "[data-tour='connection-list']",
    position: "right",
  },
  {
    id: "main-content",
    title: "Main Workspace",
    description:
      "This is your workspace. From here you can open the Dashboard, start a new Migration, or manage Jobs. The workspace adapts based on what you're doing.",
    target: "[data-tour='main-content']",
    position: "left",
  },
  {
    id: "new-migration-btn",
    title: "Start a Migration",
    description:
      'Click "New Migration" to open the 7-step migration wizard. It guides you through selecting source and target databases, mapping tables, configuring options, and executing the migration.',
    target: "[data-tour='new-migration-btn']",
    position: "bottom",
  },
  {
    id: "dashboard-btn",
    title: "Open Dashboard",
    description:
      "The Dashboard gives you an overview of your migration activity: total jobs, active jobs, recent runs, and quick action buttons. It's your mission control.",
    target: "[data-tour='dashboard-btn']",
    position: "bottom",
  },
  {
    id: "jobs-btn",
    title: "Job Management",
    description:
      "The Jobs view lets you create, schedule, and monitor recurring migrations and comparisons. Set up cron schedules, chain jobs together, and track execution history.",
    target: "[data-tour='jobs-btn']",
    position: "bottom",
  },
  {
    id: "tab-bar",
    title: "Tab Navigation",
    description:
      "Open comparisons, migrations, and queries appear as tabs here. You can have multiple tabs open and switch between them. Close tabs with the X button or Ctrl+W.",
    target: "[data-tour='tab-bar']",
    position: "bottom",
  },
  {
    id: "bottom-panel",
    title: "Output & Logs",
    description:
      "The output panel shows real-time logs during migrations, connection events, and errors. Drag the top edge to resize it. Toggle visibility with Ctrl+` (backtick).",
    target: "[data-tour='bottom-panel']",
    position: "top",
  },
  {
    id: "status-bar",
    title: "Status Bar",
    description:
      "The status bar at the bottom shows your current theme and app status. It keeps you informed of what's happening in the background.",
    target: "[data-tour='status-bar']",
    position: "top",
  },
  {
    id: "migration-overview",
    title: "Migration Wizard Overview",
    description:
      "The migration wizard has 7 steps:\n\n1. Select Source - Choose the source database\n2. Select Target - Choose the target database\n3. Map Tables - Map source tables to target tables\n4. Configure - Set migration mode (Upsert, Mirror, Append, Merge, Schema Only)\n5. Transform - Add data transformation rules\n6. Dry Run - Preview changes before executing\n7. Execute - Run the migration with real-time progress",
    target: null,
    position: "center",
  },
  {
    id: "migration-modes",
    title: "5 Migration Modes",
    description:
      "Upsert supports 5 migration modes:\n\n\u2022 Upsert - Insert new rows, update existing (no deletes)\n\u2022 Mirror - Make target match source exactly (inserts + updates + deletes)\n\u2022 Append Only - Only insert new rows, never update or delete\n\u2022 Merge - Insert new + update existing, never delete\n\u2022 Schema Only - Compare schemas without touching data",
    target: null,
    position: "center",
  },
  {
    id: "schema-validation",
    title: "Smart Schema Validation",
    description:
      "Upsert automatically validates data against target schemas before migration:\n\n\u2022 Strings exceeding column max_length are automatically truncated\n\u2022 Rows missing required NOT NULL columns are gracefully skipped\n\u2022 Tables are processed in FK dependency order (parents before children)\n\u2022 Dry run warns about schema incompatibilities before you execute",
    target: null,
    position: "center",
  },
  {
    id: "transforms",
    title: "Data Transformations",
    description:
      "The transform pipeline lets you modify data during migration:\n\n\u2022 Rename Column - Map source columns to different target names\n\u2022 Type Cast - Convert between types (string to number, etc.)\n\u2022 Value Map - Replace specific values (e.g., status codes)\n\u2022 Default for Null - Fill NULL values with defaults\n\u2022 Computed Column - Generate new columns from expressions\n\u2022 Row Filter - Include/exclude rows by condition\n\u2022 Drop Column - Remove columns from migration",
    target: null,
    position: "center",
  },
  {
    id: "keyboard-shortcuts",
    title: "Keyboard Shortcuts",
    description:
      "Speed up your workflow with keyboard shortcuts:\n\n\u2022 Ctrl+K - Open command palette\n\u2022 Ctrl+B - Toggle sidebar\n\u2022 Ctrl+` - Toggle output panel\n\u2022 Ctrl+W - Close active tab\n\u2022 Ctrl+F - Search in data diff viewer",
    target: null,
    position: "center",
  },
  {
    id: "complete",
    title: "You're All Set!",
    description:
      "You now know the key features of Upsert. Start by adding a database connection, then explore migrations, comparisons, and scheduled jobs. You can restart this tour anytime from the status bar.\n\nHappy migrating!",
    target: null,
    position: "center",
  },
];

export const useTourStore = create<TourState>()((set, get) => ({
  isActive: false,
  currentStep: 0,
  steps: TOUR_STEPS,

  startTour: () => set({ isActive: true, currentStep: 0 }),
  endTour: () => set({ isActive: false, currentStep: 0 }),

  nextStep: () => {
    const { currentStep, steps } = get();
    if (currentStep < steps.length - 1) {
      const next = currentStep + 1;
      set({ currentStep: next });
      steps[next]?.onEnter?.();
    } else {
      set({ isActive: false, currentStep: 0 });
    }
  },

  prevStep: () => {
    const { currentStep, steps } = get();
    if (currentStep > 0) {
      const prev = currentStep - 1;
      set({ currentStep: prev });
      steps[prev]?.onEnter?.();
    }
  },

  goToStep: (index: number) => {
    const { steps } = get();
    if (index >= 0 && index < steps.length) {
      set({ currentStep: index });
      steps[index]?.onEnter?.();
    }
  },
}));
