-- crates/mempalace/src/schema.sql
CREATE TABLE IF NOT EXISTS unit_stats (
  unit_id TEXT PRIMARY KEY,
  slice_id TEXT NOT NULL,
  loop_type TEXT NOT NULL,
  spawned_at_ms INTEGER NOT NULL,
  died_at_ms INTEGER NOT NULL,
  peak_memory_mb INTEGER,
  status TEXT NOT NULL CHECK(status IN ('completed', 'killed', 'failed')),
  snapshot_path TEXT,
  created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_slice_id ON unit_stats(slice_id);
CREATE INDEX IF NOT EXISTS idx_loop_type ON unit_stats(loop_type);
CREATE INDEX IF NOT EXISTS idx_status ON unit_stats(status);
CREATE INDEX IF NOT EXISTS idx_died_at_ms ON unit_stats(died_at_ms);
