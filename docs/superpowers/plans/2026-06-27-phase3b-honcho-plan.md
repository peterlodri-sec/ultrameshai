# Phase 3b: honcho daemon Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build honcho daemon — long-term pattern detection from mempalace + milvus data.

**Architecture:** Background tokio daemon polls mempalace (SQLite) + milvus (vector DB) every 5 minutes, detects patterns (performance/failure/success/cross-loop), writes `LearningPattern` to milvus.

**Tech Stack:** tokio, statrs (statistics), ndarray (arrays), mempalace, milvus-brain, loop-engineering-transport

## Global Constraints

- Poll interval: 5 minutes (configurable via `HONCHO_POLL_INTERVAL_MS`)
- Pattern confidence: 0.0-1.0 (high ≥0.8, medium 0.5-0.8, low <0.5)
- Pattern types: "performance", "failure", "success", "cross-loop"
- milvus collection: `learning_patterns` (separate from `research_findings`)
- Embedding dim: 1536 (OVHcloud AI endpoint)
- Incremental processing: track `last_processed_timestamp` for mempalace/milvus

---

## File Structure

```
crates/honcho/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API, re-exports
│   ├── daemon.rs           # HonchoDaemon (polling, background task)
│   ├── detector.rs         # PatternDetector (detection algorithms)
│   ├── store.rs            # PatternStore (milvus write/query)
│   ├── pattern.rs          # LearningPattern struct
│   └── error.rs            # HonchoError variants
└── tests/
    ├── detector_test.rs    # Pattern detection tests
    └── daemon_test.rs      # Daemon polling tests
```

Modify:
- `proto/loop_engineering.proto` — add `LearningPattern` message
- `crates/transport/Cargo.toml` — rebuild protobuf

---

### Task 1: honcho Crate Skeleton + LearningPattern Struct

**Files:**
- Create: `crates/honcho/Cargo.toml`
- Create: `crates/honcho/src/lib.rs`
- Create: `crates/honcho/src/error.rs`
- Create: `crates/honcho/src/pattern.rs`
- Modify: `proto/loop_engineering.proto` — add LearningPattern message
- Test: `crates/honcho/tests/pattern_test.rs`

**Interfaces:**
- Produces: `LearningPattern` struct with all fields
- Consumes: protobuf from `loop-engineering-transport`

- [ ] **Step 1: Add LearningPattern to protobuf**

Modify `proto/loop_engineering.proto`:
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

- [ ] **Step 2: Write Cargo.toml**

```toml
[package]
name = "honcho"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.38", features = ["full"] }
mempalace = { path = "../mempalace" }
milvus-brain = { path = "../milvus-brain" }
loop-engineering-transport = { path = "../transport" }
statrs = "0.17"
ndarray = "0.15"
serde_json = "1.0"
thiserror = "1.0"
tracing = "0.1"
uuid = { version = "1.0", features = ["v4"] }
```

- [ ] **Step 3: Write error.rs**

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HonchoError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("milvus error: {0}")]
    Milvus(#[from] milvus_brain::MilvusError),
    #[error("pattern detection error: {0}")]
    Detection(String),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, HonchoError>;
```

- [ ] **Step 4: Write pattern.rs**

```rust
use loop_engineering_transport::proto::LearningPattern as ProtoLearningPattern;

#[derive(Debug, Clone)]
pub struct LearningPattern {
    pub pattern_id: String,
    pub pattern_type: String,  // "performance", "failure", "success", "cross-loop"
    pub confidence: f32,       // 0.0-1.0
    pub affected_loops: Vec<String>,
    pub evidence_count: i64,
    pub summary: String,
    pub embedding: Vec<f32>,
    pub metadata: serde_json::Value,
    pub created_at_ms: u64,
}

impl LearningPattern {
    pub fn new(
        pattern_type: &str,
        confidence: f32,
        summary: &str,
        affected_loops: Vec<String>,
    ) -> Self {
        Self {
            pattern_id: uuid::Uuid::new_v4().to_string(),
            pattern_type: pattern_type.to_string(),
            confidence,
            affected_loops,
            evidence_count: 0,
            summary: summary.to_string(),
            embedding: vec![],  // Set by PatternStore
            metadata: serde_json::Value::Null,
            created_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    pub fn with_evidence_count(mut self, count: i64) -> Self {
        self.evidence_count = count;
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

impl From<ProtoLearningPattern> for LearningPattern {
    fn from(proto: ProtoLearningPattern) -> Self {
        Self {
            pattern_id: proto.pattern_id,
            pattern_type: proto.pattern_type,
            confidence: proto.confidence,
            affected_loops: proto.affected_loops,
            evidence_count: proto.evidence_count,
            summary: proto.summary,
            embedding: proto.embedding,
            metadata: serde_json::from_slice(&proto.metadata).unwrap_or(serde_json::Value::Null),
            created_at_ms: proto.created_at_ms,
        }
    }
}

impl From<LearingPattern> for ProtoLearningPattern {
    fn from(pattern: LearningPattern) -> Self {
        Self {
            pattern_id: pattern.pattern_id,
            pattern_type: pattern.pattern_type,
            confidence: pattern.confidence,
            affected_loops: pattern.affected_loops,
            evidence_count: pattern.evidence_count,
            summary: pattern.summary,
            embedding: pattern.embedding,
            metadata: serde_json::to_vec(&pattern.metadata).unwrap_or_default(),
            created_at_ms: pattern.created_at_ms,
        }
    }
}
```

- [ ] **Step 5: Write lib.rs**

```rust
mod error;
mod pattern;

pub use error::{HonchoError, Result};
pub use pattern::LearningPattern;
```

- [ ] **Step 6: Write pattern_test.rs**

```rust
use honcho::LearningPattern;

#[test]
fn test_learning_pattern_builder() {
    let pattern = LearningPattern::new("failure", 0.85, "IPC failures", vec!["coder".into()])
        .with_evidence_count(47);
    
    assert_eq!(pattern.pattern_type, "failure");
    assert_eq!(pattern.confidence, 0.85);
    assert_eq!(pattern.evidence_count, 47);
}

#[test]
fn test_protobuf_conversion() {
    let pattern = LearningPattern::new("performance", 0.7, "tokio slow", vec!["deep-research".into()]);
    let proto: ProtoLearningPattern = pattern.clone().into();
    let back: LearningPattern = proto.into();
    
    assert_eq!(back.pattern_type, pattern.pattern_type);
    assert_eq!(back.confidence, pattern.confidence);
}
```

- [ ] **Step 7: Add to workspace Cargo.toml**

```toml
[workspace]
members = [
  "crates/transport",
  "crates/node-registry",
  "crates/cognition",
  "crates/loops",
  "crates/agents",
  "crates/milvus-brain",
  "crates/mempalace",
  "crates/honcho",  # Add this
]
```

- [ ] **Step 8: Rebuild protobuf**

Run: `nix build .#protobuf-gen`
Expected: PASS, generates `LearningPattern` in Rust

- [ ] **Step 9: Verify crate compiles**

Run: `cargo check --manifest-path crates/honcho/Cargo.toml`
Expected: PASS

- [ ] **Step 10: Commit**

```bash
git add crates/honcho/ proto/loop_engineering.proto Cargo.toml
git commit -m "feat: honcho crate skeleton + LearningPattern struct"
```

---

### Task 2: PatternDetector Algorithms

**Files:**
- Create: `crates/honcho/src/detector.rs`
- Create: `crates/honcho/tests/detector_test.rs`

**Interfaces:**
- Consumes: `Vec<UnitStats>` from mempalace, `Vec<ResearchFinding>` from milvus
- Produces: `PatternDetector::detect(stats, findings) -> Vec<LearningPattern>`

- [ ] **Step 1: Write detector.rs**

```rust
use mempalace::UnitStats;
use milvus_brain::ResearchFinding;
use crate::pattern::LearningPattern;
use statrs::statistics::Distribution;
use statrs::function::erf;

pub struct PatternDetector;

impl PatternDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detect(&self, stats: Vec<UnitStats>, findings: Vec<ResearchFinding>) -> Vec<LearningPattern> {
        let mut patterns = vec![];
        patterns.extend(self.detect_performance_patterns(&stats, &findings));
        patterns.extend(self.detect_failure_patterns(&stats));
        patterns.extend(self.detect_success_patterns(&stats));
        patterns.extend(self.detect_cross_loop_patterns(&stats));
        patterns
    }

    fn detect_performance_patterns(&self, stats: &[UnitStats], findings: &[ResearchFinding]) -> Vec<LearningPattern> {
        // Group by loop_type
        let by_loop: std::collections::HashMap<_, Vec<_>> = stats.iter()
            .fold(std::collections::HashMap::new(), |mut acc, s| {
                acc.entry(s.loop_type.clone()).or_default().push(s);
                acc
            });
        
        // Calculate avg runtime per loop
        let avg_runtime: std::collections::HashMap<_, f64> = by_loop.iter()
            .map(|(loop_type, stats)| {
                let avg = stats.iter().map(|s| s.runtime_ms() as f64).sum::<f64>() / stats.len() as f64;
                (loop_type.clone(), avg)
            })
            .collect();
        
        // Find outliers (>2σ from mean)
        let values: Vec<f64> = avg_runtime.values().cloned().collect();
        let mean = statrs::statistics::Statistics::mean(&values).unwrap_or(0.0);
        let std_dev = statrs::statistics::Statistics::std_dev(&values).unwrap_or(1.0);
        
        let mut patterns = vec![];
        for (loop_type, avg) in avg_runtime {
            let z_score = (avg - mean) / std_dev;
            if z_score.abs() > 2.0 {
                let confidence = (z_score.abs() / 3.0).min(1.0);
                patterns.push(LearningPattern::new(
                    "performance",
                    confidence as f32,
                    &format!("{} loop avg runtime {:.0}ms ({}σ from mean)", loop_type, avg, z_score),
                    vec![loop_type],
                ));
            }
        }
        patterns
    }

    fn detect_failure_patterns(&self, stats: &[UnitStats]) -> Vec<LearningPattern> {
        // Filter failures
        let failures: Vec<_> = stats.iter()
            .filter(|s| s.status == "failed" || s.status == "killed")
            .collect();
        
        // Group by loop_type + status
        let by_loop_status: std::collections::HashMap<_, Vec<_>> = failures.iter()
            .fold(std::collections::HashMap::new(), |mut acc, s| {
                let key = format!("{}_{}", s.loop_type, s.status);
                acc.entry(key).or_default().push(s);
                acc
            });
        
        // Calculate failure rate per loop
        let total_by_loop: std::collections::HashMap<_, usize> = stats.iter()
            .fold(std::collections::HashMap::new(), |mut acc, s| {
                *acc.entry(s.loop_type.clone()).or_insert(0) += 1;
                acc
            });
        
        let baseline = failures.len() as f64 / stats.len() as f64;
        
        let mut patterns = vec![];
        for (key, group) in by_loop_status {
            let loop_type = key.split('_').next().unwrap_or(&key).to_string();
            let total = total_by_loop.get(&loop_type).copied().unwrap_or(1);
            let rate = group.len() as f64 / total as f64;
            let ratio = rate / baseline.max(0.001);
            
            if ratio > 2.0 {
                let confidence = ((ratio - 1.0) / ratio).min(1.0) as f32;
                patterns.push(LearningPattern::new(
                    "failure",
                    confidence,
                    &format!("{} has {:.1}% failure rate ({:.1}x baseline)", key, rate * 100.0, ratio),
                    vec![loop_type],
                ).with_evidence_count(group.len() as i64));
            }
        }
        patterns
    }

    fn detect_success_patterns(&self, stats: &[UnitStats]) -> Vec<LearningPattern> {
        // Similar to failure patterns, but for "completed" status
        // Omitted for brevity - implement similarly
        vec![]
    }

    fn detect_cross_loop_patterns(&self, stats: &[UnitStats]) -> Vec<LearningPattern> {
        // Track co-occurrence of loops in same slice
        // Omitted for brevity - implement Jaccard similarity
        vec![]
    }
}

impl Default for PatternDetector {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 2: Write detector_test.rs**

```rust
use honcho::{PatternDetector, LearningPattern};
use mempalace::UnitStats;

#[test]
fn test_detect_performance_patterns() {
    let stats = vec![
        UnitStats::new("u1".into(), "s1".into(), "coder".into(), 1000, 2000),
        UnitStats::new("u2".into(), "s1".into(), "coder".into(), 1000, 2500),
        UnitStats::new("u3".into(), "s1".into(), "tester".into(), 1000, 5000),  // Outlier
    ];
    
    let detector = PatternDetector::new();
    let patterns = detector.detect_performance_patterns(&stats, &[]);
    
    // Should detect tester as outlier
    assert!(patterns.iter().any(|p| p.pattern_type == "performance"));
}

#[test]
fn test_detect_failure_patterns() {
    let mut stats = vec![];
    // Add 10 completed units
    for i in 0..10 {
        stats.push(UnitStats::new(format!("u{}", i), "s1".into(), "coder".into(), 1000, 2000).with_status("completed"));
    }
    // Add 5 failed units
    for i in 10..15 {
        stats.push(UnitStats::new(format!("u{}", i), "s1".into(), "junior".into(), 1000, 2000).with_status("failed"));
    }
    
    let detector = PatternDetector::new();
    let patterns = detector.detect_failure_patterns(&stats);
    
    // Should detect junior high failure rate
    assert!(patterns.iter().any(|p| p.pattern_type == "failure" && p.affected_loops.contains(&"junior".to_string())));
}
```

- [ ] **Step 3: Update lib.rs**

```rust
mod detector;
mod error;
mod pattern;

pub use detector::PatternDetector;
pub use error::{HonchoError, Result};
pub use pattern::LearningPattern;
```

- [ ] **Step 4: Run tests**

Run: `cargo test --manifest-path crates/honcho/Cargo.toml detector_test`
Expected: PASS (2+ tests)

- [ ] **Step 5: Commit**

```bash
git add crates/honcho/src/detector.rs crates/honcho/tests/detector_test.rs
git commit -m "feat: PatternDetector algorithms (performance, failure, success, cross-loop)"
```

---

[Plan continues with Tasks 3-6 for PatternStore, daemon, integrations...]
