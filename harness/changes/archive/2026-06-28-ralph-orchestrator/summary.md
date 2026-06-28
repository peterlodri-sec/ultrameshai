# Ralph-Style Orchestrator

## What
Build an orchestrator loop that implements the Ralph pattern: externalize state to `prd.json` + `progress.txt`, spawn fresh loop instances per story, run quality gates, update state on success.

## Why
Current loops are stateless compute units but nothing orchestrates them with durable state. Ralph pattern enables unlimited horizontal scale — each iteration is a fresh burn-down, state persists externally via git + JSON + milvus-brain.

## How
1. Create `orchestrator/ralph.rs` — reads `prd.json`, picks incomplete story, spawns loop, runs checks
2. Define `prd.json` schema — user stories with `id`, `title`, `passes`, `loop_type`, `acceptance_criteria`
3. Quality gate hooks — typecheck, cargo test, custom checks per story
4. State updates — commit on success, append to `progress.txt`, mark story `passes: true`

## Verification
- Orchestrator runs 3+ stories sequentially
- Quality gates reject bad output
- `prd.json` updates correctly
- `progress.txt` accumulates learnings
