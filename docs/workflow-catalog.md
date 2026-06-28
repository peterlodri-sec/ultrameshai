# Workflow Catalog

**Purpose:** Document validated end-to-end workflows that combine multiple loops.

---

## Validated Workflows

### 1. Original 10 E2E

**ID:** `original-10-e2e`

**Description:** Run all 10 original loops with real LLM integration, verifying each calls the model and returns a response.

**Loops involved:**
- `deepwork`, `bruteforce-coder`, `deep-research`, `testers`, `yardmaster`, `devops`, `ui`, `red-team`, `juniors`, `ralph`

**Success criteria:**
- All 10 loops compile without errors
- All loop type tests pass (correct `loop_type()` strings)
- All 157+ workspace tests pass
- ECL change archived: `2026-06-28-original10-e2e`

**ECL reference:** `harness/changes/archive/2026-06-28-original10-e2e/`

---

### 2. Models TOML Extraction

**ID:** `models-toml-config`

**Description:** Extract model tier aliases and loop mappings from hardcoded Rust to `crates/cognition/config/models.toml`.

**Loops involved:** All 48 loops (via `ModelRouter`)

**Success criteria:**
- `config/models.toml` contains all 11 tiers + 48 loop mappings
- `ModelRouter::new()` loads from TOML at compile time
- All tests pass (no behavior change)
- ECL change archived: `2026-06-28-models-toml-extraction`

**ECL reference:** `harness/changes/archive/2026-06-28-models-toml-extraction/`

---

### 3. Loop Logic Implementation

**ID:** `loop-logic-implementation`

**Description:** Upgrade 38 stub loops to full LLM-integrated implementations following the `deepwork.rs` pattern.

**Loops involved:** 38 new loops (SWE-bench, testing, devops, quality, memory, ralph-expanded, infra, utility, math, research)

**Success criteria:**
- All 38 loops have `LlmClient`, `Session`, `PromptDispatcher`
- All dispatch prompt templates correctly
- `cargo test` passes (157+ tests)
- ECL change archived: `2026-06-28-loop-logic-implementation`

**ECL reference:** `harness/changes/archive/2026-06-28-loop-logic-implementation/`

---

### 4. Ralph Orchestrator

**ID:** `ralph-orchestrator`

**Description:** Build orchestrator that implements Ralph pattern: externalize state to `prd.json` + `progress.txt`, spawn fresh loop instances per story, run quality gates, update state on success.

**Loops involved:** Orchestrator spawns loops based on `prd.json` story `loop_type`

**Success criteria:**
- `crates/orchestrator/src/prd.rs` — UserStory, Prd structs
- `crates/orchestrator/src/ralph.rs` — RalphOrchestrator with quality gates
- `prd.json.example` — 5-story example PRD
- `cargo check` and `cargo test` pass
- ECL change archived: `2026-06-28-ralph-orchestrator`

**ECL reference:** `harness/changes/archive/2026-06-28-ralph-orchestrator/`

---

## Guided Workflows

### SWE-Bench Pipeline (In Progress)

**ID:** `swe-bench-pipeline`

**Description:** Full SWE-bench workflow: analyze issue → explore codebase → plan fix → implement → test → validate.

**Loops involved:** `issue-analyzer`, `codebase-explorer`, `fix-planner`, `fix-implementer`, `tester-unit`, `regression-checker`, `diff-builder`

**Status:** Guided — loops implemented, ECL pending

---

### Testing Suite (In Progress)

**ID:** `testing-suite`

**Description:** Comprehensive testing: unit → integration → benchmark → property → mutation.

**Loops involved:** `tester-unit`, `tester-integration`, `tester-benchmark`, `tester-property`, `tester-mutation`

**Status:** Guided — loops implemented, ECL pending

---

## Planned Workflows

### DevOps Pipeline

**ID:** `devops-pipeline`

**Description:** Build → package → deploy → cache management.

**Loops involved:** `devops-build`, `devops-package`, `devops-deploy`, `devops-cache`

**Status:** Planned — skeleton only

---

### Quality Gates

**ID:** `quality-gates`

**Description:** Lint → audit → coverage → typecheck → style enforcement.

**Loops involved:** `quality-lint`, `quality-audit`, `quality-coverage`, `quality-typecheck`, `quality-style`

**Status:** Planned — skeleton only

---

### Memory-Driven Development

**ID:** `memory-driven-dev`

**Description:** Index findings → mine patterns → summarize for context injection.

**Loops involved:** `memory-indexer`, `memory-pattern-miner`, `memory-summarizer`

**Status:** Planned — skeleton only

---

## Adding a Workflow

To add a validated workflow:

1. Create ECL change documenting the workflow
2. Run workflow end-to-end successfully
3. Archive ECL change
4. Add entry to this catalog with:
   - Unique ID
   - Description
   - Loops involved
   - Success criteria
   - ECL reference

---

**Source of truth:** `harness/changes/archive/` (ECL changes)
