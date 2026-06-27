# Loop-Engineering Agent Stack — Design Spec

**Date:** 2026-06-27
**Status:** Approved (design phase)
**Target:** SWE-bench Verified score >=98%, then build the Rust/Zig mesh app

---

## 1. System Identity & Goal

A multi-loop coding agent system ("loop-engineering") that beats SWE-bench Verified at >=98% by divide-and-conquer: decompose tasks into E2E slices, fan out 100-10k agent units across a headscale mesh of cloud VMs + RPis, each unit sandboxed in a ~100MB nix+nushell runtime.

Single-agent LLM coding plateaus at ~40-50% on SWE-bench. The gap to 98%+ is structural: context isolation, parallel verification, adversarial self-testing, and accumulated learning. Loops solve this by separating cognition types and running them at scale.

### Success Criteria

1. Public SWE-bench Verified score >=98%
2. 10k agent units spawn-ready in <500ms (runtime scaffolding, not LLM tokens)
3. Per-slice eBPF red-team gate + background playground producing SBOMs, signatures, CVE reports
4. milvus BRAIN accumulates patterns across tasks (the "remembers us" effect)
5. Mesh app build deferred — this spec covers the agent stack only

### Scope Boundary

The mesh app (Rust core + Zig meshd) is a FUTURE workload. This spec designs the agent platform that will build it. Transport layer uses headscale (cloud-native), not the custom mesh. Tailscale already set up; Headscale migration = one SWE-bench run (dogfood).

---

## 2. Five-Layer Architecture

| Layer | Responsibility | Implementation |
|-------|---------------|----------------|
| **1. Cognition** | LLM client, per-agent state, prompting | Per-loop model routing (frontier cloud for deepwork/research/yardmaster, local/cheap for coders/testers/devops) |
| **2. Orchestration** | Tool dispatch, topology generation, routing | Yardmaster loop + nix-generated topology (nix instantiates the agent unit graph) |
| **3. Transport** | Inter-node comms, mesh fabric | headscale (cloud-native WireGuard), not custom mesh |
| **4. Execution** | Target daemon, runtimes, sandboxes | nix shells + nushell harnesses per unit, eBPF sandbox for red-team |
| **5. Memory** | Short-term, long-term, semantic BRAIN | mempalace (short-term) -> honcho (long-term) -> milvus (BRAIN, semantic + embeddings + learning spikes) |

Data flow: Cognition produces work -> Orchestration routes -> Transport carries -> Execution runs -> Memory ingests all stages -> feeds back to Cognition.

---

## 3. The Ten Loops

| # | Loop | Role | Model tier | Layer focus |
|---|------|------|-----------|-------------|
| 1 | **deepwork** | Heavy reasoning, planning, hard problem decomposition | frontier (cloud) | Cognition + Memory |
| 2 | **bruteforce-coder** | Fast mechanical code gen, full virtualization (nix+nushell) | high-volume (local/cheap cloud) | Cognition + Execution |
| 3 | **deep-research** | Topic + neighbor domain research, web | frontier + web tools | Cognition + Memory (semantic) |
| 4 | **testers** | Write + run tests, verify E2E slices | local, deterministic | Execution + Memory (short-term) |
| 5 | **yardmaster** | Orchestrates all loops, slices tasks, resolves conflicts, picks pipeline vs wave | frontier, few instances | Orchestration |
| 6 | **devops** | Merges, conflict resolution, nixification, packaging | local, deterministic | Execution + Transport |
| 7 | **UI** | User-facing reporting, interaction, approval gates | frontier | Cognition + Orchestration |
| 8 | **red-team** | eBPF binary exploitation, SBOM/CVE/signature generation | local, deterministic + frontier for novel exploits | Execution + Memory |
| 9 | **juniors** | 8-20B OSS coder models, 1:1 with coders, random pick from OSS pool | OSS 8-20B pool (random pick from 5-6 models) | Cognition + Execution |
| 10 | **ralph** | Pair observer + coach, real-time feedback to tight loop pairs | small local model (fast, cheap) | Cognition + Memory |

### Loop Roles (to be re-brainstormed in detail)

The 10 loops above are the outer architecture. Complex loops (coder, deepwork, red-team) are themselves multi-loop systems and will get separate recursive spec -> plan -> build cycles. Others (devops, UI, juniors, ralph, testers) stay flat.

### Ralph Pair Assignments

Ralph attaches to tight loop pairs only (frequent + high-stakes interaction):

| Ralph | Watches pair | Why |
|-------|-------------|-----|
| ralph-CT | coder + tester | coder output must match tester expectations |
| ralph-RRT | research + red-team | exploit ctx exchange, attack-surface alignment |
| ralph-CJ | coder + junior | delegation quality, junior research burst relevance |
| ralph-DW-R | deepwork + research | plan must match research findings |
| ralph-T-RT | tester + red-team | test coverage vs exploit surface |
| meta-ralph | whole flow | observes entire pipeline/wave execution, merges all pair-ralph observations |

Loose pairs (e.g. devops+UI) don't get ralph — interaction too sparse.

### Ralph Mechanics

- Spawns with the pair, lives as long as the pair's interaction lasts
- Reads both loops' message stream (A2A + blackboard writes)
- Observes: pattern mismatches, repeated failures, context drift, delegation quality
- Coaches: injects short hints into either loop's context mid-flight
- Writes pair interaction patterns to milvus -> learning spikes -> future ralphs benefit
- Dies with the pair, `/stats` to mempalace
- No veto power (observer + coach, not auditor)

### Meta-Ralph

- One meta-ralph watches the entire pipeline/wave execution
- Merges all pair-ralph observations into a flow-level pattern set
- Writes to milvus BRAIN
- Periodically exports to HuggingFace (dataset of agent interaction patterns, exploit signatures, CVEs, learning spikes)
- HF export = public artifact, contributes to "remembers us" + reproducibility

---

## 4. Agent Unit Model & Scheduling

```
Yardmaster
  | decomposes task -> E2E slices
  | assigns slices -> loops (pipeline or wave per slice graph)
  v
+---------------------------------------------+
|  Loop (self-governing)                      |
|  receives N slices from yardmaster          |
|  spawns units as needed, on any node        |
|  that has capacity + matching caps          |
+------------------+--------------------------+
                   | unit spawn request
                   v
+---------------------------------------------+
|  Agent Unit                                  |
|  +----------+ +------+ +---------+          |
|  | nix shell| | fs   | | context |  ~100MB soft, 150MB elastic
|  | (runtime)| |overlay| | (mem)  |  snapshot+kill at >160MB
|  +----------+ +------+ +---------+          |
|  bound to 1 E2E slice                       |
|  on any headscale mesh node                 |
|  reports /stats to mempalace on death       |
+---------------------------------------------+
```

### Scheduling Model (Decentralized, Loop-Self-Scheduled)

| Responsibility | Owner |
|----------------|-------|
| Task -> slice decomposition | yardmaster |
| Slice -> loop assignment | yardmaster |
| Unit spawn + node placement | **each loop** (self-scheduling) |
| Node capacity advertising | **mesh nodes** (publish caps via headscale) |
| Unit lifecycle (spawn/snapshot/kill) | **loop's runtime** (nix shell + nushell) |

### How Loops Self-Schedule

1. Yardmaster assigns slice(s) to a loop.
2. Loop queries mesh node registry (nodes advertise: CPU, RAM free, caps available, nix closures cached).
3. Loop spawns unit on best-fit node. If node fills up, loop picks next or asks yardmaster to rebalance slices.
4. Unit runs slice, reports `/stats` to mempalace on death.
5. Loop tracks its own units, can fan out 1k units across 100 nodes if slices are independent (wave mode).

### Memory Cap

- 100MB soft target
- Elastic to 150MB (1.5x)
- Snapshot + kill at >160MB
- Snapshot saves work before death

### Sandbox Tiers

| Tier | Caps | Used by |
|------|------|---------|
| Standard | nix shell, no special caps | coders, research, deepwork, UI |
| Test | nix shell + project fs + test runner | testers |
| Red-team | nix shell + CAP_NET_ADMIN + eBPF + target binary + fuzzing harness | red-team (VM only) |
| Devops | nix shell + git + merge tools | devops |

---

## 5. Communication & Memory

### Hybrid: Directed Pipeline + Shared Blackboard

```
                    +------------------------------+
                    |      BLACKBOARD (milvus)      |
                    |  all loops R/W, async         |
                    |  embeddings + patterns +      |
                    |  exploit signatures + CVEs    |
                    +------^-------------^---------+
                           |             |
   +-----------+    +------+---+   +----+-----+
   | research  |<-->| red-team |   |  coders  |
   +-----+-----+    +-----+----+   +----+-----+
         |               |              |
         |   A2A tight   |              |
         |   (direct)    |              |
         v               v              v
      +----------------------------------+
      |       BLACKBOARD (milvus)        |
      |   all loops R/W, async          |
      +----------^----------------^-----+
                 |                |
         +-------+--+      +------+----+
         | deepwork |      |  testers  |
         +------+---+      +-----+-----+
                |                |
                +----> coders <---+
                          |
                    +-----v------+
                    |   devops   |
                    +-----+------+
                          |
                     +----v----+
                     |   UI    |
                     +---------+
      yardmaster oversees, reassigns on failure
      meta-ralph observes whole flow
```

### Comms Channels

| Channel | Between | Mechanism | Notes |
|---------|---------|-----------|-------|
| Pipeline (directed) | research->deepwork->coder->tester->red-team->devops->UI | protobuf over UDS (same-node) / WireGuard (cross-node) | Stage ordering per slice |
| A2A tight | research <-> red-team | direct message queue, low-latency | Red-team gets research exploit-domain knowledge, CVE history, attack-surface POV. Research gets red-team findings. |
| A2A tight | coder <-> junior (1:1) | direct queue | delegate subtask, receive draft |
| Junior fan-out | junior -> research burst -> milvus -> coder | <=5min cap, write BRAIN, semantic merge re-inject | Junior autonomous |
| Blackboard (async) | all loops | milvus gRPC, mesh-wide | Cross-loop signals: regressions, conflicts, learning spikes, pattern accumulation |

### Unified Research Memory

All agents doing research (deep-research loop, junior bursts, red-team research phase, learning spikes) share ONE unified memory: milvus BRAIN. While in research mode, the agent IS the BRAIN — its context and milvus are one. When it exits research mode, findings stay in milvus, agent returns to its loop with a digest.

### Memory Hierarchy

| Store | What | Who writes | When |
|-------|------|-----------|------|
| **mempalace** (short-term) | Agent `/stats`: hooks (start, fan-out, dispatch, end), slice state, unit telemetry | all units | on death |
| **honcho** (long-term) | Continuous ingest from mempalace + own observations across tasks | honcho daemon | always running |
| **milvus BRAIN** (semantic) | Unified research memory, embeddings (AST graph + code + docs + exploit patterns + CVEs), learning spike outputs | any agent in research mode + learning spikes | during research + periodically |

mempalace + honcho remain for lifecycle/state tracking. milvus is the single research-scoped consciousness — no fragmentation of research knowledge across agent-local stores.

### Learning Spikes

Background loop fans out deep-research + librarian agents. They ingest recent milvus state, find patterns (e.g. "coder loop fails on IPC struct alignment 80% of time"), write new patterns back to milvus. Yardmaster reads these -> adapts slicing strategy. This is the "remembers us" effect — milvus accumulates institutional knowledge across SWE-bench tasks.

### Junior Research Bursts

- Trigger: junior hits uncertainty mid-subtask (unknown API, unfamiliar pattern, needs CVE info)
- Spawn: junior fans out a short deep-research agent (<=5min hard cap)
- Research burst writes findings to milvus BRAIN (benefits all loops)
- Research findings re-injected into junior -> semantic merge into parent coder's context
- Coder memory expands: `coder_mem ++ junior(all bursts + drafts + observations)`

---

## 6. Transport & IPC

```
+-----------------------------------------------------------------+
|  Mesh node (VM or RPi)                                          |
|                                                                 |
|  Loop A --UDS--> Loop B        same-node: pipelined protobuf    |
|   |                  |          over Unix Domain Socket          |
|   |                  |          (kernel memory-to-memory)       |
|   v                  v                                           |
|  +----------------------------+                                 |
|  |  milvus (gRPC, same node   |                                 |
|  |  or mesh via wireguard)    |                                 |
|  +----------------------------+                                 |
|                                                                 |
+-------------------+---------------------------------------------+
                    | wireguard (headscale)
                    |
+-------------------v---------------------------------------------+
|  Other mesh node                                                |
|  ... loops, units, milvus replicas ...                         |
+-----------------------------------------------------------------+
```

### Transport Matrix

| Path | Mechanism | Why |
|------|-----------|-----|
| Loop-to-loop, same node | Pipelined protobuf over UDS | Zero network stack, kernel mem-to-mem, typed binary, non-blocking queue |
| Loop-to-loop, cross-node | Protobuf over WireGuard (headscale) | Encrypted, mesh-native, already set up |
| Any -> milvus BRAIN | gRPC (UDS same-node, WireGuard cross-node) | milvus native protocol, mesh-wide access |
| Unit `/stats` -> mempalace | Protobuf over UDS (unit dies on same node it spawned) | Lifecycle telemetry, fire-and-forget |
| Junior research burst -> milvus | gRPC into BRAIN | Unified research memory |
| A2A tight (research<->red-team, coder<->junior) | Pipelined protobuf over UDS (co-located when possible) | Low-latency rich context |

### Co-location Preference

Yardmaster + loops try to co-locate tight pairs (research+red-team, coder+junior) on same node -> use UDS. If impossible (node full), fall back to WireGuard cross-node. Loops decide this during self-scheduling.

### Protobuf Schema

One `.proto` package for the whole system. Loops, units, milvus messages, `/stats`, slice assignments, research findings — all typed in shared schema. Versioned. Generated stubs for nix shells (Rust, Zig, Python, nushell bindings as needed).

---

## 7. eBPF Red-Team Playground

Per-slice gate (blocks merge) + background playground (feeds milvus).

### Per-Slice Red-Team Gate

```
Slice passes testers
       |
       v
+------------------------------------------+
|  Red-team gate (per-slice, VM only)      |
|                                          |
|  1. Build slice binary in nix shell      |
|  2. Spawn eBPF sandbox (CAP_NET_ADMIN)  |
|  3. Agents attempt exploitation:        |
|     - IPC struct fuzzing                 |
|     - Buffer overflow injection          |
|     - TUN fd hijack attempts             |
|     - nix closure dep fuzzing (boringtun,|
|       hyper, tokio, x25519-dalek)        |
|  4. eBPF traces: syscalls, memory,      |
|     network, file access                  |
|  5. Verdict: PASS (no exploit) or        |
|     FAIL (exploit found -> reject slice)|
|  6. Outputs: SBOM, signatures, CVE report|
+--------------+---------------------------+
               | PASS
               v
          devops merge
               | FAIL
               v
        reject -> coder gets exploit report
        (via A2A from research, via blackboard)
```

### Background Playground (Continuous)

- Runs against merged builds, not gated per-slice
- Deep fuzzing of nix closure deps (long-running, compute-heavy)
- Exploit patterns -> milvus BRAIN -> learning spikes -> future coders avoid same vulns
- CVE database accumulates over time

### eBPF Observation Layer

- `bpftrace` / custom eBPF programs attach to: syscalls (`enter/exit`), memory allocations, network packets, file descriptors
- Zero-overhead tracing (in-kernel, no ptrace)
- Agents read eBPF ring buffer -> learn exploit signatures -> write to milvus

### Outputs Per Slice

- SBOM (nix generates this natively — `nix path-info` + `nix-store --query --graph`)
- Binary signatures (hashes, reproducible build attestation)
- CVE report (exploit findings + severity + fix recommendation)

### Target Scope

Self (mesh app binaries when built) + nix closure deps. For SWE-bench phase: target = the SWE-bench repo's built artifacts + their deps.

---

## 8. SWE-bench Integration & Verification

```
SWE-bench task (GitHub issue + repo snapshot)
       |
       v
  yardmaster
  +-- deep-research: analyze issue + repo + related issues
  +-- deepwork: decompose into E2E slices (test-bounded)
  +-- assign slices -> loops (pipeline or wave per slice graph)
  |
  |  per slice:
  |   research -> deepwork plan -> coder(+junior) -> tester -> red-team -> devops
  |
  +-- devops: merge all slices -> candidate patch
  +-- run SWE-bench test suite (FAIL_TO_PASS + PASS_TO_PASS)
  |
  +-- if fail: error-driven recalibration
      +-- capture test output + compiler diagnostics + LSP errors
      +-- pipe back to coder via blackboard
      +-- 3-5 iteration cap per slice
      +-- if still fail: yardmaster re-slices or escalates to deepwork
```

### SWE-bench Specifics

- Each task = one repo snapshot + one issue + test suite (FAIL_TO_PASS must pass, PASS_TO_PASS must not regress)
- Agent stack runs in sandboxed container (nix shell with repo + test deps pre-cached)
- Red-team gate runs against the patched binary (if repo has binaries) or skips if pure library
- Score = % of tasks where all FAIL_TO_PASS pass AND all PASS_TO_PASS still pass

### Why 98%+ Is Achievable

1. E2E slice decomposition isolates context (no lost-in-the-middle)
2. 10k-unit fan-out means many candidate patches per slice, testers pick survivors
3. Red-team catches security regressions standard tests miss
4. milvus accumulates patterns across tasks (task N benefits from tasks 1..N-1)
5. Error-driven recalibration (3-5 iterations) catches deep bugs
6. Junior research bursts fill knowledge gaps mid-task
7. Ralph pair coaching prevents context drift mid-flight

### Verification Gate

Public SWE-bench Verified leaderboard submission. Score >=98% = success criterion #1.

---

## 9. Build Order / Phasing

Incremental, each phase verifiable.

| Phase | What | Verify |
|-------|------|--------|
| **0. Substrate** | nix shells + nushell harness + headscale mesh + UDS protobuf | 10k unit spawn <500ms on VM+RPi |
| **1. Pipeline MVP** | yardmaster + deepwork + coder + tester + devops loops, static pipeline only | 1 SWE-bench task end-to-end, any score |
| **2. Memory** | mempalace + honcho + milvus BRAIN + unified research memory | milvus indexes across 100 tasks, patterns retrievable |
| **3. Wave mode** | yardmaster picks pipeline vs wave, parallel slice fan-out | 10 SWE-bench tasks, score >50% |
| **4. Red-team** | eBPF sandbox + per-slice gate + background playground + SBOM/CVE output | red-team blocks 1+ exploitable slice, CVE report generated |
| **5. Juniors** | 9th loop, OSS model pool, 1:1 with coders, research bursts | score >80% |
| **6. Learning spikes** | background deep-research + librarian -> milvus pattern accumulation | score improves over time without code changes |
| **7. Ralph + meta-ralph** | 10th loop, pair observers + coaches, meta-ralph + HF export | pair interaction patterns in milvus, HF dataset published |
| **8. Full system** | all 10 loops, adaptive hybrid, eBPF, learning | SWE-bench Verified >=98% |
| **9. Mesh app** | use the agent stack to build the Rust/Zig mesh app | (out of scope for this spec) |

Phase 0 is the <500ms spawn claim — verifiable independently. Phase 1 is the first end-to-end run. Each phase adds capability + raises score.

---

## 10. Recursion (Scoped)

Complex loops are themselves multi-loop systems. Separate spec -> plan -> build cycles for:

- **Coder loop** (3-4 sub-loops: planner, editor, reviewer, junior-liaison)
- **Deepwork loop** (sub-loops TBD)
- **Red-team loop** (sub-loops TBD)

Other loops (devops, UI, juniors, ralph, testers, research, yardmaster) stay flat at the outer 10-loop level.

Recursive sub-loop design deferred to separate specs. This spec covers the outer architecture only.

---

## Open Questions (Deferred)

- Loop-role detail re-brainstorm (per earlier note)
- Coder sub-loop decomposition (separate spec)
- Deepwork sub-loop decomposition (separate spec)
- Red-team sub-loop decomposition (separate spec)
- vaked / ultrawhale concepts (skipped for now)
- Exact OSS model pool for juniors (5-6 models, TBD)
- milvus deployment topology (single node vs cluster, replication across mesh)
- nix topology generation specifics (how nix instantiates the unit graph)