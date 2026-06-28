# Requirements

## Functional

### 1. 4D Memory Schema (Supabase)
- `memories` table: id, embedding (pgvector), content, loop_type, encoded_at, accessed_at[], access_count, decay_rate, strength
- `connectors` table: from_id, to_id, connector_type (similarity/temporal/causal/reinforcement), weight, created_at
- Indexes on embedding (ivfflat), connectors (from_id, to_id, connector_type)

### 2. Inter-Connectors
- **Similarity**: Semantic proximity (embedding distance)
- **Temporal**: Sequential patterns (encoded_at ordering)
- **Causal**: A triggered B (explicit dependency)
- **Reinforcement**: Access count strengthens edge

### 3. Random Recall + Deja Vu
- Stochastic graph walk from query node
- Confidence threshold = "I've seen this before"
- Returns: related memories + confidence scores

### 4. Mandatory A2A Calls
- Every loop.process() must make ≥1 A2A call
- Log request/response to `a2a_exchanges` table
- Schema: from_node, to_node, from_loop, to_loop, request, response, latency_ms, success

### 5. Motivation/Reward System
- `agent_rewards` table: agent_id, loop_type, task_id, reward_type, reward_amount, motivation_score, success
- `agent_motivations` table: total_points, completion_rate, a2a_count, motivation_tier (bronze/silver/gold/platinum)
- AgentMotivation struct: purpose, reward_type, reward_amount, intrinsic_drive, social_pressure

## Non-functional
- Supabase connection via existing infra (devcx53)
- milvus-brain integration for vector search
- A2A calls are async, non-blocking
- Reward calculation is deterministic

## Out of scope
- UI dashboard for motivation tiers
- Cross-node federation (future)
- Decay algorithms (future)
