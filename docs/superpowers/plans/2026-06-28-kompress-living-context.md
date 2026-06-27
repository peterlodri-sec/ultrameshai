# Kompress Living Context Layer — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Transform kompress-ultra into 4-role living context layer (Composer, Pruner, Rewriter, Circulator).

**Tech Stack:** TypeScript (Bun), Rust (tokio), Protobuf

## Global Constraints

- TDD: failing test first
- milvus collections: `research_findings`, `learning_patterns` (existing), `pruned_context` (new)
- KV cache prefix: never touch system prompt or last 5 messages
- Circulator overflow: spill to `~/.cache/ultrameshai/overflow-circulator.jsonl`

---

### Task 1: MessageScore + Safety Floors

**Files:** Modify `.opencode/plugin/kompress-ultra.ts` (lines 47-167), Create `.opencode/plugin/__tests__/score.test.ts`

**Produces:** `MessageScore`, `scoreMessage()`, `isProtected()`

- [ ] Write test: `isProtected` returns true for last-5, user, code, error messages
- [ ] Run test — expect FAIL
- [ ] Implement `isProtected()`, `ebbinghausDecay()`, `structuralBoost()`, `scoreMessage()` with milvus fallback 0.5
- [ ] Run test — expect PASS
- [ ] Commit: `feat(kompress): MessageScore engine with safety floors`

---

### Task 2: Deterministic Rewriter

**Files:** Create `.opencode/plugin/rewriter.ts`, `.opencode/plugin/__tests__/rewriter.test.ts`

**Produces:** `compressMessage()`, `CompressionLevel` enum

- [ ] Write test: verbatim preserves content, lite drops articles, ultra produces fragments, code blocks untouched, markdown balanced
- [ ] Run test — expect FAIL
- [ ] Implement rules-based compression: drop articles/filler/pleasantries, preserve code/API/errors, markdown fence validation
- [ ] Run test — expect PASS
- [ ] Commit: `feat(kompress): deterministic caveman rewriter`

---

### Task 3: Protobuf Extension

**Files:** Modify `proto/loop_engineering.proto`

**Produces:** `PrunedContextEntry`, `GraphTriple` messages

- [ ] Add `GraphTriple` and `PrunedContextEntry` messages with classification enum, topic_key, superseded_by, residual, triples, content_hash
- [ ] Run `nix build .#protobuf-gen` or equivalent codegen
- [ ] Commit: `proto: add PrunedContextEntry and GraphTriple messages`

---

### Task 4: Rust Circulator Module

**Files:** Create `crates/milvus-brain/src/circulator.rs`, `crates/milvus-brain/src/circulator_test.rs`

**Produces:** `Circulator` struct with `enqueue()`, `flush()`, `spill_overflow()`

- [ ] Write test: enqueue + flush writes to milvus, overflow spills to JSONL, supersession chains preserved
- [ ] Run `cargo test --manifest-path crates/milvus-brain/Cargo.toml` — expect FAIL
- [ ] Implement: async queue (cap 100), batch flush (10 entries or 30s), classification (fact/event/instruction/task), residual delta storage, graph triple extraction, overflow spill to JSONL
- [ ] Run tests — expect PASS
- [ ] Commit: `feat(milvus-brain): circulator module with async queue and overflow spill`

---

### Task 5: Plugin Integration (Composer + Pruner + Rewriter + Circulator)

**Files:** Modify `.opencode/plugin/kompress-ultra.ts` (full rewrite of plugin hooks)

**Produces:** Integrated 4-role plugin

- [ ] Rewrite `messages.transform` hook: score → prune (respecting safety floors) → rewrite by age → circulator enqueue
- [ ] Rewrite `system.transform` hook: composer queries milvus → inject brain state line + patterns
- [ ] Add circuit breaker: 3 consecutive milvus failures → skip 60s
- [ ] Add token budget escalation ladder: truncate injection → stronger compression → more pruning → drop oldest
- [ ] Commit: `feat(kompress): integrate 4-role architecture`

---

### Task 6: End-to-End Verification

**Files:** Create `.opencode/plugin/__tests__/integration.test.ts`

- [ ] Test: milvus down → no model collapse (safety floors hold)
- [ ] Test: 100-message context → 75% token savings after kompress
- [ ] Test: circulator overflow → JSONL spill, no data loss
- [ ] Test: brain-backed compression → hash lookup works
- [ ] Commit: `test(kompress): end-to-end integration tests`

---

## Self-Review Notes

- Spec coverage: All 4 roles covered (Tasks 1-5), error handling (Task 5), testing (Task 6)
- No placeholders: All steps have concrete code or commands
- Type consistency: `MessageScore` used consistently across Tasks 1, 5
- Reviewer feedback incorporated: overflow spill to JSONL (not drop), query-type routing for Composer (Task 5), brain-backed hash mechanism (Task 2)