# Requirements

## Functional
1. `prd.json` schema with user stories:
   - `id`: string (e.g., "US-001")
   - `title`: string
   - `description`: string
   - `loop_type`: string (which loop to spawn)
   - `acceptance_criteria`: array of strings
   - `passes`: boolean
   - `branchName`: string (git branch for this story)

2. Orchestrator must:
   - Load `prd.json` from project root
   - Pick highest-priority story where `passes == false`
   - Spawn appropriate loop type with fresh context
   - Run quality gates (configurable per story)
   - On success: commit, update `prd.json`, append to `progress.txt`
   - On failure: log error, mark story as failed, continue or stop

3. Quality gates:
   - `cargo check` — always run
   - `cargo test` — always run
   - Custom commands per story (e.g., `cargo clippy`, `npm run lint`)

4. State persistence:
   - `prd.json` — updated after each story
   - `progress.txt` — append-only learnings
   - Git commits — one per passing story

## Non-functional
1. Each loop invocation = fresh context (no accumulated state)
2. Orchestrator itself is stateless — reads from durable storage
3. Failure isolation — one failing story doesn't corrupt state
4. Configurable max iterations (default: 10)

## Out of scope
- Browser verification for UI stories (future)
- Parallel story execution (future — dependencies first)
- Auto-handoff on context full (future)
