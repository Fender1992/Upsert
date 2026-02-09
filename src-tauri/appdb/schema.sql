-- Connection profiles (passwords stored in Stronghold, never here)
CREATE TABLE IF NOT EXISTS connections (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  engine TEXT NOT NULL,
  host TEXT,
  port INTEGER,
  database_name TEXT,
  username TEXT,
  file_path TEXT,
  read_only INTEGER DEFAULT 1,
  credential_key TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- App settings (key-value)
CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

-- Migration execution history
CREATE TABLE IF NOT EXISTS migration_history (
  id TEXT PRIMARY KEY,
  source_connection_id TEXT,
  target_connection_id TEXT,
  mode TEXT NOT NULL,
  status TEXT NOT NULL,
  config_json TEXT,
  result_json TEXT,
  error TEXT,
  started_at TEXT NOT NULL DEFAULT (datetime('now')),
  completed_at TEXT,
  rows_inserted INTEGER DEFAULT 0,
  rows_updated INTEGER DEFAULT 0,
  rows_deleted INTEGER DEFAULT 0,
  rows_skipped INTEGER DEFAULT 0,
  error_count INTEGER DEFAULT 0,
  duration_ms INTEGER DEFAULT 0
);

-- Chat message history
CREATE TABLE IF NOT EXISTS chat_messages (
  id TEXT PRIMARY KEY,
  role TEXT NOT NULL,
  content TEXT NOT NULL,
  model TEXT,
  timestamp INTEGER NOT NULL
);

-- Audit log
CREATE TABLE IF NOT EXISTS audit_log (
  id TEXT PRIMARY KEY,
  timestamp TEXT NOT NULL DEFAULT (datetime('now')),
  user_name TEXT,
  action TEXT NOT NULL,
  source_connection TEXT,
  target_connection TEXT,
  affected_rows INTEGER,
  details TEXT
);

-- RAG context chunks for chat (vectorized schema context)
CREATE TABLE IF NOT EXISTS context_chunks (
  id TEXT PRIMARY KEY,
  connection_id TEXT,
  chunk_type TEXT NOT NULL,
  label TEXT NOT NULL,
  content TEXT NOT NULL,
  embedding BLOB,
  model TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_context_chunks_connection ON context_chunks(connection_id);
CREATE INDEX IF NOT EXISTS idx_context_chunks_type ON context_chunks(chunk_type);
