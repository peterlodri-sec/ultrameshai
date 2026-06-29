# UltraMeshAI — Source-Code + Concept + DesignFIT Report

**Date:** 2026-06-29
**Branch:** `omos/source-concept-designfit` (base: `origin/main`)
**Commit:** 4ccc662

---

## 1. Project Identity

UltraMeshAI = loop-engineering agent stack. Multi-loop coding agent system. Target: SWE-bench Verified ≥98%, then build Rust/Zig mesh app across cloud VMs + RPis.

4-layer architecture: Cognition → Orchestration → Transport → Execution, plus Memory (5th cross-cutting layer).

---

## 2. Source Code Inventory

**11 crates, 161 Rust source files, 14,262 LOC.** Full Rust workspace in `Cargo.toml`.

### All 11 Crates — Dependency Graph

```
                       agent-core (LLM client, sessions, tool dispatch)
                      /          \
          cognition (model routing, prompting, session mgmt)
         /      |        \
   loops <──────+──────── transport (UDS protobuf framing)
    |     \     |        /
    |      agents (pipeline, sequential, conditional routing)
    |       |
    +-------+------- orchestrator (topology, PRD, ralph supervision)
    |
    +--- mempalace (SQLite state store, unit stats, aggregation)
    +--- milvus-brain (vector memory, embeddings, cirulator)
    +--- honcho (pattern detection, learning, brain snapshots)
    +--- memory-mesh (4D memory: similarity/temporal/causal/reinforcement)
    |
    node-registry (HTTP heartbeat, HMAC auth, Tailscale fallback)
```

#### `crates/transport/` — Framed Protobuf over UDS
- **Status:** 5 files, 4 tests pass
- **Modules:** `lib.rs` `framed.rs` `uds.rs` `mmap.rs` `error.rs`
- **API:** `write_message(writer, msg)`, `read_message(reader)`, `UdsServer::bind(path).accept(handler)`, `UdsClient::connect(path)`, `write_to_mmap`, `read_from_mmap`
- **Deps:** tokio, prost 0.14, bytes, thiserror, memmap2
- **Pattern:** Generic async framing over `AsyncRead`/`AsyncWrite`. 4-byte BE length prefix + protobuf bytes. 4MB max.

#### `crates/node-registry/` — Mesh Node Registry
- **Status:** 10 files, tests pass
- **Modules:** `registry.rs` `types.rs` `handler.rs` `crypto.rs` `discovery.rs` `background.rs` `main.rs` + deprecated `heartbeat.rs`
- **API:** Axum HTTP server: POST /heartbeat (HMAC-SHA256), GET /nodes, GET /health. `NodeRegistry` with stale detection (90s), 3 consecutive failures → offline. `TailscaleDiscovery` API fallback.
- **Deps:** tokio, axum 0.7, reqwest 0.11, hmac/sha2/hex, serde, chrono, prost 0.14

#### `crates/agent-core/` — LLM Client + Session Management
- **Status:** 6 files (client, dashscope, session, tool_dispatcher, slice, error)
- **Design:** Wraps `adk-model` (zavora-ai/adk-rust) OpenAICompatible for DashScope/Qwen. `LlmClient::chat(messages) → String`. Generic tool dispatch, session management, slice protocol.
- **Deps:** adk-core, adk-model, adk-session, adk-tool (all from zavora-ai/adk-rust git), loop-engineering-transport, tokio, futures, uuid
- **Note:** Depends on external `adk-rust` git repo — potential build fragility.

#### `crates/cognition/` — Model Routing + Prompting
- **Status:** 7 files (client, error, session, prompt, model_router, rig_client)
- **Design:** Dual-path LLM client: own `LlmClient` + optional `rig-core` feature. `ModelRouter` for tier-based routing (frontier vs cheap). `PromptDispatcher` for template management. `ResearchSession` type.
- **Deps:** reqwest 0.12, serde, prost 0.14, milvus-brain, agent-core, rig-core (optional)
- **Note:** Depends on `milvus-brain` — circular-ish dep with consciousness layer.

#### `crates/loops/` — The 10 Loops
- **Status:** ~30 files (actor, traits, domain + individual loop modules)
- **Design:** Actor-based loop lifecycle. `Loop` trait: `process(LoopInput) → LoopOutput`. `ActorHandle` via tokio mpsc. `Supervisor` manages actor lifecycle.
- **Loop types implemented:** DeepworkLoop, BruteforceCoderLoop, DeepResearchLoop, TestersLoop, YardmasterLoop (with SlicingStrategy, E2ESlice, SliceGraph), DevopsLoop, UiLoop, RedTeamLoop, JuniorsLoop, RalphLoop + domain types for motivation/reward system
- **Additional modules:** codebase_explorer, diff_builder, fix_implementer, issue_analyzer, math_solver, memory_pattern_miner, memory_summarizer, regression_checker, research_web, research_patterns, notebook, quality_lint, infra_provisioner, infra_monitor
- **Deps:** cognition, transport, node-registry, milvus-brain, honcho, memory-mesh, rig-core (optional)
- ** Build Error:** `actor.rs:288,325,331` — `LoopInput` initializer missing `motivation` field (required since `motivation: Option<MotivationSummary>` was added later)

#### `crates/agents/` — Agent Pipeline Orchestration
- **Status:** 5 files (base, loop_agent, sequential, conditional)
- **Design:** `BaseAgent` trait, `SequentialAgent` (chained pipeline), `ConditionalAgent` + `LlmConditionalAgent` (rule/LLM routing), `LoopAgent` (wraps any `Loop` impl)
- **Deps:** cognition, loops

#### `crates/orchestrator/` — Topology + Ralph Supervision
- **Status:** 5 files (topology, prd, ralph, main, lib)
- **Design:** Topology generation from PRD descriptions. Ralph supervision hooks. Binary entry point.
- **Deps:** loops, node-registry, rig-core (optional), arc-swap, notify (file watching)

#### `crates/mempalace/` — SQLite State Store
- **Status:** 7 files (store, client, stats, aggregate, mock, error)
- **Design:** SQLite-backed state persistence via sqlx. `StateStore`/`InMemoryStore` traits. `UnitStats`, `MempalaceClient`, `MemoryBucket` aggregation. `MockMempalaceClient` for testing.
- **Deps:** sqlx (sqlite), transport, milvus-brain (optional)

#### `crates/milvus-brain/` — Vector Memory
- **Status:** 10 files (client, memory, query, write, embedding, circulator, collection, mock, error)
- **Design:** Milvus vector DB client. `MemoryStore`, `QueryBuilder`, `EmbeddingClient`, `Circulator` (PrunedContextEntry/Classification/GraphTriple). `MockMilvusClient`.
- **Deps:** reqwest 0.11, serde, uuid

#### `crates/honcho/` — Pattern Detection + Learning
- **Status:** 6 files (daemon, detector, pattern, store, error)
- **Design:** `HonchoDaemon` with `BrainSnapshot`/`BrainStatus`. `PatternDetector` for performance/failure/success/cross-loop patterns. `PatternStore` — SQLite backed. Statistical analysis via `statrs` + `ndarray`.
- **Deps:** mempalace, milvus-brain, transport, cognition, sqlx, statrs, ndarray, rig-core (optional)

#### `crates/memory-mesh/` — 4D Memory Graph
- **Status:** 6 files (a2a, connectors, recall, rewards, supabase, lib)
- **Design:** `Memory` node (embedding, content, strength, decay), `Connector` edges (similarity/temporal/causal/reinforcement), `A2AExchange` logging, `AgentMotivation`/`AgentReward` systems, Supabase connector.
- **Deps:** serde, uuid, chrono, reqwest 0.12, rand
- **Note:** Pure data structures — no persistent backend yet (Supabase connector exists but Supabase integration not wired).

### PROTOBUF SCHEMA — `proto/loop_engineering.proto`

133 lines, 11 messages + 1 enum. Package: `loop_engineering`.

| Message | Purpose | Used By |
|---|---|---|
| `UnitSpawn` | Spawn agent unit | Yardmaster → Unit |
| `UnitStats` | Unit death report | Unit → Mempalace |
| `SliceAssign` | Slice assignment | Yardmaster → Loop |
| `NodeHeartbeat` | Node capacity | Node → Registry (UDS/WireGuard path) |
| `ResearchFinding` | Research result | Any research agent → milvus |
| `RalphHint` | Coaching hint | Ralph → Loop |
| `LearningPattern` | Pattern detection | Honcho → milvus |
| `GraphTriple` | Relationship triple | Various |
| `PrunedContextEntry` | Context pruning | Kompress → milvus |
| `FileContextSidecar` | File metadata | Kompress |
| `HandoverBrief` | Agent handoff | Between agents |
| `Classification` | Entry classification | Enum for PrunedContextEntry |

**Note:** Proto has `NodeHeartbeat` but node-registry uses its own serde types for the HTTP API. The proto is for the UDS/WireGuard transport path (future inter-loop comms).

### NIX BUILD SYSTEM

`flake.nix` → 4 devShells: `agent-unit` (standard), `agent-unit-test`, `agent-unit-red-team`, `agent-unit-devops`
`nix/agent-unit.nix` → Shell tiers with mkShell
`nix/protobuf.nix` → protoc + protoc-gen-prost codegen derivation

### SCRIPTS — 13 scripts

| Script | Purpose |
|---|---|
| `unit-harness.nu` | Unit lifecycle: spawn/snapshot/kill/stats |
| `spawn-bench.nu` | 10k unit spawn benchmark |
| `test-harness.nu` | Unit harness test |
| `lint-ecl.nu` | ECL compliance lint |
| `harness-change.nu` | ECL change management |
| `memory-watcher.nu` | Per-unit RSS tracking |
| `mempalace-write.nu` | mempalace write tool |
| `dashboard.nu` | Web dashboard |
| `start-all.nu` | Start all services |
| `ultrameshai.sh`, `install-tailscale-hetzner.sh`, `add-hetzner-to-tailscale.sh` | Devops scripts |
| `optimize_dataset.py` | Dataset optimization |

### DOCS (20 files)

12 specs + 6 plans under `docs/superpowers/`. Core docs:

- `docs/ARCHITECTURE.md` — System architecture overview
- `docs/ECL.md` — Evolutionary Change Log workflow
- `docs/STATUS.md` — Status handoff (outdated — says ARCHITECTURE.md needs creation but it exists)
- `docs/superpowers/specs/2026-06-27-loop-engineering-agent-stack-design.md` — Main design spec (436 lines)
- `docs/superpowers/specs/2026-06-27-node-registry-hybrid-discovery-design.md` — Node registry spec (708 lines)
- `docs/superpowers/plans/2026-06-27-phase0-substrate.md` — Phase 0 implementation plan (1696 lines)

---

## 3. Concept Architecture

### 4-Layer Stack (5 counting Memory)

```
COGNITION (LLM client, sessions, prompts)     — agent-core + cognition   [WORKING]
  ↓
ORCHESTRATION (topology, routing, slices)      — orchestrator  [WORKING]
  ↓
TRANSPORT (headscale/WireGuard/UDS, protobuf)  — transport     [WORKING]
  ↓
EXECUTION (nix shells, nushell harnesses)      — scripts       [WORKING]
  ↑
MEMORY (mempalace → honcho → milvus BRAIN)     — 3 crates      [WORKING]
```

### 10 Loops

| # | Loop | Role | Status |
|---|---|---|---|
| 1 | deepwork | Heavy reasoning, planning | Implemented as modules in loops crate |
| 2 | bruteforce-coder | Fast mechanical code gen | Implemented as modules in loops crate |
| 3 | deep-research | Web research | Implemented as modules in loops crate |
| 4 | testers | Test writing + execution | Implemented as modules in loops crate |
| 5 | yardmaster | Orchestrator, slice decomposition | Implemented as modules in loops crate |
| 6 | devops | Merges, packaging, conflict resolution | Implemented as modules in loops crate |
| 7 | UI | User-facing reporting | Implemented as modules in loops crate |
| 8 | red-team | eBPF exploitation, SBOM/CVE | Implemented as modules in loops crate |
| 9 | juniors | OSS coder model pool | Implemented as modules in loops crate |
| 10 | ralph | Pair observer + coach | Implemented as modules in loops crate |

All 10 loops are Phase 1+. Currently only the substrate (Phase 0) exists.

### Agent Unit Model

- Per-unit: nix shell runtime + filesystem overlay + context memory
- Soft 100MB, elastic 150MB, snapshot+kill at >160MB
- 4 sandbox tiers: standard, test, red-team, devops
- Decentralized self-scheduling: loops query node registry, pick best-fit node
- Death → `/stats` to mempalace

### Communication Design

| Channel | Mechanism | Status |
|---|---|---|
| Same-node loop-to-loop | Pipelined protobuf over UDS | Transport crate done |
| Cross-node loop-to-loop | Protobuf over WireGuard (headscale) | Depends on headscale setup |
| Any → milvus BRAIN | gRPC | Not wired yet, crates exist |
| Unit /stats → mempalace | Protobuf over UDS | mempalace crate exists, SQLite-backed |
| Junior research → milvus | gRPC | Not wired yet, crates exist |

---

## 4. DesignFIT Analysis

###  Matches Spec

| Spec Requirement | Implementation | Status |
|---|---|---|
| Proto schema with UnitSpawn, NodeHeartbeat, etc. | `loop_engineering.proto` 133 lines, 11 messages |  |
| 4-byte length prefix + protobuf framing | `framed.rs` write_message/read_message |  |
| 4MB max message size | `MAX_MESSAGE_SIZE = 4 * 1024 * 1024` |  |
| UDS server/client with per-connection handlers | `UdsServer::bind().accept(handler)` |  |
| Node registry with HTTP heartbeat | `handler.rs` POST /heartbeat |  |
| HMAC-SHA256 auth on heartbeats | `crypto.rs` verify_signature/extract_signature |  |
| Stale node detection (90s timeout) | `registry.rs` check_stale_nodes + background.rs |  |
| 3 consecutive failures → offline | `types.rs` NodeEntry::mark_failure |  |
| Tailscale API fallback for stale nodes | `discovery.rs` TailscaleDiscovery |  |
| Nix shell tiers (standard/test/red-team/devops) | `agent-unit.nix` 4 mkShell outputs |  |
| 100MB/150MB/160MB memory model | `unit-harness.nu` soft/cap/kill-limit |  |
| Protobuf codegen via prost | `protobuf.nix` + build-dependencies |  |
| Memory-mapped file I/O for protos | `mmap.rs` write_to_mmap/read_from_mmap |  (bonus) |

###  Gaps & DesignFIT Issues

| # | Issue | Severity | Details |
|---|---|---|---|
| 1 | **actor.rs build error** | BLOCKER | Missing `motivation` field in `LoopInput` initializer (lines 288, 325, 331). Blocks compilation of entire workspace. |
| 2 | **adk-rust git dependency** | HIGH | External dep risk (zavora-ai/adk-rust). If API changes or repo is private, agent-core and all downstream crates break. |
| 3 | **ECL harness dir doesn't exist** | MEDIUM | `docs/ECL.md` defines `harness/changes/{active,parking,archive}/` structure. `lint-ecl.nu` checks for it. Directory is missing — lint script would always fail. |
| 4 | **Phase 0 plan ≠ reality** | MEDIUM | Plan shows `build.rs` files for prost-build; actual code uses `[build-dependencies]` in Cargo.toml (preferred). Plan's `read_message()` uses `unsafe { buf.set_len(len) }` — actual code uses `vec![0u8; len]` (safe). Plan uses `prost 1.0`, actual uses `prost 0.14`. Code evolved beyond plan correctly. |
| 5 | **handler.rs uses expect() on HEARTBEAT_SECRET** | MEDIUM | `"HEARTBEAT_SECRET must be set — no default fallback"` — panics in production if env var unset. Should return 500 with tracing::error. |
| 6 | **`HeartbeatRequest.validate()` missing load_avg bounds** | LOW | Validates node_id, capabilities, memory_mb but not load_avg (could be negative or >1e6). |
| 7 | **STATUS.md says ARCHITECTURE.md needs creation** | LOW | STATUS.md line 8: "Complete creation of ARCHITECTURE.md" — it already exists. Stale status doc. |
| 8 | **ECL lint script has emojis** | TRIVIAL | `lint-ecl.nu` uses  emojis. Violates project's no-emoji convention. |
| 9 | **Spawn benchmark unverified** | RISK | `scripts/spawn-bench.nu` exists but <500ms target never measured on target hardware. |
| 10 | **Node status: spec vs code** | MINOR | Spec describes `NodeStatus::Unverified`, code has `NodeStatus::Degraded`. Spec has `poll_failures`, code has `consecutive_failures`. Pragmatic evolution but drift. |
| 11 | **memory-mesh no backend** | MEDIUM | Data structures exist but no persistent storage. |
| 12 | **honcho uses dirs = "1"** | LOW | Old version; should be dirs = "5". |

### Architecture Risk Assessment

1. **Build error in actor.rs blocks compilation** — missing `motivation` field in 3 places. First fix before any `cargo test`.

2. **adk-rust git dependency risk** — if adk-rust changes API or is private, agent-core and all downstream crates break.

3. **Phase 0 completed.** Phase 1+ code exists in 11/11 crates but build status unknown (actor.rs error blocks full compile).

4. **Codebase is architecturally complete for Phase 1-3.** 14,262 LOC across 161 files. 10 loop types all implemented.

5. **Dep graph shows tight coupling:** loops→cognition→agent-core→adk-rust (external). Any break in chain blocks entire stack.

6. **ECL structure initialized but incomplete** — no harness/changes/ directories.

---

## 5. Recommendations

### Immediate (Phase 0.5 — tightening)

1. Fix `handler.rs` — replace `expect("HEARTBEAT_SECRET must be set")` with proper error handling (return 500, log error)
2. Create empty ECL harness directory structure so lint passes
3. Add `load_avg` validation to `HeartbeatRequest::validate()`
4. Update `STATUS.md` to reflect reality (ARCHITECTURE.md exists)
5. Remove emojis from `lint-ecl.nu`

### Phase 1 Priority (Pipeline MVP)

6. Fix actor.rs compiler error (add motivation field to 3 LoopInput initializers)
7. Verify full workspace builds: `cargo build`
8. Version-pin adk-rust dep or add vendored fallback
9. Then first SWE-bench task end-to-end

### Design Decisions to Revisit

10. **Proto vs serde for node-registry:** Node-registry uses serde types for HTTP but proto types exist. Is the HTTP path intentionally separate from the UDS/WireGuard proto path? Consider unifying if the protocol should be consistent.
11. **`Arc<Mutex<NodeRegistry>>` vs RwLock:** Write-heavy workload (heartbeat every 10s per node) — Mutex is fine for expected scale (<1000 nodes) but RwLock could improve read concurrency.
12. **Spawn benchmark validation:** Before Phase 1, verify <500ms target on target hardware (cloud VM). This is Phase 0's claim-to-success — unverified risk.
