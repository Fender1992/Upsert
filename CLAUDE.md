# Upsert - Database Comparison & Migration Tool

## Architecture
- **Backend**: Tauri 2.x (Rust) - `src-tauri/`
- **Frontend**: React 18 (TypeScript) + Tailwind CSS v4 + Zustand - `src/`
- **IPC**: Tauri commands with auto-generated TypeScript bindings via `tauri-specta`
- **Credential Storage**: `tauri-plugin-stronghold` (encrypted vault)

## Build & Run
```
npm run dev          # Start Vite dev server only
npm run tauri dev    # Start full Tauri app (Rust + React)
npm run build        # Build frontend
npm run tauri build  # Build release binary
cargo test --manifest-path src-tauri/Cargo.toml  # Rust tests
npm run test         # Frontend tests
```

## Project Structure
```
src-tauri/src/
  db/connectors/     # DatabaseConnector trait + 7 engine implementations
  db/schema.rs       # Engine-agnostic schema types
  db/comparator.rs   # Schema diff engine
  db/data_comparator.rs  # Row-level data diff
  db/migrator.rs     # Migration engine (5 modes)
  db/type_mapper.rs  # Cross-engine type mapping matrix
  db/transformer.rs  # ETL transformation pipeline
  jobs/              # Job scheduling & execution
  security/          # Credential encryption, audit logging
  commands/          # Tauri IPC command handlers

src/
  components/        # React components (connections, comparison, migration, jobs, shared, onboarding)
  stores/            # Zustand stores (connection, comparison, migration, ui, settings)
  hooks/             # Custom React hooks
  types/             # TypeScript type definitions
  lib/               # Utility functions
```

## Key Conventions
- All Tauri commands annotated with `#[tauri::command]`
- Rust: use `anyhow::Result` for error handling, `async-trait` for async traits
- Frontend: functional components with hooks, no class components
- All database connections default to **read-only** mode
- Credentials never stored in plaintext - always use Stronghold
- Type mappings go through canonical types: source_native → CanonicalType → target_native

## Database Engines
SQL Server (tiberius), PostgreSQL (tokio-postgres), MySQL (mysql_async), SQLite (rusqlite/bundled), Oracle (stubbed - no Instant Client), MongoDB (mongodb), CosmosDB

## Testing
- Rust: `cargo test` - unit tests alongside source, integration tests in `tests/`
- Frontend: Vitest + React Testing Library
- Oracle connector excluded from all tests (stubbed)
