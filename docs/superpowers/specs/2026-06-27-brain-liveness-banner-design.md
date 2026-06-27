# Brain Liveness Banner — Design Spec

## Status

**Approved** — Peter, 2026-06-27

## Overview

Honcho daemon emits an ASCII health banner every 5 minutes. Banner streams to terminal (via tracing), persists to a state file, and is readable by the dashboard and DCP status plugin.

Goal: make brain liveness visible as a first-class UX signal. No data flowing = stale.

---

## 1. BrainSnapshot

**File:** `crates/honcho/src/daemon.rs`

```rust
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};

#[derive(Debug, Clone)]
pub enum BrainStatus {
    Alive,   // data received within 2x poll interval
    Stale,   // no data within 2x poll interval
    Unknown, // never received data
}

pub struct BrainSnapshot {
    pub status: BrainStatus,
    pub patterns_total: u64,
    pub findings_total: u64,
    pub units_processed: u64,
    pub last_data_at_ms: u64,   // 0 = never
    pub poll_count: u64,
    pub interval_ms: u64,
}

impl BrainSnapshot {
    pub fn compute(patterns: u64, findings: u64, units: u64, last_data_ms: u64, poll_count: u64, interval_ms: u64) -> Self { ... }
    pub fn status(&self) -> BrainStatus { ... }
}
```

All fields are plain data (no locks). `BrainSnapshot` is `Clone + Send + Sync`.

---

## 2. HonchoDaemon changes

### 2a. Fields added

```rust
// atomic counters — no locks on hot path
patterns_total: Arc<AtomicU64>,
findings_total: Arc<AtomicU64>,
units_processed: Arc<AtomicU64>,
last_data_at_ms: Arc<AtomicU64>,  // unix ms of most recent data arrival (mempalace or milvus)
poll_count: Arc<AtomicU64>,
```

### 2b. `HonchoDaemon::new()` — initialize atomics to 0

### 2c. `HonchoDaemon::tick()` — existing poll cycle, rename

The inner `while running` loop body in `start()` becomes a named method:

```rust
async fn tick(&self) {
    // existing: poll mempalace, poll milvus, process_batch
    // NEW: update atomics after successful data receipt
    if !stats.is_empty() || !findings.is_empty() {
        let now = now_ms();
        self.last_data_at_ms.store(now, Ordering::Relaxed);
        self.units_processed.fetch_add(stats.len() as u64, Ordering::Relaxed);
        self.patterns_total.fetch_add(count as u64, Ordering::Relaxed);
        self.poll_count.fetch_add(1, Ordering::Relaxed);
    }
}
```

### 2d. `snapshot()` method — read all atomics, return BrainSnapshot

```rust
pub fn snapshot(&self) -> BrainSnapshot {
    let last = self.last_data_at_ms.load(Ordering::Relaxed);
    let pc = self.poll_count.load(Ordering::Relaxed);
    BrainSnapshot::compute(
        self.patterns_total.load(Ordering::Relaxed),
        self.findings_total.load(Ordering::Relaxed),
        self.units_processed.load(Ordering::Relaxed),
        last,
        pc,
        self.poll_interval_ms,
    )
}
```

### 2e. State file writer

Write `BrainSnapshot` JSON to `~/.cache/ultrameshai/brain-state.json` on every tick (after atomic update).

```rust
async fn write_state_file(&self, snap: &BrainSnapshot) -> std::io::Result<()> {
    let path = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("ultrameshai")
        .join("brain-state.json");

    tokio::fs::create_dir_all(path.parent().unwrap()).await?;
    let json = serde_json::to_string_pretty(snap).unwrap();
    tokio::fs::write(path, json).await
}
```

Call `write_state_file(&snap).await` inside `tick()` after updating atomics.

### 2f. Banner formatter

```rust
pub fn format_banner(snap: &BrainSnapshot) -> String {
    let status_icon = match snap.status() {
        BrainStatus::Alive => "🧠",
        BrainStatus::Stale => "💤",
        BrainStatus::Unknown => "❓",
    };
    let status_label = match snap.status() {
        BrainStatus::Alive => "ALIVE",
        BrainStatus::Stale => "STALE",
        BrainStatus::Unknown => "UNKNOWN",
    };
    let age = if snap.last_data_at_ms == 0 {
        "never".to_string()
    } else {
        let elapsed = now_ms() - snap.last_data_at_ms;
        let secs = elapsed / 1000;
        if secs < 60 {
            format!("{}s ago", secs)
        } else {
            format!("{}m ago", secs / 60)
        }
    };

    format!(
        r#"
╔══════════════════════════════════════════════════════════╗
║  {}  BRAIN  {}                                      ║
║  patterns: {:>5}   findings: {:>5}   units: {:>6}        ║
║  last data: {:>12}   poll #{:>4}   interval: {}m       ║
╚══════════════════════════════════════════════════════════╝"#,
        status_icon, status_label,
        snap.patterns_total, snap.findings_total, snap.units_processed,
        age, snap.poll_count, snap.interval_ms / 60000,
    )
}
```

### 2g. `tick()` calls banner + state file

After updating atomics in `tick()`:

```rust
let snap = self.snapshot();
tracing::info!(banner = "brain", "{}", Self::format_banner(&snap));
let _ = self.write_state_file(&snap).await;
```

### 2h. `now_ms()` helper

```rust
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
```

Add `dirs = "1"` to `Cargo.toml` under `honcho` for `dirs::cache_dir()`.

---

## 3. honcho/lib.rs export

```rust
pub use daemon::{BrainSnapshot, BrainStatus, HonchoDaemon};
```

---

## 4. dashboard.nu changes

**File:** `scripts/dashboard.nu`

Read `~/.cache/ultrameshai/brain-state.json` and display the banner inline.

```nu
def brain-banner [] {
    let state = (open ~/.cache/ultrameshai/brain-state.json | from json)
    print $state.status
    print $state.patterns_total
    # render the full banner
}
```

Call `brain-banner` inside the main dashboard loop (already runs every 5 min).

---

## 5. kompress-ultra DCP plugin changes

**File:** `.opencode/plugins/kompress-ultra.ts`

Read `~/.cache/ultrameshai/brain-state.json` on each `messages.transform` or `system.transform` hook invocation.

Render the banner in the DCP status block (already exists in the plugin — augment it).

```ts
const brainState = await readBrainState();
const brainLine = `${brainState.status_icon} BRAIN ${brainState.status_label} | patterns:${brainState.patterns_total} findings:${brainState.findings_total}`;
```

---

## 6. Files touched

| File | Change |
|------|--------|
| `crates/honcho/Cargo.toml` | add `dirs = "1"` |
| `crates/honcho/src/daemon.rs` | BrainSnapshot, atomics, tick(), snapshot(), format_banner(), write_state_file() |
| `crates/honcho/src/lib.rs` | export BrainSnapshot, BrainStatus |
| `scripts/dashboard.nu` | read brain-state.json, display banner |
| `.opencode/plugins/kompress-ultra.ts` | read brain-state.json, render in DCP status block |

---

## 7. Edge cases

- `~/.cache` doesn't exist — `create_dir_all` handles
- milvus/mempalace unavailable on first poll — `Unknown` status, banner shows `❓ UNKNOWN`
- subsequent poll with data — transitions to `Alive`
- data stops flowing — after 2x interval, transitions to `Stale`
- honcho daemon not running — dashboard shows stale/no data, DCP block shows nothing

---

## 8. No changes to

- milvus-brain (client, embedding, write, query, memory, collection)
- mempalace
- model_router, cognition, transport
- loop_engineering.proto
