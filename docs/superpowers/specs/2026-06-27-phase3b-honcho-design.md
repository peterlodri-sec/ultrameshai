# Phase 3b: honcho daemon — Design Spec

**Date:** 2026-06-27
**Status:** Approved (design phase)
**Target:** Long-term pattern detection from mempalace + milvus — the "remembers us" effect

---

## 1. System Identity & Goal

honcho is the long-term memory layer for the loop-engineering agent stack. Unlike mempalace (short-term unit telemetry) and milvus BRAIN (semantic research memory), honcho detects cross-task patterns: performance correlations, failure clusters, success predictors. It reads mempalace stats + milvus findings, finds patterns, writes `LearningPattern` messages back to milvus for semantic search.

Phase 2 completed: milvus-brain crate (vector DB, embeddings, research findings). Phase 3 completed: mempalace crate (SQLite, unit stats, aggregations). Phase 3b adds honcho daemon for pattern detection.

### Success Criteria

1. honcho daemon polls mempalace + milvus periodically (configurable interval)
2. PatternDetector finds: performance patterns, failure patterns, success patterns, cross-loop patterns
3. LearningPattern protobuf message: pattern_type, confidence, affected_loops, evidence_count, embedding
4. Patterns stored in milvus collection `learning_patterns` (separate from `research_findings`)
5. Yardmaster + loops can query patterns: "find patterns about tokio failures"
6. Background daemon runs continuously, writes patterns without blocking agent execution

### Scope Boundary

This spec covers honcho pattern detection only. Learning spikes (Phase 3c) are separate: background deep-research agents that read honcho patterns, write recommendations. honcho finds patterns; learning spikes act on them.

---

## 2. Pattern Types

| Type | What | Example | Confidence Calculation |
|------|------|---------|----------------------|
| **Performance** | Runtime/memory correlations | "coder loop avg 500ms, but 2s when researching tokio" | Pearson correlation coefficient (p-value < 0.05) |
| **Failure** | Error clusters | "80% IPC struct failures when junior models used" | Failure rate / baseline rate (ratio > 2.0) |
| **Success** | Positive predictors | "slices with red-team gate pass 30% more tests" | Success rate delta (t-test p-value < 0.05) |
| **Cross-loop** | Multi-loop interactions | "research→coder→tester pipeline has 15% rework rate" | Co-occurrence frequency (Jaccard similarity > 0.3) |

### Confidence Thresholds

- **High confidence (≥0.8):** Auto-apply in yardmaster slicing decisions
- **Medium confidence (0.5-0.8):** Surface as recommendations
- **Low confidence (<0.5):** Log for manual review, accumulate evidence

---

## 3. milvus Collection Schema

### Collection: `learning_patterns`

```
Collection: learning_patterns
Primary Key: pattern_id (VARCHAR, max 64)
Vectors: embedding (FLOAT_VECTOR, dim 1536)
Scalar Fields:
  - pattern_type: VARCHAR(32) — "performance", "failure", "success", "cross-loop"
  - confidence: FLOAT (0.0-1.0) — pattern confidence score
  - affected_loops: JSON_ARRAY — ["coder", "tester", "junior"]
  - evidence_count: INT64 — number of data points supporting pattern
  - summary: VARCHAR(4096) — human-readable pattern description
  - metadata: JSON — pattern-specific metadata (correlation coefficients, p-values)
  - created_at: INT64 — Unix timestamp (ms)
  - embedding_model: VARCHAR(32) — which model produced embedding
Indexes:
  - embedding: IVF_FLAT, nlist=1024, metric_type=COSINE
  - pattern_type: inverted index
  - confidence: inverted index (for filtering by threshold)
  - created_at: inverted index (for time-range queries)
```

### LearningPattern Protobuf Message

Add to `proto/loop_engineering.proto`:

```protobuf
message LearningPattern {
  string pattern_id = 1;
  string pattern_type = 2;     // "performance", "failure", "success", "cross-loop"
  float confidence = 3;        // 0.0-1.0
  repeated string affected_loops = 4;
  int64 evidence_count = 5;
  string summary = 6;
  bytes embedding = 7;         // vector embedding (1536 floats)
  bytes metadata = 8;          // JSON metadata (correlation, p-values)
  uint64 created_at_ms = 9;
}
```

---

## 4. honcho daemon Architecture

```
+------------------+     +------------------+
|  mempalace       |     |  milvus BRAIN    |
|  (SQLite)        |     |  (vector DB)     |
+--------+---------+     +--------+---------+
         |                        |
         | poll every 5min        | poll every 5min
         v                        v
+--------------------------------------------------+
|              honcho daemon                       |
|  +------------------+  +----------------------+  |
|  | PatternDetector  |  | PatternStore         |  |
|  | - detect_perf    |  | - write_pattern()    |  |
|  | - detect_failure |  | - query_similar()    |  |
|  | - detect_success |  |                      |  |
|  | - detect_cross   |  |                      |  |
|  +------------------+  +----------------------+  |
+---------------------------+----------------------+
                            |
                            | write patterns
                            v
                   +------------------+
                   |  milvus          |
                   |  learning_patterns|
                   +------------------+
```

### Polling Strategy

- **Interval:** 5 minutes (configurable via `HONCHO_POLL_INTERVAL_MS`)
- **Incremental:** Track `last_processed_timestamp` for mempalace stats + milvus findings
- **Batch:** Process up to 1000 new records per poll (avoid OOM)
- **Retry:** Exponential backoff on milvus/mempalace connection failures

### PatternDetector Algorithms

**Performance patterns:**
```rust
// Group by loop_type + research_topic
// Calculate avg runtime per group
// Find outliers (>2σ from mean)
// Correlate with milvus findings (topic similarity)
```

**Failure patterns:**
```rust
// Filter mempalace: status = 'failed' OR 'killed'
// Group by loop_type + slice_id
// Calculate failure rate per group
// Compare to baseline (overall failure rate)
// Flag if ratio > 2.0
```

**Success patterns:**
```rust
// Filter mempalace: status = 'completed'
// Group by pipeline/wave mode
// Calculate success rate delta
// T-test for statistical significance
```

**Cross-loop patterns:**
```rust
// Track slice handoffs (research→coder→tester)
// Calculate rework rate (slices that loop back)
// Jaccard similarity on loop sequences
// Flag high-co-occurrence patterns
```

---

## 5. Rust Crate Structure

### Crate: `honcho`

```
crates/honcho/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API, re-exports
│   ├── daemon.rs           # honcho daemon (tokio runtime, polling)
│   ├── detector.rs         # PatternDetector (detection algorithms)
│   ├── store.rs            # PatternStore (milvus write/query)
│   ├── pattern.rs          # LearningPattern struct
│   └── error.rs            # HonchoError variants
└── tests/
    ├── detector_test.rs    # Pattern detection tests
    └── daemon_test.rs      # Daemon polling tests
```

### Dependencies

```toml
[dependencies]
tokio = { version = "1.38", features = ["full"] }
mempalace = { path = "../mempalace" }
milvus-brain = { path = "../milvus-brain" }
loop-engineering-transport = { path = "../transport" }
statrs = "0.17"           # Statistical functions (correlation, t-test)
ndarray = "0.15"          # Array operations for embeddings
serde_json = "1.0"
thiserror = "1.0"
tracing = "0.1"
```

### Public API

```rust
use honcho::{HonchoDaemon, PatternDetector, PatternStore};

// Start daemon (background task)
let daemon = HonchoDaemon::new(
    "mempalace.db",
    "http://localhost:19530",
    Duration::from_secs(300),
).await?;
daemon.start().await?;

// Manual pattern detection (for testing)
let detector = PatternDetector::new();
let patterns = detector.detect(mempalace_stats, milvus_findings);

// Write pattern to milvus
let store = PatternStore::connect("http://localhost:19530").await?;
store.write_pattern(patterns[0].clone()).await?;

// Query similar patterns
let query = "tokio failures";
let similar = store.query_similar(query, 10).await?;
```

---

## 6. Integration Points

### Yardmaster Loop

Before slicing tasks, yardmaster queries honcho patterns:

```rust
// Yardmaster receives task
let task = receive_task();

// Query patterns for this task type
let patterns = store.query_similar(&task.description, 5).await?;

// Adjust slicing strategy based on patterns
for pattern in patterns {
    if pattern.confidence > 0.8 {
        apply_pattern(pattern);  // e.g., "use wave mode, not pipeline"
    }
}

// Slice task with adjusted strategy
let slices = yardmaster.slice(task);
```

### Loop Mid-Execution

Loops can query patterns during execution:

```rust
// Deep-research loop hits uncertainty
let topic = "tokio UDS pipelining";
let patterns = store.query_similar(topic, 3).await?;

// Check if patterns recommend junior burst
for pattern in patterns {
    if pattern.pattern_type == "success" && pattern.summary.contains("junior burst") {
        spawn_junior_burst(topic);
    }
}
```

---

## 7. Local Development

### Docker-Compose (reuse milvus stack)

```yaml
# docker-compose.milvus.yml (already exists)
# honcho daemon connects to same milvus instance

services:
  honcho:
    build: ./crates/honcho
    environment:
      - MEMPALACE_DB=/data/mempalace.db
      - MILVUS_URL=http://milvus:19530
      - POLL_INTERVAL_MS=300000
    volumes:
      - honcho_data:/data
    depends_on:
      - milvus

volumes:
  honcho_data:
```

### Local Dev Workflow

```bash
# Start milvus stack
docker-compose -f docker-compose.milvus.yml up -d

# Run honcho daemon locally
cargo run --manifest-path crates/honcho/Cargo.toml -- \
  --mempalace-db mempalace.db \
  --milvus-url http://localhost:19530 \
  --poll-interval-ms 60000

# Generate test patterns
cargo test --manifest-path crates/honcho/Cargo.toml detector_test

# Query patterns via CLI (future)
honcho query "tokio failures"
```

---

## 8. Build Order / Phasing

| Phase | What | Verify |
|-------|------|--------|
| **3b.1** | honcho crate skeleton + LearningPattern struct | compiles, tests pass |
| **3b.2** | PatternDetector algorithms (perf/failure/success/cross) | unit tests for each detector |
| **3b.3** | PatternStore (milvus write/query) | integration test: write + search |
| **3b.4** | honcho daemon (polling, incremental processing) | daemon runs, polls every N ms |
| **3b.5** | Yardmaster integration (read patterns before slicing) | yardmaster adjusts strategy based on patterns |
| **3b.6** | Loop integration (mid-execution queries) | deep-research loop queries patterns |

---

## 9. Open Questions (Deferred)

- Pattern expiration: when to purge old patterns? (after N days? after evidence becomes stale?)
- Confidence decay: should pattern confidence decrease over time without new evidence?
- Pattern conflicts: what if two patterns contradict? (e.g., "use pipeline" vs "use wave")
- Learning spikes (Phase 3c): how do background agents read honcho patterns and write recommendations?
- Pattern visualization: dashboard for humans to browse patterns?

---

## Appendix A: PatternDetector Pseudocode

### Performance Pattern Detection

```rust
fn detect_performance_patterns(stats: Vec<UnitStats>, findings: Vec<ResearchFinding>) -> Vec<LearningPattern> {
    // Group stats by loop_type
    let by_loop = group_by(stats, |s| s.loop_type);
    
    // Calculate avg runtime per loop
    let avg_runtime: HashMap<String, f64> = by_loop.iter()
        .map(|(loop_type, stats)| {
            let avg = stats.iter().map(|s| s.runtime_ms()).sum::<u64>() as f64 / stats.len() as f64;
            (loop_type.clone(), avg)
        })
        .collect();
    
    // Find outliers (>2σ from mean)
    let global_mean = avg_runtime.values().sum::<f64>() / avg_runtime.len() as f64;
    let global_std = std_dev(avg_runtime.values());
    
    let mut patterns = vec![];
    for (loop_type, avg) in avg_runtime {
        let z_score = (avg - global_mean) / global_std;
        if z_score.abs() > 2.0 {
            // Correlate with research topics
            let topics = find_related_topics(&loop_type, &findings);
            patterns.push(LearningPattern {
                pattern_type: "performance".into(),
                confidence: z_score.abs() / 3.0,  // Normalize to 0-1
                summary: format!("{} loop avg runtime {:.0}ms ({}σ from mean)", loop_type, avg, z_score),
                ..
            });
        }
    }
    patterns
}
```

### Failure Pattern Detection

```rust
fn detect_failure_patterns(stats: Vec<UnitStats>) -> Vec<LearningPattern> {
    // Filter failures
    let failures: Vec<_> = stats.iter()
        .filter(|s| s.status == "failed" || s.status == "killed")
        .collect();
    
    // Group by loop_type + status
    let by_loop_status = group_by(&failures, |s| format!("{}_{}", s.loop_type, s.status));
    
    // Calculate failure rate per loop
    let total_by_loop = group_by(stats, |s| s.loop_type.clone());
    let failure_rates: HashMap<String, f64> = by_loop_status.iter()
        .map(|(key, failures)| {
            let loop_type = key.split('_').next().unwrap();
            let total = total_by_loop.get(loop_type).map(|v| v.len()).unwrap_or(1);
            (key.clone(), failures.len() as f64 / total as f64)
        })
        .collect();
    
    // Compare to baseline
    let baseline = failures.len() as f64 / stats.len() as f64;
    
    let mut patterns = vec![];
    for (key, rate) in failure_rates {
        let ratio = rate / baseline;
        if ratio > 2.0 {
            patterns.push(LearningPattern {
                pattern_type: "failure".into(),
                confidence: (ratio - 1.0) / ratio,  // Normalize to 0-1
                summary: format!("{} has {:.1}% failure rate ({:.1}x baseline)", key, rate * 100.0, ratio),
                evidence_count: by_loop_status.get(&key).map(|v| v.len()).unwrap_or(0) as i64,
                ..
            });
        }
    }
    patterns
}
```

---

## Appendix B: Example Patterns

```json
{
  "pattern_id": "pattern-001",
  "pattern_type": "failure",
  "confidence": 0.85,
  "affected_loops": ["coder", "junior"],
  "evidence_count": 47,
  "summary": "IPC struct alignment failures 80% when junior models used for tokio research",
  "metadata": {
    "failure_rate": 0.80,
    "baseline_rate": 0.15,
    "ratio": 5.3,
    "p_value": 0.003
  },
  "created_at_ms": 1719500000000
}

{
  "pattern_id": "pattern-002",
  "pattern_type": "success",
  "confidence": 0.72,
  "affected_loops": ["red-team", "tester"],
  "evidence_count": 23,
  "summary": "Slices with red-team gate pass 30% more tests than direct merge",
  "metadata": {
    "success_rate_delta": 0.30,
    "t_statistic": 2.45,
    "p_value": 0.018
  },
  "created_at_ms": 1719500100000
}
```

(End of file)
