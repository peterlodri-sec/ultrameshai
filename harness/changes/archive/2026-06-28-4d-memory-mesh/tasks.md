# Tasks

## Supabase Migrations
- [ ] Create `supabase/migrations/001_memories.sql`
- [ ] Create `supabase/migrations/002_connectors.sql`
- [ ] Create `supabase/migrations/003_a2a_exchanges.sql`
- [ ] Create `supabase/migrations/004_agent_rewards.sql`

## memory-mesh Crate
- [ ] Create `crates/memory-mesh/Cargo.toml`
- [ ] Create `crates/memory-mesh/src/lib.rs` — core types
- [ ] Create `crates/memory-mesh/src/connectors.rs` — 4 connector types
- [ ] Create `crates/memory-mesh/src/recall.rs` — stochastic walk + deja vu
- [ ] Create `crates/memory-mesh/src/a2a.rs` — A2A client + logging
- [ ] Create `crates/memory-mesh/src/rewards.rs` — reward calculation

## Wire Memory Loops
- [ ] Update `memory-indexer` — write to Supabase + milvus
- [ ] Update `memory-pattern-miner` — query connectors + recall
- [ ] Update `memory-summarizer` — compress recalled context

## Enforce A2A
- [ ] Add `make_a2a_call()` to Loop trait or wrapper
- [ ] Update all 48 loops to call A2A in process()
- [ ] Log A2A exchanges to Supabase

## Motivation System
- [ ] Extend LoopInput with `motivation: Option<AgentMotivation>`
- [ ] Track reward in LoopOutput
- [ ] Update agent_motivations table on completion

## Verify
- [ ] cargo check, cargo test
- [ ] Run loop with A2A logging
- [ ] Verify rewards table updates
- [ ] ECL lint and archive
