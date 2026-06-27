# Kompress — Living Context Layer Design Spec

**Date:** 2026-06-28
**Status:** Draft (design phase)
**Target:** Context orchestrator + knowledge circulation for loop-engineering agent stack

---

## 1. System Identity & Goal

Kompress is the bidirectional valve between Layer 5 (milvus BRAIN) and Layer 1 (LLM context). It manages every agent's context window as a living, circulating system — not a passive buffer.

**Core loop:**
```
Agent context grows → kompress prunes → discarded knowledge feeds brain
Brain learns patterns → kompress reads insights → injects into next agent context
Every cycle, the system carries forward what it learned
```

Two roles, one system:
- **Context orchestrator** — assemble optimal context per agent per task
- **Knowledge circulator** — pruned messages don't disappear; they become brain fuel

### Success Criteria

1. 75% token savings on context windows >50 messages without losing task-critical information
2. Zero model collapse when milvus is down (safety floors: recency, user/code/error protection, 50% prune cap)
3. Knowledge circulation: pruned messages → milvus embeddings → brain patterns → next session injection
4. KV cache prefix stability: rewrites never touch system prompt or last 5 messages (TokenPilot compliance)
5. Multi-agent isolation: independent budgets, no cross-agent leakage

### Scope Boundary

Kompress is a context management layer, not a storage layer. It reads/writes milvus but doesn't own the schema. It works with existing honcho daemon, mempalace, and BrainSnapshot. The mesh app and transport layer are out of scope.

---

## 2. Four Roles

### 2.1 Composer — Inject Brain Insights

**When:** Before each turn, in `system.transform` hook.

**What:** Query milvus for task-relevant patterns, findings, and pruned insights. Inject into system prompt as a compact brain state line.

**Format:**
```
🧠 BRAIN Alive | patterns:3 findings:12 units:45 | last:12s ago
```

**Retrieval strategy** (from Cloudflare Agent Memory research):
- 5 parallel channels: FTS, fact-key lookup, raw message search, direct vector, HyDE vector
- RRF fusion with recency tiebreak
- Small model (17B) for classification, large model (120B) only for synthesis

**Token budget:** 50 tokens for brain state line. Patterns/findings injection budget is agent-specific (see AgentTokenBudget.brain_injection_budget, Section 3.5).

**Memory routing** (from Core Concepts taxonomy): Auto-select retrieval source by query type — personal memory vs public knowledge vs task-specific.

### 2.2 Pruner — Drop Low-Signal Messages

**When:** After each LLM turn, in `messages.transform` hook.

**Scoring** (MessageScore):
- Relevance (0.4 weight): Vector similarity to current task goal + recent messages
- Recency (0.3 weight): Ebbinghaus decay curve — newer messages score higher
- Structural importance (0.3 weight): User messages, code blocks, errors, tool results get boost

**Safety floors:**
- Last 5 messages: never pruned (recency protection + KV cache prefix)
- User messages: never pruned (still compressed by Rewriter after age 5)
- Code blocks: never pruned
- Error messages: never pruned
- 50% cap: never drop more than half the context
- Empty context guard: if pruning would leave context empty, skip entirely

**Milvus down fallback:** Score defaults to 0.5 (neutral). Recency protection and 50% cap still apply. Circuit breaker after 3 consecutive failures.

### 2.3 Rewriter — Caveman-Compress Old Messages

**When:** After pruning, before next turn.

**Compression by age** (TokenPilot compliance: only compress tail, never prefix):
| Age (messages from end) | Level | Savings | Treatment |
|------------------------|-------|---------|-----------|
| 0-5 | Verbatim | 0% | Untouched (KV cache prefix) |
| 6-15 | Caveman-lite | 40% | Drop articles, filler, pleasantries. Code exact. |
| 16+ | Caveman-ultra | 75% | Fragments OK. [thing] [action] [reason]. Code exact. |
| Brain-backed | Brain-backed | 90% | Content hash matches existing milvus entry; compress to pointer + key facts |

**Brain-backed check:** Before compressing, hash message content (SHA-256 truncated to 128 bits). Query milvus for matching content_hash. If found, replace with `[brain-ref: {hash_prefix}] + 1-sentence summary`.

**Rewriter implementation:** Deterministic text transform, not an LLM call. Rules-based: drop articles (a/an/the), filler (just/really/basically), pleasantries, hedging. Short synonyms. Fragments OK. Code/API/errors exact. Pattern: `[thing] [action] [reason]`. Runs locally in the plugin, no external dependency.

**Round-trip test:** After compression, verify markdown structure is intact (fences balanced, links valid).

### 2.4 Circulator — Feed Discarded Knowledge to Brain

**When:** After pruning, async background write.

**Pipeline** (from Cloudflare Agent Memory):
1. **Extract:** Pruned messages → structured facts (typed state, from User as Code paper)
2. **Classify:** Facts / Events / Instructions / Tasks (Cloudflare taxonomy)
3. **Supersede:** New facts replace old facts on same topic key. Version chain preserved.
4. **Residual store:** Only embed deltas from existing knowledge (DeltaMem), not full messages
5. **Graph triples:** Extract (subject, predicate, object) triples (G-Long) for relationship-aware retrieval
6. **Append-only log:** Pruned messages logged, then periodically consolidated (User as Code)

**Backpressure handling:**
- Async write, never block the agent
- Queue cap: 100 entries
- Batch flush: every 10 entries or 30 seconds
- On overflow: drop oldest unflushed entries (they're already pruned from context)

**Conflict-driven forgetting** (from Core Concepts): New evidence supersedes old memories. Supersession chains preserve history.

---

## 3. Data Models

### 3.1 MessageScore

```typescript
interface MessageScore {
  relevance: number;   // 0-1, vector similarity to task goal
  recency: number;     // 0-1, Ebbinghaus decay
  structural: number;  // 0-1, user/code/error boost
  total: number;       // weighted sum
  protected: boolean;  // last 5, user, code, error
}
```

### 3.2 CompressionLevel

```typescript
enum CompressionLevel {
  Verbatim = 0,       // 0% savings
  Lite = 1,           // 40% savings
  Ultra = 2,          // 75% savings
  BrainBacked = 3,    // 90% savings (milvus-backed)
}
```

### 3.3 BrainInjection

```typescript
interface BrainInjection {
  patterns: LearningPattern[];    // from honcho
  findings: ResearchFinding[];    // from milvus
  pruned_insights: string[];      // from circulator
  token_budget: number;          // max tokens for injection
}
```

### 3.4 PrunedContextEntry

```typescript
interface PrunedContextEntry {
  session_id: string;
  agent_type: string;
  message_role: string;
  content_hash: string;          // SHA-256 for dedup
  classification: 'fact' | 'event' | 'instruction' | 'task';
  topic_key?: string;            // for supersession
  superseded_by?: string;        // version chain
  triples: { s: string; p: string; o: string }[];
  residual: string;              // delta from existing knowledge
  timestamp_ms: number;
}
```

### 3.5 AgentTokenBudget

```typescript
interface AgentTokenBudget {
  agent_type: string;
  max_context_tokens: number;
  compression_aggressiveness: number;  // 0.0-1.0
  brain_injection_budget: number;
}

const DEFAULT_BUDGETS: Record<string, AgentTokenBudget> = {
  coder: { max_context_tokens: 100_000, compression_aggressiveness: 0.8, brain_injection_budget: 500 },
  researcher: { max_context_tokens: 128_000, compression_aggressiveness: 0.4, brain_injection_budget: 1000 },
  reviewer: { max_context_tokens: 64_000, compression_aggressiveness: 0.6, brain_injection_budget: 500 },
  orchestrator: { max_context_tokens: 128_000, compression_aggressiveness: 0.5, brain_injection_budget: 800 },
};
```

### 3.6 Memory Control Layer

From Core Concepts taxonomy — explicit prioritization + forgetting:

```typescript
interface MemoryControl {
  prioritization: {
    recency_weight: number;      // 0.3
    relevance_weight: number;    // 0.4
    structural_weight: number;   // 0.3
  };
  forgetting: {
    decay_half_life_ms: number;  // Ebbinghaus: 20min, 1hr, 9hr, 1day, ...
    conflict_supersede: boolean; // new evidence replaces old
    privacy_expiration_ms: number; // PII auto-delete
  };
}
```

---

## 4. Execution Flow Per Turn

```
Turn N:
  1. Composer: query milvus → inject brain state + patterns into system prompt
  2. LLM: process context, return completion
  3. Pruner: score all messages → identify candidates below threshold
  4. Rewriter: compress by age (verbatim 0-5, lite 6-15, ultra 16+)
  5. Circulator: async embed pruned messages → milvus
  6. Next turn begins with optimized context
```

**Timing:**
- Composer: <100ms (milvus query + format)
- Pruner: <50ms (scoring is local)
- Rewriter: <200ms (caveman compression is deterministic)
- Circulator: async, never blocks

---

## 5. Error Handling

### 5.1 Milvus Down

- ScoreMessage fallback: 0.0 → 0.5 (neutral, messages survive)
- Recency protection: last 5 messages always kept
- 50% prune cap: never drop more than half
- Circuit breaker: after 3 consecutive failures, skip milvus for 60 seconds
- Composer: brain state shows `💤 BRAIN STALE` or `❓ BRAIN UNKNOWN`

### 5.2 Token Budget Exceeded

Escalation ladder:
1. Truncate brain injection (drop oldest patterns first)
2. Increase compression level (lite → ultra → brain-backed)
3. Increase pruning aggressiveness (lower threshold)
4. Drop oldest unprotected messages

### 5.3 Rewriter Corruption

- Code fences, errors, API names: never touched
- Round-trip markdown test: fences balanced, links valid
- On corruption: revert to verbatim for that message

### 5.4 Circulator Backpressure

- Async write, queue cap 100
- Batch flush: every 10 entries or 30 seconds
- On overflow: drop oldest unflushed (already pruned from context)

### 5.5 Session Boundary

- Flush circulator queue
- Save BrainSnapshot to `~/.cache/ultrameshai/brain-state.json`
- Clear per-session state
- Preserve cross-session patterns in milvus

### 5.6 Multi-Agent Isolation

- Independent budgets per agent type
- Scoped by `(agent_type, session_id, task_scope)` (from adk-rust)
- No cross-agent leakage
- Shared memory profiles for team knowledge (Cloudflare pattern)

---

## 6. Research Comparison

### What others do (and don't do)

| System | Live pruning | Knowledge circulation | Context rewriting | Per-agent composition | Circulator |
|--------|-------------|----------------------|-------------------|----------------------|------------|
| **Kompress** | ✅ | ✅ | ✅ | ✅ | ✅ |
| agentmemory (24.2k★) | ❌ | ❌ | ❌ | ❌ | ❌ |
| adk-rust memory (503★) | ❌ | ❌ | ❌ | ❌ | ❌ |
| ai-memory (851★) | ❌ | ❌ | ❌ | ❌ | ❌ |
| Cloudflare Agent Memory | ❌ | Partial | ❌ | ❌ | ❌ |

### Key borrowings

- **Cloudflare:** Ingestion pipeline (extract → verify → classify → store), 5-channel retrieval with RRF fusion, supersession chains
- **TokenPilot:** Prefix stability for KV cache, dual-granularity (global compaction + local eviction)
- **User as Code:** Append-only log + periodic consolidation, typed state for structured facts
- **DeltaMem:** Residual storage (only embed deltas)
- **G-Long:** Graph triples for relationship-aware retrieval
- **Core Concepts taxonomy:** Memory Control Layer, memory routing, conflict-driven forgetting

---

## 7. Files Touched

| File | Change |
|------|--------|
| `.opencode/plugin/kompress-ultra.ts` | Full rewrite: 4-role architecture, safety floors, circulator |
| `crates/honcho/src/daemon.rs` | No changes (BrainSnapshot already exists) |
| `crates/milvus-brain/` | New circulator writer module |
| `proto/loop_engineering.proto` | New PrunedContextEntry message type |

---

## 8. Testing Strategy

### Unit tests
- ScoreMessage: verify safety floors (last 5, user, code, error never pruned)
- Rewriter: round-trip markdown test, code preservation
- Circulator: classification accuracy, supersession chains
- Token budget: escalation ladder triggers correctly

### Integration tests
- Milvus down: context survives, no collapse
- Multi-agent: no cross-agent leakage
- Session boundary: circulator flush, BrainSnapshot save

### Benchmark
- Token savings: measure context size before/after kompress
- Task retention: SWE-bench slices with/without kompress
- Knowledge circulation: patterns learned in session N available in session N+1

---

## 9. Not In Scope

- Mesh app (Rust/Zig) — future workload
- Transport layer (Headscale/WireGuard) — separate spec
- Model fine-tuning — parked idea
- GitHub sponsors webhook — parked idea
- Daily model card updates — parked idea
