PRAGMA journal_mode = WAL;

CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tracks (
  id TEXT PRIMARY KEY,
  project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
  path TEXT NOT NULL UNIQUE,
  title TEXT NOT NULL,
  artist TEXT,
  album TEXT,
  duration_ms INTEGER NOT NULL,
  sample_rate INTEGER NOT NULL,
  channels INTEGER NOT NULL,
  format TEXT NOT NULL,
  file_hash TEXT NOT NULL,
  peaks TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS preferences (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);