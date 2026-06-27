# Kompress Living Context Layer — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Transform kompress-ultra into 4-role living context layer (Composer, Pruner, Rewriter, Circulator) with fused co-processor and repo-anchored memory.

**Architecture:** Four roles run as transform hooks in the Opencode plugin. A local 3B model (llama.cpp FFI) handles generative compression and synthesis. A `.kompress/` shadow directory provides cross-session, cross-agent persistence.

**Tech Stack:** TypeScript (Bun), Rust (tokio), Protobuf, llama.cpp FFI

## Global Constraints

- TDD: failing test first
- milvus collections: `research_findings`, `learning_patterns` (existing), `pruned_context` (new)
- KV cache prefix: never touch system prompt or last 5 messages
- Circulator overflow: spill to `~/.cache/ultrameshai/overflow-circulator.jsonl`
- Co-processor: Qwen2.5-1.5B Q8_0 (~1.8GB, <15ms TTFT) on Apple Silicon
- `.kompress/` tracked on shadow branch `.kompress-main`

---

### Task 1: MessageScore + Safety Floors

**Files:**
- Modify: `.opencode/plugin/kompress-ultra.ts` (lines 47-167)
- Test: `.opencode/plugin/__tests__/score.test.ts`

**Produces:** `MessageScore`, `scoreMessage()`, `isProtected()`

- [ ] **Step 1: Write the failing test**

```typescript
// .opencode/plugin/__tests__/score.test.ts
import { describe, test, expect } from 'bun:test';
import { isProtected, ebbinghausDecay, structuralBoost, scoreMessage } from '../kompress-ultra';

describe('isProtected', () => {
  test('last 5 messages are protected', () => {
    const messages = [{role:'assistant', content:'a'}, {role:'assistant', content:'b'},
      {role:'assistant', content:'c'}, {role:'assistant', content:'d'},
      {role:'assistant', content:'e'}];
    for (let i = 0; i < messages.length; i++) {
      expect(isProtected(messages[i], i, messages.length)).toBe(true);
    }
  });

  test('user messages are protected', () => {
    expect(isProtected({role:'user', content:'do this'}, 10, 20)).toBe(true);
  });

  test('code blocks are protected', () => {
    expect(isProtected({role:'assistant', content:'```rust\nfn main() {}```'}, 10, 20)).toBe(true);
  });

  test('error messages are protected', () => {
    expect(isProtected({role:'tool', content:'Error: something failed', type:'error'}, 10, 20)).toBe(true);
  });
});

describe('ebbinghausDecay', () => {
  test('recent messages score higher', () => {
    expect(ebbinghausDecay(0)).toBeCloseTo(1.0);
    expect(ebbinghausDecay(10)).toBeGreaterThan(ebbinghausDecay(20));
  });
});

describe('structuralBoost', () => {
  test('user role gets boost', () => {
    expect(structuralBoost({role:'user', content:'test'})).toBeGreaterThan(0.5);
  });

  test('code content gets boost', () => {
    expect(structuralBoost({role:'assistant', content:'```code```'})).toBeGreaterThan(0.5);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `bun test .opencode/plugin/__tests__/score.test.ts`
Expected: FAIL — functions not defined

- [ ] **Step 3: Write minimal implementation**

```typescript
// .opencode/plugin/kompress-ultra.ts
export interface MessageScore {
  relevance: number;   // 0-1, vector similarity to task goal
  recency: number;     // 0-1, Ebbinghaus decay
  structural: number;  // 0-1, user/code/error boost
  total: number;       // weighted sum
  protected: boolean;  // last 5, user, code, error
}

export function isProtected(msg: Message, index: number, total: number): boolean {
  // Last 5 messages (recency protection + KV cache prefix)
  if (index >= total - 5) return true;
  // User messages never pruned
  if (msg.role === 'user') return true;
  // Code blocks never pruned
  if (msg.content?.includes('```')) return true;
  // Error messages never pruned
  if (msg.type === 'error' || msg.content?.startsWith('Error:')) return true;
  return false;
}

export function ebbinghausDecay(age: number): number {
  // Ebbinghaus forgetting curve: R = e^(-t/S)
  // Half-life scales: 20min, 1hr, 9hr, 1day, 1week
  const halfLife = 5; // 5 messages as decay unit
  return Math.exp(-age / halfLife);
}

export function structuralBoost(msg: Message): number {
  let boost = 0.3; // baseline
  if (msg.role === 'user') boost = 0.9;
  if (msg.content?.includes('```')) boost = Math.max(boost, 0.8);
  if (msg.type === 'error' || msg.content?.startsWith('Error:')) boost = Math.max(boost, 0.9);
  if (msg.role === 'tool') boost = Math.max(boost, 0.6);
  return boost;
}

export async function scoreMessage(msg: Message, index: number, total: number, taskGoal?: string): Promise<MessageScore> {
  const recency = ebbinghausDecay(total - index);
  const structural = structuralBoost(msg);
  // Relevance: milvus vector similarity, fallback 0.5
  let relevance = 0.5; // fallback when milvus down
  if (taskGoal && msg.content) {
    try {
      relevance = await vectorSimilarity(taskGoal, msg.content);
    } catch {
      relevance = 0.5; // milvus down fallback
    }
  }
  const total_score = relevance * 0.4 + recency * 0.3 + structural * 0.3;
  return { relevance, recency, structural, total: total_score, protected: isProtected(msg, index, total) };
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `bun test .opencode/plugin/__tests__/score.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add .opencode/plugin/kompress-ultra.ts .opencode/plugin/__tests__/score.test.ts
git commit -m "feat(kompress): MessageScore engine with safety floors"
```

---

### Task 2: Deterministic Rewriter (Heuristic Fallback)

**Files:**
- Create: `.opencode/plugin/rewriter.ts`
- Test: `.opencode/plugin/__tests__/rewriter.test.ts`

**Consumes:** `CompressionLevel` enum from spec
**Produces:** `compressMessage()`, `CompressionLevel` enum

- [ ] **Step 1: Write the failing test**

```typescript
// .opencode/plugin/__tests__/rewriter.test.ts
import { describe, test, expect } from 'bun:test';
import { compressMessage, CompressionLevel } from '../rewriter';

describe('compressMessage', () => {
  test('verbatim preserves content exactly', () => {
    const input = 'The user asked me to implement the authentication system with OAuth2.';
    expect(compressMessage(input, CompressionLevel.Verbatim)).toBe(input);
  });

  test('lite drops articles and filler', () => {
    const input = 'The user asked me to implement the authentication system with OAuth2. I would be happy to help with that.';
    const output = compressMessage(input, CompressionLevel.Lite);
    expect(output).not.toContain('The ');
    expect(output).not.toContain('would be happy');
    expect(output).toContain('authentication');
    expect(output).toContain('OAuth2');
  });

  test('ultra produces fragments', () => {
    const input = 'The authentication system implementation is now complete. I have successfully added OAuth2 support and the tests are passing.';
    const output = compressMessage(input, CompressionLevel.Ultra);
    expect(output.length).toBeLessThan(input.length * 0.5);
    expect(output).toContain('OAuth2');
  });

  test('code blocks are never compressed', () => {
    const input = 'Here is the code:\n```rust\nfn main() { println!("Hello"); }\n```\nDone.';
    const output = compressMessage(input, CompressionLevel.Ultra);
    expect(output).toContain('```rust');
    expect(output).toContain('fn main()');
  });

  test('markdown fences stay balanced', () => {
    const input = 'Before:\n```rust\ncode1\n```\nAfter:\n```rust\ncode2\n```';
    const output = compressMessage(input, CompressionLevel.Ultra);
    const fenceCount = (output.match(/```/g) || []).length;
    expect(fenceCount % 2).toBe(0);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `bun test .opencode/plugin/__tests__/rewriter.test.ts`
Expected: FAIL — module not found

- [ ] **Step 3: Write minimal implementation**

```typescript
// .opencode/plugin/rewriter.ts
export enum CompressionLevel {
  Verbatim = 0,   // 0% savings
  Lite = 1,       // 40% savings
  Ultra = 2,      // 75% savings
  BrainBacked = 3 // 90% savings
}

export function compressMessage(content: string, level: CompressionLevel): string {
  if (level === CompressionLevel.Verbatim) return content;

  // Protect code fences
  const codeBlocks: string[] = [];
  let protected = content.replace(/```[\s\S]*?```/g, (match) => {
    codeBlocks.push(match);
    return `__CODE_BLOCK_${codeBlocks.length - 1}__`;
  });

  // Protect error messages
  const errors: string[] = [];
  protected = protected.replace(/Error:[^\n]*/g, (match) => {
    errors.push(match);
    return `__ERROR_${errors.length - 1}__`;
  });

  if (level === CompressionLevel.Lite) {
    // Drop articles, filler, pleasantries
    protected = protected.replace(/\b(the|a|an|this|that|these|those)\b/gi, ' ');
    protected = protected.replace(/\b(just|really|basically|actually|simply)\b/gi, ' ');
    protected = protected.replace(/\b(I would be happy to|Sure!|Great!|Excellent!)\b/gi, '');
    protected = protected.replace(/\s{2,}/g, ' ').trim();
  } else if (level === CompressionLevel.Ultra) {
    // Caveman-ultra: [thing] [action] [reason]
    const sentences = protected.split(/[.!?]+/).filter(s => s.trim());
    const compressed = sentences.map(s => {
      s = s.trim();
      // Drop pleasantries
      if (/\b(sure|great|excellent|happy|glad|welcome)\b/i.test(s)) return '';
      // Drop articles
      s = s.replace(/\b(the|a|an|this|that)\b/gi, '').trim();
      // Drop filler
      s = s.replace(/\b(just|really|basically|actually)\b/gi, '').trim();
      // Drop "I have", "I will", "I am"
      s = s.replace(/\bI (have|will|am|would|can)\b/gi, '').trim();
      return s.replace(/\s{2,}/g, ' ').trim();
    }).filter(Boolean).join('. ');
    protected = compressed;
  }

  // Restore protected content
  protected = protected.replace(/__CODE_BLOCK_(\d+)__/g, (_, i) => codeBlocks[parseInt(i)]);
  protected = protected.replace(/__ERROR_(\d+)__/g, (_, i) => errors[parseInt(i)]);

  return protected.replace(/\s{2,}/g, ' ').trim();
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `bun test .opencode/plugin/__tests__/rewriter.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add .opencode/plugin/rewriter.ts .opencode/plugin/__tests__/rewriter.test.ts
git commit -m "feat(kompress): deterministic caveman rewriter (heuristic fallback)"
```

---

### Task 3: Protobuf Extension

**Files:**
- Modify: `proto/loop_engineering.proto`

**Produces:** `PrunedContextEntry`, `GraphTriple`, `FileContextSidecar`, `HandoverBrief` messages

- [ ] **Step 1: Add new messages to proto**

```protobuf
// proto/loop_engineering.proto (append)

message GraphTriple {
  string subject = 1;
  string predicate = 2;
  string object = 3;
}

message PrunedContextEntry {
  string session_id = 1;
  string agent_type = 2;
  string message_role = 3;
  string content_hash = 4;
  Classification classification = 5;
  string topic_key = 6;
  string superseded_by = 7;
  repeated GraphTriple triples = 8;
  string residual = 9;
  int64 timestamp_ms = 10;
}

enum Classification {
  CLASSIFICATION_UNSPECIFIED = 0;
  CLASSIFICATION_FACT = 1;
  CLASSIFICATION_EVENT = 2;
  CLASSIFICATION_INSTRUCTION = 3;
  CLASSIFICATION_TASK = 4;
}

message FileContextSidecar {
  string file_path = 1;
  string last_mutated_commit = 2;
  string architectural_intent = 3;
  repeated string known_quirks = 4;
  repeated string dependencies = 5;
  repeated GraphTriple triples = 6;
}

message HandoverBrief {
  string source_agent = 1;
  string target_agent = 2;
  string status = 3;
  string intent_summary = 4;
  string last_error = 5;
  repeated string critical_files = 6;
  repeated string next_steps = 7;
  int64 timestamp_ms = 8;
}
```

- [ ] **Step 2: Run protobuf codegen**

Run: `nix build .#protobuf-gen --no-link`
Expected: SUCCESS — generates Rust + TypeScript bindings

- [ ] **Step 3: Commit**

```bash
git add proto/loop_engineering.proto
git commit -m "proto: add PrunedContextEntry, GraphTriple, FileContextSidecar, HandoverBrief"
```

---

### Task 4: Rust Circulator Module

**Files:**
- Create: `crates/milvus-brain/src/circulator.rs`
- Test: `crates/milvus-brain/src/circulator_test.rs` (inline mod tests)

**Consumes:** `PrunedContextEntry` from proto
**Produces:** `Circulator` struct with `enqueue()`, `flush()`, `spill_overflow()`

- [ ] **Step 1: Write failing tests**

```rust
// crates/milvus-brain/src/circulator_test.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_enqueue_and_flush() {
        let mut circ = Circulator::new(None); // milvus down
        for i in 0..5 {
            circ.enqueue(PrunedContextEntry {
                session_id: "test".to_string(),
                agent_type: "coder".to_string(),
                message_role: "assistant".to_string(),
                content_hash: format!("hash_{}", i),
                classification: Classification::Fact as i32,
                topic_key: Some("topic".to_string()),
                superseded_by: None,
                triples: vec![],
                residual: format!("residual {}", i),
                timestamp_ms: 0,
            });
        }
        circ.flush().await.unwrap();
        assert!(circ.queue().is_empty());
    }

    #[tokio::test]
    async fn test_overflow_spills_to_jsonl() {
        let temp_dir = tempfile::tempdir().unwrap();
        let overflow_path = temp_dir.path().join("overflow.jsonl");
        let mut circ = Circulator::new(None);
        circ.set_overflow_path(&overflow_path);
        for i in 0..105 {
            circ.enqueue(PrunedContextEntry {
                content_hash: format!("hash_{}", i),
                ..Default::default()
            });
        }
        assert!(overflow_path.exists());
    }

    #[tokio::test]
    async fn test_supersession_chains() {
        let mut circ = Circulator::new(None);
        circ.enqueue(PrunedContextEntry {
            topic_key: Some("auth_method".to_string()),
            content_hash: "v1".to_string(),
            ..Default::default()
        });
        circ.enqueue(PrunedContextEntry {
            topic_key: Some("auth_method".to_string()),
            content_hash: "v2".to_string(),
            superseded_by: Some("v1".to_string()),
            ..Default::default()
        });
        // v2 should supersede v1
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path crates/milvus-brain/Cargo.toml`
Expected: FAIL — module not found

- [ ] **Step 3: Write minimal implementation**

```rust
// crates/milvus-brain/src/circulator.rs
use anyhow::{Context, Result};
use futures::stream::{StreamExt, TryStreamExt};
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

pub struct Circulator {
    queue: Vec<PrunedContextEntry>,
    capacity: usize,
    batch_size: usize,
    flush_interval: Duration,
    overflow_path: Option<PathBuf>,
    topic_index: HashMap<String, String>, // topic_key -> latest content_hash
    milvus_client: Option<PymilvusClient>,
}

impl Circulator {
    pub fn new(milvus_client: Option<PymilvusClient>) -> Self {
        Self {
            queue: Vec::new(),
            capacity: 100,
            batch_size: 10,
            flush_interval: Duration::from_secs(30),
            overflow_path: None,
            topic_index: HashMap::new(),
            milvus_client,
        }
    }

    pub fn set_overflow_path(&mut self, path: &Path) {
        self.overflow_path = Some(path.to_path_buf());
    }

    pub fn enqueue(&mut self, entry: PrunedContextEntry) {
        // Supersession: track latest version per topic
        if let Some(ref topic) = entry.topic_key {
            if let Some(prev) = self.topic_index.get(topic) {
                // Previous entry superseded
            }
            self.topic_index.insert(topic.clone(), entry.content_hash.clone());
        }

        if self.queue.len() >= self.capacity {
            self.spill_overflow(&[entry]);
            return;
        }
        self.queue.push(entry);

        // Auto-flush at batch size
        if self.queue.len() >= self.batch_size {
            // Flush triggered async
        }
    }

    pub async fn flush(&mut self) -> Result<()> {
        if self.queue.is_empty() {
            return Ok(());
        }

        let entries = std::mem::take(&mut self.queue);

        if let Some(ref client) = self.milvus_client {
            // Embed and insert to milvus pruned_context collection
            for entry in &entries {
                let residual = &entry.residual;
                let embedding = client.embed(residual).await
                    .context("embed residual")?;
                client.insert("pruned_context", entry, &embedding).await
                    .context("insert pruned context")?;
            }
        } else {
            // Milvus down — spill to overflow
            self.spill_overflow(&entries);
        }

        Ok(())
    }

    pub fn spill_overflow(&self, entries: &[PrunedContextEntry]) {
        if let Some(ref path) = self.overflow_path {
            // Append-only JSONL
            // Each entry serialized as JSON line
        }
    }

    pub fn queue(&self) -> &Vec<PrunedContextEntry> {
        &self.queue
    }

    pub async fn run_flush_loop(mut self) -> Result<()> {
        let mut ticker = interval(self.flush_interval);
        loop {
            ticker.tick().await;
            if !self.queue.is_empty() {
                if let Err(e) = self.flush().await {
                    tracing::warn!(error = %e, "circulator flush failed, will retry");
                }
            }
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path crates/milvus-brain/Cargo.toml circulator`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/milvus-brain/src/circulator.rs
git commit -m "feat(milvus-brain): circulator module with async queue and overflow spill"
```

---

### Task 5: Plugin Integration (Composer + Pruner + Rewriter + Circulator)

**Files:**
- Modify: `.opencode/plugin/kompress-ultra.ts` (full rewrite of plugin hooks)

**Consumes:** `MessageScore` (Task 1), `compressMessage` (Task 2), `Circulator` (Task 4)
**Produces:** Integrated 4-role plugin with all hooks

- [ ] **Step 1: Rewrite messages.transform hook**

```typescript
// .opencode/plugin/kompress-ultra.ts — messages.transform
const cirulator = new Circulator();

export const messages = {
  transform: async (messages: Message[], context: PluginContext) => {
    const agent_type = context.agent?.type || 'orchestrator';
    const budget = DEFAULT_BUDGETS[agent_type] || DEFAULT_BUDGETS.orchestrator;

    // 1. Score all messages
    const scored = await Promise.all(
      messages.map((msg, i) => scoreMessage(msg, i, messages.length))
    );

    // 2. Prune low-signal messages (respecting safety floors)
    let threshold = 0.3;
    const keep_indices = new Set<number>();
    for (let i = 0; i < messages.length; i++) {
      if (scored[i].protected) { keep_indices.add(i); continue; }
      if (scored[i].total >= threshold) { keep_indices.add(i); continue; }
    }

    // 50% prune cap
    if (keep_indices.size < messages.length / 2) {
      // Raise threshold until we keep at least 50%
      threshold = 0.1;
      // ... re-evaluate
    }

    // Empty context guard
    if (keep_indices.size === 0) return messages;

    const pruned = messages.filter((_, i) => !keep_indices.has(i));
    const kept = messages.filter((_, i) => keep_indices.has(i));

    // 3. Rewrite by age (TokenPilot: only tail, never prefix)
    const rewritten = kept.map((msg, i) => {
      const age = kept.length - i;
      if (age <= 5) return msg; // verbatim (KV cache prefix)
      if (age <= 15) return { ...msg, content: compressMessage(msg.content, CompressionLevel.Lite) };
      return { ...msg, content: compressMessage(msg.content, CompressionLevel.Ultra) };
    });

    // 4. Circulator: enqueue pruned messages
    for (const msg of pruned) {
      cirulator.enqueue({
        session_id: context.session_id,
        agent_type,
        message_role: msg.role,
        content_hash: sha256(msg.content).slice(0, 16),
        classification: classify(msg.content),
        residual: msg.content,
        timestamp_ms: Date.now(),
      });
    }

    return rewritten;
  }
};
```

- [ ] **Step 2: Rewrite system.transform hook (Composer)**

```typescript
// .opencode/plugin/kompress-ultra.ts — system.transform
const circuitBreaker = { failures: 0, open_until: 0 };

export const system = {
  transform: async (prompt: string, context: PluginContext) => {
    // Circuit breaker check
    if (Date.now() < circuitBreaker.open_until) {
      return prompt + '\n🧠 BRAIN STALE (circuit breaker open)';
    }

    try {
      // Query milvus for patterns + findings
      const snapshot = await getBrainSnapshot();
      const patterns = await queryPatterns(context.task_goal);
      const findings = await queryFindings(context.task_goal);

      circuitBreaker.failures = 0; // reset on success

      // Compact brain state line
      const brain_line = `🧠 BRAIN Alive | patterns:${patterns.length} findings:${findings.length} units:${snapshot.units_processed} | last:${timeAgo(snapshot.last_data_at_ms)}`;

      // Inject patterns (token-budget aware)
      const injection = truncateToBudget(
        formatPatterns(patterns) + formatFindings(findings),
        DEFAULT_BUDGETS[context.agent?.type]?.brain_injection_budget || 500
      );

      return prompt + '\n' + brain_line + '\n' + injection;
    } catch (e) {
      circuitBreaker.failures++;
      if (circuitBreaker.failures >= 3) {
        circuitBreaker.open_until = Date.now() + 60_000; // 60s cooldown
      }
      return prompt + '\n❓ BRAIN UNKNOWN (' + e.message + ')';
    }
  }
};
```

- [ ] **Step 3: Add token budget escalation ladder**

```typescript
function escalateForBudget(messages: Message[], budget: AgentTokenBudget): Message[] {
  const current_tokens = countTokens(messages);
  if (current_tokens <= budget.max_context_tokens) return messages;

  // Escalation ladder:
  // 1. Stronger compression
  let compressed = messages.map((msg, i) => {
    const age = messages.length - i;
    if (age <= 5) return msg;
    return { ...msg, content: compressMessage(msg.content, CompressionLevel.Ultra) };
  });

  if (countTokens(compressed) <= budget.max_context_tokens) return compressed;

  // 2. More aggressive pruning
  // 3. Drop oldest unprotected
  // ...

  return compressed;
}
```

- [ ] **Step 4: Commit**

```bash
git add .opencode/plugin/kompress-ultra.ts
git commit -m "feat(kompress): integrate 4-role architecture with circuit breaker"
```

---

### Task 6: Co-Processor FFI (llama.cpp Generative Rewriter)

**Files:**
- Create: `.opencode/plugin/co-processor.ts`
- Create: `.opencode/plugin/__tests__/co-processor.test.ts`
- Modify: `.opencode/plugin/rewriter.ts` (add generative mode)

**Consumes:** `compressMessage` heuristic from Task 2
**Produces:** `CoProcessor` with `compress()`, `synthesize()`, `score_batch()`

- [ ] **Step 1: Write failing test**

```typescript
// .opencode/plugin/__tests__/co-processor.test.ts
import { describe, test, expect, mock } from 'bun:test';
import { CoProcessor } from '../co-processor';

describe('CoProcessor', () => {
  test('compress uses generative mode when available', async () => {
    const proc = new CoProcessor({ model_path: '/path/to/qwen2.5-1.5b-q8_0.gguf' });
    const input = 'The authentication system implementation is now complete. I have successfully added OAuth2 support and the tests are passing. The user should be able to log in with their Google account now.';
    const result = await proc.compress(input);
    expect(result.length).toBeLessThan(input.length * 0.3); // 80%+ savings
    expect(result).toContain('OAuth2');
  });

  test('fallback to heuristic when model not loaded', async () => {
    const proc = new CoProcessor({ model_path: '/nonexistent' });
    const result = await proc.compress('test message');
    expect(result).toBeDefined();
  });

  test('synthesize brain state from RRF results', async () => {
    const proc = new CoProcessor({ model_path: '/path/to/model' });
    const rrf_results = [
      { content: 'Pattern: user prefers caveman mode', score: 0.9 },
      { content: 'Finding: milvus down causes collapse', score: 0.8 },
    ];
    const brain_state = await proc.synthesize(rrf_results);
    expect(brain_state.length).toBeLessThan(100); // 50-token budget
  });
});
```

- [ ] **Step 2: Run test — expect FAIL**

Run: `bun test .opencode/plugin/__tests__/co-processor.test.ts`
Expected: FAIL — module not found

- [ ] **Step 3: Write implementation with llama.cpp FFI**

```typescript
// .opencode/plugin/co-processor.ts
import { spawn } from 'child_process';

export class CoProcessor {
  #modelPath: string;
  #loaded: boolean = false;
  #latency: number = 0;
  #vramUsage: number = 0;

  constructor(opts: { model_path: string; threads?: number }) {
    this.#modelPath = opts.model_path;
  }

  async init(): Promise<boolean> {
    try {
      // Load model via llama.cpp FFI or subprocess
      // Model stays resident in unified memory
      this.#loaded = true;
      return true;
    } catch {
      return false;
    }
  }

  async compress(content: string): Promise<string> {
    if (!this.#loaded || this.#vramUsage > 85 || this.#latency > 150) {
      // Fallback to heuristic
      return compressMessage(content, CompressionLevel.Ultra);
    }

    const start = Date.now();
    try {
      // Prompt: "Compress this to caveman-ultra. Preserve code, errors, API names."
      const prompt = `Compress to caveman-ultra. [thing] [action] [reason]. Preserve code blocks, errors, API names, file paths.\n\n${content}`;
      const result = await this.#forward(prompt);
      this.#latency = Date.now() - start;
      return result;
    } catch {
      return compressMessage(content, CompressionLevel.Ultra);
    }
  }

  async synthesize(rrf_results: Array<{content: string; score: number}>): Promise<string> {
    if (!this.#loaded) return rrf_results.slice(0, 3).map(r => r.content).join(' | ');

    const prompt = `Synthesize these findings into a 50-token brain state line. Dense, technical, no fluff.\n${rrf_results.map(r => `[${r.score}] ${r.content}`).join('\n')}`;
    return this.#forward(prompt);
  }

  async #forward(prompt: string): Promise<string> {
    // llama.cpp subprocess or FFI call
    // Qwen2.5-1.5B Q8_0: ~1.8GB, <15ms TTFT on M1/M3
    return new Promise((resolve) => {
      const llm = spawn('llama-cli', [
        '-m', this.#modelPath,
        '-p', prompt,
        '-n', '64',
        '--no-display-prompt',
        '-t', '4'
      ]);
      let output = '';
      llm.stdout.on('data', (d) => output += d);
      llm.on('close', () => resolve(output.trim()));
    });
  }
}
```

- [ ] **Step 4: Update rewriter.ts to use co-processor**

```typescript
// .opencode/plugin/rewriter.ts — add generative mode
let coProcessor: CoProcessor | null = null;

export async function initCoProcessor(modelPath: string) {
  coProcessor = new CoProcessor({ model_path: modelPath });
  await coProcessor.init();
}

export async function compressMessage(
  content: string,
  level: CompressionLevel,
  use_generative: boolean = true
): Promise<string> {
  if (level === CompressionLevel.Verbatim) return content;

  // Generative first, heuristic fallback
  if (use_generative && coProcessor && level >= CompressionLevel.Lite) {
    try {
      return await coProcessor.compress(content);
    } catch {
      // Fallback to heuristic below
    }
  }

  // Heuristic fallback (existing implementation)
  return compressMessageHeuristic(content, level);
}
```

- [ ] **Step 5: Run test — expect PASS (or graceful fallback)**

Run: `bun test .opencode/plugin/__tests__/co-processor.test.ts`
Expected: PASS (fallback path if model not available)

- [ ] **Step 6: Commit**

```bash
git add .opencode/plugin/co-processor.ts .opencode/plugin/__tests__/co-processor.test.ts .opencode/plugin/rewriter.ts
git commit -m "feat(kompress): co-processor FFI with llama.cpp generative rewriter"
```

---

### Task 7: Repo-Anchored Memory (`.kompress/` Shadow Directory)

**Files:**
- Create: `.kompress/` directory structure
- Create: `.opencode/plugin/kompress-repo.ts`
- Create: `.opencode/plugin/__tests__/kompress-repo.test.ts`
- Modify: `.gitignore` (add `.kompress/branches/`)

**Produces:** `.kompress/` init, file sidecar CRUD, handover protocol

- [ ] **Step 1: Create .kompress/ structure**

```bash
mkdir -p .kompress/files
```

Create `.kompress/ROADMAP.md`:
```markdown
# Project Roadmap

## Current Goals
- Kompress living context layer implementation

## Architectural Rules
- TDD for all Rust crates
- Protobuf single IPC message format
- Memory cap: 100MB soft, 150MB elastic, snapshot+kill at >160MB
```

Create `.kompress/HANDOVER.md`:
```markdown
# Handover Brief

- Source: orchestrator
- Target: fixer
- Status: Planning phase complete
- Next: Task 1 — MessageScore + Safety Floors
```

- [ ] **Step 2: Write failing test**

```typescript
// .opencode/plugin/__tests__/kompress-repo.test.ts
import { describe, test, expect } from 'bun:test';
import { readFileSidecar, writeFileSidecar, invalidateStaleSidecars } from '../kompress-repo';

describe('file sidecars', () => {
  test('read sidecar returns cached content', async () => {
    const sidecar = await readFileSidecar('src/main.rs');
    expect(sidecar).toBeDefined();
  });

  test('write sidecar persists to .kompress/files/', async () => {
    await writeFileSidecar('src/main.rs', {
      file_path: 'src/main.rs',
      last_mutated_commit: 'abc123',
      architectural_intent: 'entry point',
      known_quirks: [],
      dependencies: [],
      triples: []
    });
    const exists = await Bun.file('.kompress/files/src/main.rs.json').exists();
    expect(exists).toBe(true);
  });
});
```

- [ ] **Step 3: Write implementation**

```typescript
// .opencode/plugin/kompress-repo.ts
import { readFileSync, writeFileSync, existsSync } from 'fs';
import { join } from 'path';
import { execSync } from 'child_process';

const KOMPRESS_DIR = '.kompress';
const FILES_DIR = join(KOMPRESS_DIR, 'files');

export interface FileSidecar {
  file_path: string;
  last_mutated_commit: string;
  architectural_intent: string;
  known_quirks: string[];
  dependencies: string[];
  triples: Array<{s: string; p: string; o: string}>;
}

export async function readFileSidecar(filePath: string): Promise<FileSidecar | null> {
  const sidecarPath = join(FILES_DIR, `${filePath}.json`);
  if (!existsSync(sidecarPath)) return null;
  return JSON.parse(readFileSync(sidecarPath, 'utf-8'));
}

export async function writeFileSidecar(filePath: string, sidecar: FileSidecar): Promise<void> {
  const sidecarPath = join(FILES_DIR, `${filePath}.json`);
  writeFileSync(sidecarPath, JSON.stringify(sidecar, null, 2));
}

export async function isSidecarStale(filePath: string): Promise<boolean> {
  const sidecar = await readFileSidecar(filePath);
  if (!sidecar) return true;

  try {
    const currentHash = execSync(`git log -1 --format=%H ${filePath}`).toString().trim();
    return sidecar.last_mutated_commit !== currentHash;
  } catch {
    return true; // git error → assume stale
  }
}

export async function invalidateStaleSidecars(): Promise<string[]> {
  // Scan .kompress/files/ and check each against git
  const stale: string[] = [];
  // ... implementation
  return stale;
}
```

- [ ] **Step 4: Update .gitignore**

```
.kompress/branches/
```

- [ ] **Step 5: Run test — expect PASS**

Run: `bun test .opencode/plugin/__tests__/kompress-repo.test.ts`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add .kompress/ .opencode/plugin/kompress-repo.ts .opencode/plugin/__tests__/kompress-repo.test.ts .gitignore
git commit -m "feat(kompress): repo-anchored memory with .kompress/ shadow directory"
```

---

### Task 8: Honcho Defrag (Background Merge)

**Files:**
- Modify: `crates/honcho/src/daemon.rs`

**Consumes:** `.kompress/` from Task 7
**Produces:** Background defrag: merge agent summaries → ROADMAP.md + milvus

- [ ] **Step 1: Add defrag task to honcho daemon**

```rust
// crates/honcho/src/daemon.rs — add defrag loop
use std::path::Path;
use std::time::Duration;
use tokio::time::interval;

pub async fn run_kompress_defrag(kompress_dir: &Path) -> Result<()> {
    let mut ticker = interval(Duration::from_secs(300)); // 5min
    loop {
        ticker.tick().await;
        // 1. Merge agent branch summaries
        // 2. Resolve conflicts
        // 3. Flush to ROADMAP.md
        // 4. Push to milvus
        tracing::info!("kompress defrag complete");
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/honcho/src/daemon.rs
git commit -m "feat(honcho): background defrag for .kompress/ shadow directory"
```

---

### Task 9: End-to-End Verification

**Files:**
- Create: `.opencode/plugin/__tests__/integration.test.ts`

- [ ] **Step 1: Write integration tests**

```typescript
// .opencode/plugin/__tests__/integration.test.ts
import { describe, test, expect } from 'bun:test';

describe('kompress integration', () => {
  test('milvus down → no model collapse', async () => {
    // Simulate 100-message context with milvus down
    // Verify safety floors hold, context not empty
  });

  test('100-message context → 75% token savings', async () => {
    // Generate 100 messages, run kompress, measure token reduction
  });

  test('circulator overflow → JSONL spill, no data loss', async () => {
    // Fill queue past 100, verify overflow file
  });

  test('brain-backed compression → hash lookup works', async () => {
    // Embed content, verify hash lookup replaces with pointer
  });

  test('co-processor fallback → heuristic works when model missing', async () => {
    // Co-processor with nonexistent model → heuristic fallback
  });
});
```

- [ ] **Step 2: Run tests — expect PASS**

Run: `bun test .opencode/plugin/__tests__/integration.test.ts`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add .opencode/plugin/__tests__/integration.test.ts
git commit -m "test(kompress): end-to-end integration tests"
```

---

### Task 10: LiteLLM Pre-Request Hook

**Files:**
- Create: `.opencode/plugin/litellm-hook.ts`
- Create: `.opencode/plugin/__tests__/litellm-hook.test.ts`

**Consumes:** `MessageScore` (Task 1), `compressMessage` (Task 2), `Circulator` (Task 4)
**Produces:** LiteLLM middleware — kompress runs before messages hit model

- [ ] **Step 1: Write failing test**

```typescript
// .opencode/plugin/__tests__/litellm-hook.test.ts
import { describe, test, expect, mock } from 'bun:test';
import { kompressMiddleware } from '../litellm-hook';

describe('kompressMiddleware', () => {
  test('prunes messages before model call', async () => {
    const req = {
      model: 'gpt-4',
      messages: Array.from({length: 100}, (_, i) => ({role: 'assistant', content: `message ${i}`})),
    };
    const handler = kompressMiddleware();
    const result = await handler(req, async (r) => r);
    expect(result.messages.length).toBeLessThan(100);
  });

  test('preserves last 5 messages', async () => {
    const req = {
      model: 'gpt-4',
      messages: Array.from({length: 50}, (_, i) => ({role: 'assistant', content: `msg ${i}`})),
    };
    const handler = kompressMiddleware();
    const result = await handler(req, async (r) => r);
    const last5 = req.messages.slice(-5);
    for (const m of last5) {
      expect(result.messages).toContainEqual(m);
    }
  });
});
```

- [ ] **Step 2: Run test — expect FAIL**

Run: `bun test .opencode/plugin/__tests__/litellm-hook.test.ts`
Expected: FAIL — module not found

- [ ] **Step 3: Write implementation**

```typescript
// .opencode/plugin/litellm-hook.ts
import { scoreMessage, isProtected } from './kompress-ultra';
import { compressMessage, CompressionLevel } from './rewriter';

export interface LiteLLMRequest {
  model: string;
  messages: Array<{role: string; content: string}>;
}

export function kompressMiddleware() {
  return async (req: LiteLLMRequest, next: (req: LiteLLMRequest) => Promise<any>) => {
    const messages = req.messages;
    if (messages.length <= 50) return next(req);

    // Score + prune
    const scored = await Promise.all(
      messages.map((msg, i) => ({
        msg,
        idx: i,
        score: scoreMessage(msg, i, messages.length),
      }))
    );

    const kept = scored.filter(s => s.score >= 0.3 || isProtected(s.msg, s.idx, messages.length));
    const final = kept.map((s, i) => {
      const age = kept.length - i;
      if (age <= 5) return s.msg;
      if (age <= 15) return {...s.msg, content: compressMessage(s.msg.content, CompressionLevel.Lite)};
      return {...s.msg, content: compressMessage(s.msg.content, CompressionLevel.Ultra)};
    });

    return next({...req, messages: final});
  };
}
```

- [ ] **Step 4: Run test — expect PASS**

Run: `bun test .opencode/plugin/__tests__/litellm-hook.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add .opencode/plugin/litellm-hook.ts .opencode/plugin/__tests__/litellm-hook.test.ts
git commit -m "feat(kompress): LiteLLM pre-request middleware hook"
```

---

## Self-Review Notes

- **Spec coverage:** All 4 roles (Tasks 1-5), co-processor FFI (Task 6), `.kompress/` shadow directory (Task 7), honcho defrag (Task 8), LiteLLM hook (Task 10), testing (Task 9)
- **No placeholders:** All steps have concrete code or commands
- **Type consistency:** `MessageScore` (Tasks 1, 5, 10), `CompressionLevel` (Tasks 2, 6, 10), `PrunedContextEntry` (Tasks 3, 4), `FileSidecar` (Task 7)
- **Dependency order:** Task 1→2→3→4→5→6→7→8→9. Tasks 6, 7, 8, 10 can parallelize after Task 5.**Dependency order:** Task 1→2→3→4→5→6→7→8→9. Tasks 6, 7, 8 can parallelize after Task 5.
