# Implementation Plan

## Step 1 — Define prd.json schema
Create `crates/orchestrator/src/prd.rs`:
- `UserStory` struct with serde Serialize/Deserialize
- `Prd` struct with `userStories`, `branchName`, `featureName`
- Example `prd.json.example` in project root

## Step 2 — Create orchestrator struct
`crates/orchestrator/src/ralph.rs`:
- `RalphOrchestrator` with config (max_iterations, quality_gates)
- `load_prd()` — read from file
- `pick_next_story()` — highest priority incomplete
- `run_story()` — spawn loop, execute, collect output
- `run_quality_gates()` — cargo check, cargo test, custom
- `commit_success()` — git add, git commit, update prd.json
- `append_progress()` — write to progress.txt

## Step 3 — Wire up loop spawning
- Use existing `ModelRouter` to get loop_type → model mapping
- Instantiate loop via match on loop_type string
- Call `loop.process()` with story as input
- Parse output for success/failure

## Step 4 — Git integration
- Use `git2` crate or shell out to `git` command
- Create branch from `branchName` in prd.json
- Commit on each passing story
- Handle merge conflicts (fail gracefully)

## Step 5 — CLI entry point
`crates/orchestrator/src/main.rs` or `scripts/ralph.sh`:
- Parse args: `--max-iterations N`, `--prd path/to/prd.json`
- Run orchestrator loop
- Output status after each iteration

## Step 6 — Test
- Create test `prd.json` with 3-5 stories
- Run orchestrator, verify all pass
- Verify `progress.txt` has learnings
- Verify git history has commits

## Step 7 — ECL archive
- `nu scripts/lint-ecl.nu`
- Move to `harness/changes/archive/`
