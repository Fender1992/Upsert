# Upsert

A cross-platform desktop application for comparing, migrating, and synchronizing databases across 7 database engines.

Built with [Tauri 2](https://v2.tauri.app/) (Rust backend) + React 18 (TypeScript) + Tailwind CSS v4.

## Supported Database Engines

| Engine | Driver | Status |
|--------|--------|--------|
| SQL Server | tiberius (TDS over Rustls) | Full support |
| PostgreSQL | tokio-postgres | Full support |
| MySQL | mysql_async | Full support |
| SQLite | rusqlite (bundled) | Full support |
| MongoDB | mongodb (official driver) | Full support |
| Oracle | - | Stubbed (requires Oracle Instant Client) |
| CosmosDB | - | Stubbed |

## Features

### Schema Comparison
Compare the structure of two databases side by side. Detects differences in tables, columns, data types, indexes, constraints, nullability, default values, and more. Produces a detailed diff report showing additions, removals, and modifications with property-level detail.

### Data Comparison
Row-level comparison between source and target tables using configurable key columns. Identifies rows that need to be inserted, updated, or deleted to bring the target in sync with the source.

### Migration Engine
Five migration modes to fit different use cases:

- **Upsert** -- Insert new rows, update existing ones based on key columns
- **Mirror** -- Full sync: inserts, updates, and deletes to make target match source exactly
- **Append Only** -- Insert new rows only, never modify existing data
- **Merge** -- Update existing rows, skip inserts
- **Schema Only** -- Migrate table structures without moving data

Each mode supports configurable conflict resolution (Source Wins, Target Wins, Newest Wins, Manual Review), batch sizing, transaction modes, retry counts, and automatic rollback.

### ETL Transform Pipeline
Apply transformations during migration:

- **Rename** -- Map a source column to a different target column name
- **Type Cast** -- Convert between data types during transfer
- **Value Map** -- Replace specific values (e.g., status codes to labels)
- **Default for Null** -- Substitute a default value when source is NULL
- **Drop Column** -- Exclude a column from migration

### Cross-Engine Type Mapping
Automatic type translation between engines via a canonical type system. Source native types are mapped to engine-agnostic canonical types, then mapped to the target engine's native types. Covers numeric, string, date/time, binary, boolean, and JSON types across all supported engines.

### AI Chat Assistant
An embedded AI assistant powered by [Ollama](https://ollama.com/) that runs entirely on your machine -- no cloud APIs, no data leaves your desktop.

**Models** (auto-downloaded on first launch):
- `tinyllama` -- Fast responses for simple questions (greetings, SQL syntax, general help)
- `llama3.2:3b` -- Detailed responses for complex queries (schema analysis, migration planning, debugging)
- `nomic-embed-text` -- Embedding model for RAG context retrieval

**RAG Context System:**
When you connect a database, the assistant automatically indexes every table's schema (columns, types, indexes, constraints) into vector embeddings stored locally. When you ask a question, only the most relevant schema chunks are retrieved and injected into the prompt -- keeping context within small model limits while giving accurate, focused answers.

The assistant has full visibility into:
- Connected database schemas (via RAG vector search)
- Active migration status, progress, and errors
- Table-level error messages with row indices for failed migrations
- Dry run results with per-table estimates and warnings
- Schema and data comparison results
- Transform rules and table mappings

**Auto-routing:** In "Auto" mode, simple questions route to `tinyllama` (fast, low resource) while complex questions route to `llama3.2:3b` (more capable). Switch to "Manual" mode to choose the model yourself.

### Job Scheduling
- File-based job store with cron-style scheduling
- Job chaining (run migration B after migration A completes)
- Job history and execution logs

### Reporting & Exports
Generate comparison and migration reports in multiple formats:
- **Markdown** -- Human-readable diff reports
- **HTML** -- Styled reports for sharing
- **CSV** -- Tabular data for spreadsheets
- **JSON** -- Machine-readable for automation

### Security
- Credentials are encrypted at rest using [Stronghold](https://github.com/nickalcala/tauri-plugin-stronghold) (libsodium-based encrypted vault)
- Passwords are never stored in plaintext or in the app database
- All database connections default to **read-only** mode
- Audit logging tracks migration executions with timestamps, affected rows, and connection details
- Sidecar execution is scoped -- only `ollama serve` is permitted

### Dashboard & Onboarding
- Dashboard with quick actions for migrations, comparisons, and jobs
- Guided onboarding wizard for first-time setup
- Interactive app tour highlighting key features
- Command palette (Ctrl+K) for quick navigation

## Quick Start

### Prerequisites

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://rustup.rs/) (stable toolchain)
- Platform build tools:
  - **Windows:** Visual Studio Build Tools with C++ workload, or full Visual Studio with "Desktop development with C++" installed. [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (pre-installed on Windows 10 1803+ and Windows 11)
  - **macOS:** Xcode Command Line Tools (`xcode-select --install`)
  - **Linux:** `build-essential`, `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `libssl-dev`, `curl`, `wget`, `unzip`

### Install & Run

```bash
git clone https://github.com/Fender1992/Upsert.git
cd Upsert
npm install          # Installs JS deps + auto-downloads Ollama sidecar binary
npm run tauri dev    # Starts the full app (Rust backend + React frontend)
```

That's it. On first launch:

1. The app opens with the **onboarding wizard** -- create your first database connection or skip to explore
2. **Ollama starts automatically** as a background sidecar process
3. Required AI models are **downloaded automatically** (~3 GB total, one-time). A progress banner shows download status in the chat drawer
4. Once models are ready, the AI assistant is available via **Ctrl+L** or the chat icon

### Build for Production

```bash
npm run tauri build
```

This produces a platform-native installer:
- **Windows:** `.msi` installer in `src-tauri/target/release/bundle/msi/`
- **macOS:** `.dmg` in `src-tauri/target/release/bundle/dmg/`
- **Linux:** `.deb` / `.AppImage` in `src-tauri/target/release/bundle/`

The Ollama binary is bundled inside the installer -- end users don't need to install Ollama separately.

## Usage

### Connecting a Database

1. Click **+ New Connection** in the sidebar (or use the onboarding wizard)
2. Select the database engine
3. Enter host, port, database name, and credentials
4. Click **Create Connection**
5. Double-click the connection in the sidebar to connect

Connections default to read-only mode. Right-click a connection for additional options (connect, disconnect, edit, delete).

When you connect a database, the AI assistant automatically indexes the schema for contextual chat assistance.

### Running a Migration

1. Click **New Migration** on the main screen or use the command palette (Ctrl+K)
2. The **7-step migration wizard** guides you through:
   - **Step 1:** Select source database
   - **Step 2:** Select target database
   - **Step 3:** Map source tables to target tables
   - **Step 4:** Configure transform rules (optional)
   - **Step 5:** Set migration mode and options
   - **Step 6:** Run a dry run to preview changes
   - **Step 7:** Execute the migration

Migration progress is shown in real-time with per-table status, row counts, and error details.

### Comparing Databases

Select two connected databases and run a schema comparison or data comparison. Results show:
- Added, removed, and modified objects
- Property-level diffs (type changes, nullability, default values)
- Row-level differences with insert/update/delete counts

### Using the AI Assistant

Open the chat drawer with **Ctrl+L**. The assistant can:
- Answer questions about your connected database schemas ("What columns does the users table have?")
- Help write SQL queries for your specific schema
- Explain migration errors and suggest fixes
- Advise on schema design and normalization
- Compare approaches for data migration strategies

The assistant runs 100% locally via Ollama -- your data never leaves your machine.

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl+K | Command palette |
| Ctrl+L | Toggle AI chat drawer |
| Ctrl+B | Toggle sidebar |
| Ctrl+` | Toggle bottom panel (logs) |
| Ctrl+W | Close active tab |
| Enter | Send chat message |
| Shift+Enter | New line in chat |

## Project Structure

```
src-tauri/src/
  db/connectors/        # DatabaseConnector trait + 7 engine implementations
  db/schema.rs          # Engine-agnostic schema types (TableInfo, ColumnInfo, etc.)
  db/comparator.rs      # Schema diff engine
  db/data_comparator.rs # Row-level data diff
  db/migrator.rs        # Migration engine (5 modes)
  db/type_mapper.rs     # Cross-engine type mapping matrix (244 tests)
  db/transformer.rs     # ETL transformation pipeline (7 rule types)
  db/registry.rs        # Connection registry (manages active connections)
  jobs/                 # Job scheduling & execution
  security/             # Credential encryption, audit logging
  commands/             # Tauri IPC command handlers
  appdb.rs              # Embedded SQLite for app state + RAG vector store
  ollama.rs             # Ollama API client (chat streaming, embeddings)
  sidecar.rs            # Ollama sidecar process lifecycle

src/
  components/
    chat/               # AI chat drawer
    connections/         # Connection sidebar, connection dialog
    comparison/          # Schema and data comparison views
    migration/           # 7-step migration wizard
    jobs/                # Job list and dashboard
    onboarding/          # First-run onboarding wizard
    tour/                # Interactive app tour
    shared/              # Tab bar, bottom panel, command palette, status bar
  stores/               # Zustand state management
    connectionStore      # Connection profiles and status
    comparisonStore      # Schema/data diff results
    migrationStore       # Migration config, progress, errors
    chatStore            # Chat messages, model status, streaming state
    uiStore              # UI state (tabs, sidebar, theme, logs)
    settingsStore        # User preferences
  lib/
    tauriCommands.ts     # Typed IPC wrappers for all Tauri commands
    chatContext.ts       # RAG context builder for AI assistant
    queryRouter.ts       # Auto-routes queries to appropriate model
  types/                 # TypeScript type definitions
```

## Testing

```bash
# Rust tests (578 tests)
cargo test --manifest-path src-tauri/Cargo.toml

# Frontend tests (81 tests)
npm run test

# Type checking
npx tsc --noEmit
```

Test breakdown:
- Type mapper: 244 tests (cross-engine type conversion matrix)
- Security: 54 tests (encryption, audit logging)
- Transformer: 36 tests (ETL pipeline rules)
- Jobs: 28 tests (scheduling, execution, chaining)
- Data comparator: 26 tests (row-level diffing)
- Migration engine: 22 tests (5 modes, conflict resolution)
- Schema comparator: 21 tests (diff detection)
- Database connectors: ~50 tests (connection, query execution)
- App database: 15 tests (persistence, RAG vector search, cosine similarity)
- Frontend stores: 81 tests (connection, UI, settings, comparison, migration state)

## Technology Stack

| Layer | Technology |
|-------|-----------|
| Desktop framework | Tauri 2.x |
| Backend | Rust (async with Tokio) |
| Frontend | React 18, TypeScript |
| Styling | Tailwind CSS v4 |
| State management | Zustand |
| Credential storage | tauri-plugin-stronghold (libsodium) |
| App database | SQLite (rusqlite, bundled) |
| AI runtime | Ollama (bundled sidecar) |
| Embedding model | nomic-embed-text (384 dimensions) |
| Chat models | tinyllama, llama3.2:3b |
| IPC | Tauri commands |

## Cross-Platform Sidecar Build

The Ollama binary is downloaded automatically during `npm install` for the current platform. For cross-platform builds (e.g., CI producing installers for all platforms), pass the target triple explicitly:

```bash
# Windows
bash scripts/download-ollama.sh x86_64-pc-windows-msvc

# Linux x64
bash scripts/download-ollama.sh x86_64-unknown-linux-gnu

# Linux ARM64
bash scripts/download-ollama.sh aarch64-unknown-linux-gnu

# macOS (Universal - works for both Intel and Apple Silicon)
bash scripts/download-ollama.sh x86_64-apple-darwin
bash scripts/download-ollama.sh aarch64-apple-darwin
```

## License

MIT
