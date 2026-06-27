#!/usr/bin/env nu

# Helper script to write unit stats to mempalace SQLite
# Usage: nu mempalace-write.nu --db-path mempalace.db --unit-id u1 --slice-id s1 --loop-type coder --status completed --peak-memory 120

def main [
  --db-path: string = "mempalace.db"
  --unit-id: string
  --slice-id: string
  --loop-type: string
  --spawned-at: int
  --died-at: int
  --peak-memory: int?
  --status: string = "completed"
  --snapshot-path: string?
] {
  # Build SQL INSERT statement
  let peak_memory_val = if $peak-memory != null { $peak-memory } else { "NULL" }
  let snapshot_path_val = if $snapshot-path != null { $"'($snapshot-path)'" } else { "NULL" }
  
  # Use sqlite3 CLI to write stats
  sqlite3 $db_path $"
    INSERT INTO unit_stats (unit_id, slice_id, loop_type, spawned_at_ms, died_at_ms, peak_memory_mb, status, snapshot_path)
    VALUES ('($unit-id)', '($slice-id)', '($loop-type)', ($spawned-at), ($died-at), ($peak_memory_val), '($status)', ($snapshot_path_val))
    ON CONFLICT(unit_id) DO UPDATE SET
      slice_id = '($slice-id)',
      loop_type = '($loop-type)',
      spawned_at_ms = ($spawned-at),
      died_at_ms = ($died-at),
      peak_memory_mb = ($peak_memory_val),
      status = '($status)',
      snapshot_path = ($snapshot_path_val)
  "
  
  echo $"Written stats for unit ($unit-id) to ($db-path)"
}
