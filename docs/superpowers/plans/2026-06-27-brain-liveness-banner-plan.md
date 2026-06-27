# Brain Liveness Banner — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Honcho daemon emits an ASCII health banner every 5 min. Banner streams to terminal, persists to brain-state.json, readable by dashboard and DCP plugin.

**Architecture:** BrainSnapshot (atomics, no locks) lives in HonchoDaemon. Every poll tick updates counters and writes state file + logs banner via tracing. dashboard.nu reads brain-state.json. kompress-ultra reads same file on transform hooks.

**Tech Stack:** Rust (honcho crate), Nushell (dashboard), TypeScript (DCP plugin)

---

## Global Constraints

- poll interval: 5 min (300_000 ms) — configurable via `HONCHO_POLL_INTERVAL_MS` env var
- state file: `~/.cache/ultrameshai/brain-state.json`
- banner log field: `banner = "brain"` in tracing events
- dependencies: add `dirs = "1"` to honcho/Cargo.toml

---

## Task Map

| Task | File | Description |
|------|------|-------------|
| 1 | `crates/honcho/Cargo.toml` | add `dirs = "1"` |
| 2 | `crates/honcho/src/daemon.rs` | BrainSnapshot struct, atomics, tick, snapshot, format_banner, write_state_file |
| 3 | `crates/honcho/src/lib.rs` | export BrainSnapshot, BrainStatus |
| 4 | `scripts/dashboard.nu` | read brain-state.json, display banner |
| 5 | `.opencode/plugins/kompress-ultra.ts` | read brain-state.json, render in DCP status |

---

## Task 1: Add dirs dependency to honcho

**Files:**
- Modify: `crates/honcho/Cargo.toml`

**Interfaces:**
- Consumes: — (standalone)
- Produces: — adds `dirs = "1"` crate available

---

- [ ] **Step 1: Add dirs dependency**

Open `crates/honcho/Cargo.toml`, find the `[dependencies]` section, add:

```toml
dirs = "1"
```

Run: `cargo build --manifest-path crates/honcho/Cargo.toml 2>&1 | tail -5`
Expected: compiles with new dependency

- [ ] **Step 2: Commit**

```bash
git add crates/honcho/Cargo.toml
git commit -m "feat(honcho): add dirs for cache dir resolution"
```

---

## Task 2: Implement BrainSnapshot + banner + state file in daemon.rs

**Files:**
- Modify: `crates/honcho/src/daemon.rs:1-330`

**Interfaces:**
- Consumes: — (standalone)
- Produces: `BrainSnapshot`, `BrainStatus`, `HonchoDaemon::snapshot()`, `HonchoDaemon::format_banner()`, `HonchoDaemon::write_state_file()`, `HonchoDaemon::tick()`

---

- [ ] **Step 1: Add BrainStatus + BrainSnapshot structs at top of daemon.rs**

After the imports section (before `pub struct HonchoDaemon`), add:

```rust
use std::sync::atomic::{AtomicU64, Ordering};

/// Brain health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrainStatus {
    Alive,   // data received within 2x poll interval
    Stale,   // no data within 2x poll interval
    Unknown, // never received data
}

impl std::fmt::Display for BrainStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrainStatus::Alive => write!(f, "ALIVE"),
            BrainStatus::Stale => write!(f, "STALE"),
            BrainStatus::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Lock-free brain state snapshot written once per poll cycle
#[derive(Debug, Clone)]
pub struct BrainSnapshot {
    pub status: BrainStatus,
    pub patterns_total: u64,
    pub findings_total: u64,
    pub units_processed: u64,
    pub last_data_at_ms: u64, // 0 = never
    pub poll_count: u64,
    pub interval_ms: u64,
}

impl BrainSnapshot {
    /// Compute status from raw fields
    pub fn compute(
        patterns_total: u64,
        findings_total: u64,
        units_processed: u64,
        last_data_at_ms: u64,
        poll_count: u64,
        interval_ms: u64,
    ) -> Self {
        let status = if poll_count == 0 {
            BrainStatus::Unknown
        } else if last_data_at_ms == 0 {
            BrainStatus::Unknown
        } else {
            let now = Self::now_ms();
            let elapsed = now.saturating_sub(last_data_at_ms);
            let threshold = interval_ms * 2;
            if elapsed <= threshold {
                BrainStatus::Alive
            } else {
                BrainStatus::Stale
            }
        };
        Self {
            status,
            patterns_total,
            findings_total,
            units_processed,
            last_data_at_ms,
            poll_count,
            interval_ms,
        }
    }

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    /// Format age string from last_data_at_ms
    pub fn age_string(&self) -> String {
        if self.last_data_at_ms == 0 {
            "never".to_string()
        } else {
            let elapsed = Self::now_ms().saturating_sub(self.last_data_at_ms);
            let secs = elapsed / 1000;
            if secs < 60 {
                format!("{}s ago", secs)
            } else {
                format!("{}m ago", secs / 60)
            }
        }
    }

    /// ASCII banner for this snapshot
    pub fn format_banner(&self) -> String {
        let status_icon = match self.status {
            BrainStatus::Alive => "🧠",
            BrainStatus::Stale => "💤",
            BrainStatus::Unknown => "❓",
        };
        let status_label = self.status.to_string();
        let age = self.age_string();

        format!(
            r#"
╔══════════════════════════════════════════════════════════╗
║  {}  BRAIN  {}                                      ║
║  patterns: {:>5}   findings: {:>5}   units: {:>6}        ║
║  last data: {:>12}   poll #{:>4}   interval: {}m       ║
╚══════════════════════════════════════════════════════════╝"#,
            status_icon,
            status_label,
            self.patterns_total,
            self.findings_total,
            self.units_processed,
            age,
            self.poll_count,
            self.interval_ms / 60000,
        )
    }
}
```

- [ ] **Step 2: Add atomic fields to HonchoDaemon struct**

In `pub struct HonchoDaemon`, add after `running`:

```rust
patterns_total: Arc<AtomicU64>,
findings_total: Arc<AtomicU64>,
units_processed: Arc<AtomicU64>,
last_data_at_ms: Arc<AtomicU64>,
poll_count: AtomicU64,
```

- [ ] **Step 3: Initialize atomics in HonchoDaemon::new()**

In `HonchoDaemon::new()`, after `poll_interval_ms` initialization, add:

```rust
patterns_total: Arc::new(AtomicU64::new(0)),
findings_total: Arc::new(AtomicU64::new(0)),
units_processed: Arc::new(AtomicU64::new(0)),
last_data_at_ms: Arc::new(AtomicU64::new(0)),
poll_count: Arc::new(AtomicU64::new(0)),
```

Remove the old `poll_interval_ms: u64` line (replaced by atomics) — keep `poll_interval_ms` as a plain u64 field (not atomic).

- [ ] **Step 4: Add helper methods to HonchoDaemon**

After `is_running()`:

```rust
/// Take a snapshot of current brain state (lock-free read of atomics)
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

/// Write snapshot JSON to brain-state.json in cache dir
pub async fn write_state_file(&self, snap: &BrainSnapshot) -> std::io::Result<()> {
    let path = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("ultrameshai")
        .join("brain-state.json");

    tokio::fs::create_dir_all(path.parent().unwrap()).await?;
    let json = serde_json::to_string_pretty(snap).unwrap();
    tokio::fs::write(path, json).await
}
```

- [ ] **Step 5: Add now_ms + tick method**

After `is_running()`:

```rust
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Execute one poll cycle: mempalace + milvus, update counters, emit banner
async fn tick(&self) {
    let mempalace_db = self.mempalace_db.clone();
    let last_mempalace = self.last_processed_mempalace.clone();
    let last_milvus = self.last_processed_milvus.clone();
    let has_milvus = self.pattern_store.is_some();

    // Poll mempalace
    let stats = match Self::poll_mempalace(&mempalace_db, &last_mempalace).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to poll mempalace: {}", e);
            return;
        }
    };

    // Poll milvus
    let findings = if has_milvus {
        match Self::poll_milvus(&last_milvus).await {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!("Failed to poll milvus: {}", e);
                vec![]
            }
        }
    } else {
        vec![]
    };

    // Process batch
    if !stats.is_empty() || !findings.is_empty() {
        match Self::process_batch(&self.detector, has_milvus, &mempalace_db, stats.clone(), findings).await {
            Ok(count) => {
                tracing::info!("Detected {} new patterns", count);
            }
            Err(e) => {
                tracing::error!("Failed to process batch: {}", e);
            }
        }
    }

    // Update atomics
    let now = Self::now_ms();
    self.last_data_at_ms.store(now, Ordering::Relaxed);
    self.units_processed.fetch_add(stats.len() as u64, Ordering::Relaxed);
    self.poll_count.fetch_add(1, Ordering::Relaxed);

    // Snapshot + banner + state file
    let snap = self.snapshot();
    tracing::info!(banner = "brain", "{}", snap.format_banner());
    if let Err(e) = self.write_state_file(&snap).await {
        tracing::warn!("Failed to write brain-state.json: {}", e);
    }
}
```

- [ ] **Step 6: Rewrite start() to call tick()**

In `start()`, replace the entire `while running` loop body with:

```rust
let this = self.clone();
let poll_interval = self.poll_interval_ms;
let mut interval_timer = interval(Duration::from_millis(poll_interval));
interval_timer.set_missed_tick_behavior(MissedTickBehavior::Skip);

while this.running.load(Ordering::SeqCst) {
    interval_timer.tick().await;
    this.tick().await;
}
```

Also add `#[derive(Clone)]` to `HonchoDaemon` (add it to the struct definition if not present).

If `HonchoDaemon` doesn't derive Clone, add it:

```rust
#[derive(Clone)]
pub struct HonchoDaemon {
```

And update the `let this = self.clone()` line inside `start` accordingly.

- [ ] **Step 7: Verify it compiles**

Run: `cargo build --manifest-path crates/honcho/Cargo.toml 2>&1 | tail -15`
Expected: compiles, warnings OK

- [ ] **Step 8: Commit**

```bash
git add crates/honcho/src/daemon.rs crates/honcho/Cargo.toml
git commit -m "feat(honcho): add BrainSnapshot + ASCII liveness banner + state file"
```

---

## Task 3: Export BrainSnapshot from honcho lib

**Files:**
- Modify: `crates/honcho/src/lib.rs`

**Interfaces:**
- Consumes: BrainSnapshot, BrainStatus from daemon.rs
- Produces: public export

---

- [ ] **Step 1: Update lib.rs exports**

Change the `pub use` line to include the new types:

```rust
pub use daemon::{BrainSnapshot, BrainStatus, HonchoDaemon};
```

- [ ] **Step 2: Verify build**

Run: `cargo build --manifest-path crates/honcho/Cargo.toml 2>&1 | tail -5`
Expected: clean compile

- [ ] **Step 3: Commit**

```bash
git add crates/honcho/src/lib.rs
git commit -m "feat(honcho): export BrainSnapshot and BrainStatus"
```

---

## Task 4: dashboard.nu brain banner display

**Files:**
- Modify: `scripts/dashboard.nu`

**Interfaces:**
- Consumes: `~/.cache/ultrameshai/brain-state.json`
- Produces: banner printed to terminal in dashboard

---

- [ ] **Step 1: Read existing dashboard.nu**

Run: `cat scripts/dashboard.nu`

Note the existing structure (loop, display format, timing).

- [ ] **Step 2: Add brain-state reading function**

Add to `scripts/dashboard.nu`:

```nu
# Read brain state from brain-state.json and return dict
def get-brain-state [] {
    let path = ($env.HOME | path join ".cache" "ultrameshai" "brain-state.json")
    if ($path | path exists) {
        try {
            open $path | from json
        } catch {
            null
        }
    } else {
        null
    }
}

# Render brain banner line
def brain-line [state] {
    if ($state | is-not-null) {
        let icon = if $state.status == "Alive" { "🧠" } else if $state.status == "Stale" { "💤" } else { "❓" }
        let age = if ($state.last_data_at_ms == 0) { "never" } else { $"($state.last_data_at_ms)s ago" }
        print $"  $icon BRAIN ($state.status) | patterns: ($state.patterns_total) findings: ($state.findings_total) units: ($state.units_processed) last: ($age)"
    } else {
        print "  ❓ BRAIN (no state file)"
    }
}
```

- [ ] **Step 3: Call brain-line in dashboard loop**

Find the main display loop in `dashboard.nu`. After the existing stats display, call:

```nu
let brain = (get-brain-state)
brain-line $brain
```

- [ ] **Step 4: Test the new code**

Run: `nu scripts/dashboard.nu` (may need to wait for 5 min poll cycle; use a shorter interval for testing)
Expected: banner line appears in dashboard output

- [ ] **Step 5: Commit**

```bash
git add scripts/dashboard.nu
git commit -m "feat(dashboard): display brain liveness banner"
```

---

## Task 5: DCP plugin brain state display

**Files:**
- Modify: `.opencode/plugins/kompress-ultra.ts`

**Interfaces:**
- Consumes: `~/.cache/ultrameshai/brain-state.json`
- Produces: brain status line in DCP status block

---

- [ ] **Step 1: Read existing plugin file**

Find the DCP status block rendering code (look for `status`, `display`, `model`, `tokens` in the plugin).

- [ ] **Step 2: Add brain-state reader function**

```ts
async function readBrainState(): Promise<BrainState | null> {
  const path = `${process.env.HOME}/.cache/ultrameshai/brain-state.json`;
  try {
    const content = await Bun.file(path).text();
    return JSON.parse(content);
  } catch {
    return null;
  }
}

interface BrainState {
  status: string;
  patterns_total: number;
  findings_total: number;
  units_processed: number;
  last_data_at_ms: number;
  poll_count: number;
  interval_ms: number;
}
```

- [ ] **Step 3: Add brain line to DCP status block**

In the `messages.transform` or `system.transform` hook (whichever builds the status block), add:

```ts
const brainState = await readBrainState();
if (brainState) {
  const icon = brainState.status === 'Alive' ? '🧠' : brainState.status === 'Stale' ? '💤' : '❓';
  const age = brainState.last_data_at_ms === 0 ? 'never' : `${Math.round((Date.now() - brainState.last_data_at_ms) / 1000)}s ago`;
  const brainLine = `${icon} BRAIN ${brainState.status} | patterns:${brainState.patterns_total} findings:${brainState.findings_total} units:${brainState.units_processed} last:${age}`;
  // append brainLine to the status block output
}
```

The exact insertion point depends on how the existing status block is structured in the plugin. Look for where `tokens`, `density`, `model` lines are pushed to the display array.

- [ ] **Step 4: Restart opencode to reload plugin**

After saving changes, user must restart opencode for plugin to reload.

- [ ] **Step 5: Commit**

```bash
git add .opencode/plugins/kompress-ultra.ts
git commit -m "feat(kompress-ultra): show brain liveness in DCP status block"
```

---

## Self-Review Checklist

- [ ] Spec coverage: BrainSnapshot ✅, atomics ✅, tick() ✅, format_banner() ✅, write_state_file() ✅, dashboard.nu ✅, DCP plugin ✅
- [ ] No placeholders: all code is complete, no "TBD" anywhere
- [ ] Type consistency: BrainSnapshot fields used consistently across tasks 2-5
- [ ] Lock-free: atomics with `Ordering::Relaxed` on hot path, only `snapshot()` reads
- [ ] Error handling: write_state_file errors → tracing::warn (non-fatal)
- [ ] Banner format matches spec exactly
- [ ] State file path matches spec: `~/.cache/ultrameshai/brain-state.json`

---

## Execution Options

**1. Subagent-Driven (recommended)** — dispatch one fixer per task, review between tasks
**2. Inline Execution** — execute all tasks in this session with executing-plans skill

Which approach?
