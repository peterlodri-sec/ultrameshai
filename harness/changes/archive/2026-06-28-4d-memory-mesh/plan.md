# Implementation Plan

## Step 1 — Supabase Migrations
Create `supabase/migrations/`:
- `001_memories.sql` — memories table + indexes
- `002_connectors.sql` — connectors table + indexes
- `003_a2a_exchanges.sql` — A2A logging table
- `004_agent_rewards.sql` — rewards + motivations tables

## Step 2 — memory-mesh Crate
Create `crates/memory-mesh/src/`:
- `lib.rs` — core types (Memory, Connector, A2AExchange, AgentMotivation)
- `connectors.rs` — 4 connector types, graph traversal
- `recall.rs` — stochastic walk, deja vu trigger
- `a2a.rs` — A2A client, logging
- `rewards.rs` — reward calculation, tier promotion

## Step 3 — Wire Memory Loops
Update:
- `memory-indexer` — write to memories table + milvus
- `memory-pattern-miner` — query connectors + recall
- `memory-summarizer` — compress recalled context

## Step 4 — Enforce A2A on All Loops
Add to base Loop trait or wrapper:
- `make_a2a_call()` method
- Log exchange to Supabase
- Fail gracefully if A2A fails (log warning, continue)

## Step 5 — Add Motivation to LoopInput
Extend LoopInput:
- `motivation: Option<AgentMotivation>`
- Track reward on LoopOutput

## Step 6 — Verify
- cargo check, cargo test
- Run loop with A2A logging
- Verify rewards table updates
- ECL lint and archive
