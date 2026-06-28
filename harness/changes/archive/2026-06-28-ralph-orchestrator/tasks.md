# Tasks

## Schema & Types
- [x] Create `crates/orchestrator/src/prd.rs` — UserStory, Prd structs
- [x] Create `prd.json.example` — example PRD with 5 stories
- [x] Add `serde`, `serde_json`, `loop-engineering-loops` to orchestrator Cargo.toml

## Orchestrator Core
- [x] Create `crates/orchestrator/src/ralph.rs` — RalphOrchestrator struct
- [x] Implement `load_prd()` — read JSON from file
- [x] Implement `pick_next_story()` — filter incomplete, sort by priority
- [x] Implement `run_story()` — placeholder (TODO: spawn loop)
- [x] Implement `run_quality_gates()` — cargo check, cargo test
- [x] Implement `commit_success()` — git operations, update state
- [x] Implement `append_progress()` — write learnings

## CLI & Entry
- [x] Create `crates/orchestrator/src/main.rs` — CLI args, run loop
- [x] Update `crates/orchestrator/src/lib.rs` — export prd, ralph modules
- [x] Fix borrow checker issue in run()

## Test & Verify
- [ ] Create test prd.json with 3-5 stories
- [ ] Run orchestrator end-to-end
- [ ] Verify prd.json updates correctly
- [ ] Verify progress.txt has learnings
- [ ] cargo check passes ✅
- [ ] cargo test passes
- [ ] ECL lint and archive
