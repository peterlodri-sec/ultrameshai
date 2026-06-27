# Phase 3: mempalace — Short-Term Memory Design Spec

**Date:** 2026-06-27
**Status:** Approved (design phase)
**Target:** Unit lifecycle telemetry store — `/stats` on death

---

## 1. System Identity & Goal

mempalace is the short-term memory layer for the loop-engineering agent stack. It stores unit `/stats` written on death: lifecycle events (spawn, death), memory telemetry (peak MB), and status (completed/killed/failed). Unlike milvus BRAIN (semantic research memory) and honcho (long-term pattern ingest), mempalace is operational — used for runtime monitoring, slice completion tracking, and memory cap verification.

Phase 2 completed: milvus-brain crate with `MemoryStore` trait abstraction. Phase 3 adds mempalace as another `MemoryStore` implementation backed by SQLite.

### Success Criteria

1. SQLite schema supports unit_stats (unit_id, slice_id, loop_type, spawned_at_ms, died_at_ms, peak_memory_mb, status, snapshot_path)
2. Rust client crate provides async write_stats(), get_unit(), query_stats(), aggregate_stats()
3. nushell unit harness writes to mempalace on unit death
4. In-memory mock enables unit tests without SQLite
5. Integration tests verify end-to-end: unit dies -> stats written -> query returns it

### Scope Boundary

This spec covers mempalace only. honcho (long-term pattern ingest) deferred to Phase 3b. mempalace stores raw unit stats; honcho will read mempalace + detect cross-task patterns.

---

## 2. SQLite Schema

### Table: `unit_stats`

```sql
CREATE TABLE unit_stats (
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

CREATE INDEX idx_slice_id ON unit_stats(slice_id);
CREATE INDEX idx_loop_type ON unit_stats(loop_type);
CREATE INDEX idx_status ON unit_stats(status);
CREATE INDEX idx_died_at_ms ON unit_stats(died_at_ms);
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| unit_id | TEXT | Primary key — unique unit identifier |
| slice_id | TEXT | E2E slice this unit was bound to |
| loop_type | TEXT | Which loop spawned this unit ("coder", "tester", "deep-research") |
| spawned_at_ms | INTEGER | Unix timestamp (ms) when unit spawned |
| died_at_ms | INTEGER | Unix timestamp (ms) when unit died |
| peak_memory_mb | INTEGER | Peak memory usage (soft cap 100MB, elastic 150MB, kill 160MB) |
| status | TEXT | Final status: "completed" (normal exit), "killed" (>160MB), "failed" (error) |
| snapshot_path | TEXT | If killed: path to snapshot directory |
| created_at | TEXT | SQLite timestamp when row inserted |

---

## 3. Rust Client Crate Structure

### Crate: `mempalace`

```
crates/mempalace/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API, re-exports
│   ├── client.rs           # MempalaceClient (SQLite connection)
│   ├── stats.rs            # UnitStats struct, From/To protobuf
│   ├── query.rs            # QueryBuilder for filtering
│   ├── aggregate.rs        # Aggregation queries (avg, count, group_by)
│   └── error.rs            # MempalaceError variants
└── tests/
    ├── client_test.rs      # Integration tests (SQLite file)
    └── mock_test.rs        # Unit tests with in-memory mock
```

### Dependencies

```toml
[dependencies]
rusqlite = { version = "0.31", features = ["bundled"] }
tokio = { version = "1.38", features = ["full"] }
thiserror = "1.0"
tracing = "0.1"
# Reuse protobuf types from loop_engineering proto
loop-engineering-transport = { path = "../transport" }
```

### Public API

```rust
use mempalace::{MempalaceClient, UnitStats, QueryBuilder};

// Connect (creates SQLite file if not exists)
let client = MempalaceClient::connect("mempalace.db").await?;

// Write stats on unit death
let stats = UnitStats {
    unit_id: "unit-001".into(),
    slice_id: "slice-001".into(),
    loop_type: "coder".into(),
    spawned_at_ms: 1719500000000,
    died_at_ms: 1719500005000,
    peak_memory_mb: 120,
    status: "completed".into(),
    snapshot_path: None,
};
client.write_stats(stats).await?;

// Query by slice_id + status
let query = QueryBuilder::new()
    .filter_slice_id("slice-001")
    .filter_status("completed")
    .build();
let units = client.query_stats(query).await?;

// Aggregate: avg runtime by loop_type
let agg = client.aggregate_by_loop_type().await?;
for row in agg {
    println!("{}: avg {}ms", row.loop_type, row.avg_runtime_ms);
}
```

### Internal Modules

**`client.rs`**: SQLite connection wrapper (rusqlite in tokio::task::spawn_blocking), connection pooling, retry logic.

**`stats.rs`**: `UnitStats` struct, conversion from protobuf `UnitStats` message.

**`query.rs`**: Fluent QueryBuilder (filter by slice_id, loop_type, status, time_range, memory_range).

**`aggregate.rs`**: Aggregation queries (count by status, avg runtime by loop_type, peak memory distribution).

**`error.rs`**: MempalaceError variants (DatabaseError, QueryError, ValidationError).

---

## 4. MemoryStore Trait Implementation

mempalace implements `MemoryStore` trait (from milvus-brain) for consistency:

```rust
use milvus_brain::MemoryStore;

impl MemoryStore for MempalaceClient {
    async fn write_finding(&self, finding: ResearchFinding) -> Result<()> {
        // mempalace doesn't store research findings
        // This method is optional for non-research stores
        Ok(())
    }

    async fn search(&self, query: QueryBuilder) -> Result<Vec<ResearchFinding>> {
        // mempalace doesn't store research findings
        Ok(vec![])
    }

    async fn delete_finding(&self, _finding_id: &str) -> Result<()> {
        Ok(())
    }
}

// mempalace-specific methods
impl MempalaceClient {
    pub async fn write_stats(&self, stats: UnitStats) -> Result<()> { ... }
    pub async fn query_stats(&self, query: StatsQueryBuilder) -> Result<Vec<UnitStats>> { ... }
    pub async fn aggregate_by_loop_type(&self) -> Result<Vec<AggRow>> { ... }
}
```

**Note:** `MemoryStore` trait may need extension for non-research stores. Alternative: create separate `StatsStore` trait for mempalace/honcho.

**Decision:** Keep `MemoryStore` generic, mempalace implements only relevant methods (write_stats, query_stats). Research-specific methods (write_finding, search) return `Ok(())` or `Err(NotSupported)`.

---

## 5. Integration with Nushell Harness

### Modified `unit-harness.nu`

```nushell
# On unit death, write to mempalace
def "unit kill" [
  unit_id: string
  pid: int
] {
  let workdir = $"/tmp/units/$unit_id"
  let snapshot_path = $"($workdir)/snapshot_((date now | into int))"
  cp -r $workdir $snapshot_path
  kill $pid
  
  # Write stats to mempalace
  let stats = {
    unit_id: $unit_id,
    slice_id: $manifest.slice_id,
    loop_type: $manifest.loop_type,
    spawned_at_ms: $manifest.spawned_at,
    died_at_ms: (date now | into int),
    peak_memory_mb: (get_peak_memory $pid),
    status: "killed",
    snapshot_path: $snapshot_path,
  }
  
  # Call mempalace CLI or write directly to SQLite
  mempalace write $stats
  
  $snapshot_path
}
```

### mempalace CLI (optional)

```bash
# Write stats
mempalace write --unit-id unit-001 --slice-id slice-001 --loop-type coder --status completed --peak-memory 120

# Query
mempalace query --slice-id slice-001 --status completed

# Aggregate
mempalace aggregate --by loop_type
```

**Decision:** Skip CLI for Phase 3 — nushell harness uses Rust client directly via FFI or writes JSON to stdin.

---

## 6. Local Mock for Testing

### In-Memory Stub

```rust
// crates/mempalace/src/mock.rs

use std::collections::HashMap;
use crate::stats::UnitStats;

pub struct MockMempalaceClient {
    stats: RwLock<HashMap<String, UnitStats>>,
}

impl MockMempalaceClient {
    pub fn new() -> Self {
        Self { stats: RwLock::new(HashMap::new()) }
    }

    pub async fn write_stats(&self, stats: UnitStats) -> Result<()> {
        let mut store = self.stats.write().await;
        store.insert(stats.unit_id.clone(), stats);
        Ok(())
    }

    pub async fn query_stats(&self, query: StatsQueryBuilder) -> Result<Vec<UnitStats>> {
        let store = self.stats.read().await;
        let mut results: Vec<_> = store.values().cloned().collect();
        
        // Apply filters
        if let Some(slice_id) = query.slice_id {
            results.retain(|s| s.slice_id == slice_id);
        }
        if let Some(status) = query.status {
            results.retain(|s| s.status == status);
        }
        
        Ok(results)
    }
}
```

---

## 7. Docker-Compose (Optional)

mempalace uses embedded SQLite — no Docker needed. For production deployment with persistence:

```yaml
version: '3.8'

services:
  mempalace:
    image: alpine:latest
    volumes:
      - mempalace_data:/data
    command: sleep infinity  # Placeholder for mempalace daemon (Phase 3b)

volumes:
  mempalace_data:
```

**Decision:** Skip Docker for Phase 3 — SQLite file stored in project root or configured path.

---

## 8. Build Order / Phasing

| Phase | What | Verify |
|-------|------|--------|
| **3.1** | mempalace crate skeleton + mock client | mock tests pass |
| **3.2** | SQLite schema + client connect | SQLite file created |
| **3.3** | write_stats() API | integration test: write unit stats |
| **3.4** | query_stats() API | integration test: query by slice/status |
| **3.5** | aggregate_stats() API | integration test: avg runtime by loop |
| **3.6** | nushell harness integration | unit death writes to mempalace |
| **3.7** | MemoryStore trait impl (optional) | consistent API with milvus |

---

## 9. Open Questions (Deferred)

- Should `MemoryStore` trait be split into `ResearchStore` + `StatsStore`?
- mempalace CLI vs direct Rust client for nushell integration
- Production deployment: single SQLite file vs WAL mode vs PostgreSQL migration
- Retention policy: when to purge old unit stats (after task completion? after N days?)
- honcho integration timeline (Phase 3b — reads mempalace for pattern detection)

---

## Appendix A: UnitStats Protobuf Mapping

From `proto/loop_engineering.proto` (Phase 0):

```protobuf
message UnitStats {
  string unit_id = 1;
  string slice_id = 2;
  string loop_type = 3;
  uint64 spawned_at_ms = 4;
  uint64 died_at_ms = 5;
  uint32 peak_memory_mb = 6;
  string status = 7;            // "completed", "killed", "failed"
  string snapshot_path = 8;    // if killed at >160MB
  bytes stats_blob = 9;        // loop-specific telemetry (deferred to Phase 3b)
}
```

SQLite mapping:
- All fields map 1:1 except `stats_blob` (deferred)
- `status` uses CHECK constraint for valid values
- `snapshot_path` is NULL for non-killed units

---

## Appendix B: Example Queries

```sql
-- All units for a slice
SELECT * FROM unit_stats WHERE slice_id = 'slice-001';

-- Units killed by memory cap
SELECT * FROM unit_stats WHERE status = 'killed' AND peak_memory_mb > 160;

-- Avg runtime by loop_type
SELECT loop_type, AVG(died_at_ms - spawned_at_ms) as avg_runtime_ms
FROM unit_stats
GROUP BY loop_type;

-- Peak memory distribution
SELECT 
  CASE 
    WHEN peak_memory_mb <= 100 THEN '0-100MB'
    WHEN peak_memory_mb <= 150 THEN '100-150MB'
    ELSE '150MB+'
  END as memory_bucket,
  COUNT(*) as unit_count
FROM unit_stats
GROUP BY memory_bucket;

-- Recent failures (last hour)
SELECT * FROM unit_stats 
WHERE status = 'failed' 
  AND died_at_ms > (strftime('%s', 'now') * 1000 - 3600000);
```

(End of file)
