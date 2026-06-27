# Phase 2: MILVUS BRAIN — Design Spec

**Date:** 2026-06-27
**Status:** Draft (design phase)
**Target:** Unified research memory for all agents in research mode — context and milvus are one

---

## 1. System Identity & Goal

MILVUS BRAIN is the semantic memory layer for the loop-engineering agent stack. Unlike mempalace (short-term `/stats`) and honcho (long-term pattern store), milvus holds embeddings + metadata for all research findings across tasks. When an agent enters research mode (deep-research loop, junior burst, red-team research phase), it IS the BRAIN — its working context and milvus queries are unified. When it exits, findings persist in milvus, agent returns to its loop with a digest.

Phase 1 completed: cognition crate (LLM client, Session, PromptDispatcher, ModelRouter), loops crate (10 loops), agents crate (ADK pattern). Phase 2 adds milvus integration so research agents can write/read semantic memory across tasks.

### Success Criteria

1. milvus collection schema supports embeddings (1536d OVHcloud), metadata (agent_id, topic, tags), timestamps
2. Rust client crate provides async write/query/delete APIs with retry + backoff
3. LlmClient + Session can write ResearchFinding messages to milvus without leaving research mode
4. Local mock (in-memory stub) enables unit tests without running milvus server
5. Docker-compose spins up milvus + etcd + minio for local dev
6. Integration tests verify end-to-end: agent writes finding -> query returns it with similarity search

### Scope Boundary

This spec covers milvus BRAIN integration only. mempalace + honcho remain for lifecycle/state tracking (Phase 0+1). milvus is the single research-scoped consciousness — no fragmentation of research knowledge across agent-local stores.

---

## 2. Memory Hierarchy

| Store | What | Who writes | When | Access pattern |
|-------|------|-----------|------|----------------|
| **mempalace** (short-term) | Agent `/stats`: hooks (start, fan-out, dispatch, end), slice state, unit telemetry | all units | on death | key-value, per-unit |
| **honcho** (long-term) | Continuous ingest from mempalace + own observations across tasks | honcho daemon | always running | time-series, cross-task patterns |
| **milvus BRAIN** (semantic) | Unified research memory: embeddings (AST graph + code + docs + exploit patterns + CVEs), learning spike outputs | any agent in research mode + learning spikes | during research + periodically | vector similarity + metadata filter |

### Option B: milvus First (Selected)

Brainstorming concluded: build milvus BRAIN first, then wire mempalace+honcho later. Rationale:
- Research mode is the hottest path for "remembers us" effect
- Embeddings + similarity search unlock semantic merge of junior bursts
- mempalace+honcho are simpler (key-value + time-series), can be added incrementally

### Research Mode = BRAIN Identity

While in research mode, agent's Session context includes milvus client. All `research_find()` calls go to milvus (not local cache). All `write_finding()` calls persist to milvus. Agent exits research mode with a digest (summarized findings), milvus retains the full embeddings.

---

## 3. milvus Collection Schema

### Collection: `research_findings`

```
Collection: research_findings
Primary Key: finding_id (VARCHAR, max 64)
Vectors: embedding (FLOAT_VECTOR, dim 1536)
Scalar Fields:
  - agent_id: VARCHAR(32) — which agent wrote this ("deep-research", "junior-burst", "red-team-research")
  - topic: VARCHAR(256) — research topic (e.g., "tokio UDS pipelining", "CVE-2024-1234")
  - summary: VARCHAR(4096) — human-readable summary
  - tags: JSON_ARRAY — ["rust", "uds", "ipc", "security"]
  - source_url: VARCHAR(512) — optional URL (docs, CVE database, GitHub issue)
  - task_id: VARCHAR(64) — SWE-bench task ID or internal task reference
  - slice_id: VARCHAR(64) — which E2E slice triggered this research
  - created_at: INT64 — Unix timestamp (ms)
  - embedding_model: VARCHAR(32) — which model produced embedding ("ovhcloud-bge-m3", "qwen-embedding")
Indexes:
  - embedding: IVF_FLAT, nlist=1024, metric_type=COSINE
  - created_at: inverted index (for time-range queries)
  - agent_id: inverted index (for filtering by agent type)
  - tags: inverted index (for tag-based filtering)
```

### Embedding Model

OVHcloud AI endpoint: `https://oai.endpoints.kepler.ai.cloud.ovh.net/v1/chat/completions`

Use `qwen-embedding` or `bge-m3` for 1536d vectors. Embedding generation happens in the research agent's Session before writing to milvus.

### ResearchFinding Protobuf Message

From `proto/loop_engineering.proto` (Phase 0):

```protobuf
message ResearchFinding {
  string finding_id = 1;
  string source_agent = 2;     // "deep-research", "junior-burst", "red-team-research"
  string topic = 3;
  string summary = 4;
  bytes embedding = 5;         // vector embedding (1536 floats, little-endian f32)
  repeated string tags = 6;
  uint64 timestamp_ms = 7;
}
```

Protobuf -> milvus mapping:
- `finding_id` -> primary key (VARCHAR)
- `source_agent` -> `agent_id` (VARCHAR)
- `topic` -> `topic` (VARCHAR)
- `summary` -> `summary` (VARCHAR)
- `embedding` -> `embedding` (FLOAT_VECTOR, dim 1536)
- `tags` -> `tags` (JSON_ARRAY)
- `timestamp_ms` -> `created_at` (INT64)

Additional fields (`source_url`, `task_id`, `slice_id`, `embedding_model`) added in milvus schema but not in protobuf — populated by Session at write time.

---

## 4. Rust Client Crate Structure

### Crate: `milvus-brain`

```
crates/milvus-brain/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API, re-exports
│   ├── client.rs           # MilvusClient (async, connection pool)
│   ├── collection.rs       # Collection schema, create/drop
│   ├── embedding.rs        # Embedding generation (OVHcloud AI endpoint)
│   ├── query.rs            # QueryBuilder, similarity search, metadata filters
│   ├── write.rs            # Batch write, upsert, delete
│   └── error.rs            # MilvusError, Result
└── tests/
    ├── client_test.rs      # Integration tests (requires docker-compose up)
    ├── embedding_test.rs   # OVHcloud embedding generation
    └── mock_test.rs        # Unit tests with in-memory mock
```

### Public API

```rust
use milvus_brain::{MilvusClient, ResearchFinding, QueryBuilder};

// Connect to milvus (production)
let client = MilvusClient::connect("http://localhost:19530").await?;

// Create collection (idempotent)
client.ensure_collection("research_findings").await?;

// Write a finding
let finding = ResearchFinding {
    finding_id: "finding-001".into(),
    source_agent: "deep-research".into(),
    topic: "tokio UDS pipelining".into(),
    summary: "Pipelined protobuf over UDS achieves 10k msg/s".into(),
    embedding: generate_embedding("tokio UDS pipelining").await?,
    tags: vec!["rust".into(), "uds".into(), "ipc".into()],
    timestamp_ms: 1719500000000,
};
client.write_finding(finding).await?;

// Query by similarity + metadata filter
let query = QueryBuilder::new()
    .similarity("tokio async io", 5)  // top 5 similar
    .filter("agent_id = 'deep-research'")
    .filter("created_at > 1719400000000")
    .build();
let results = client.search(query).await?;

// Delete by finding_id
client.delete_finding("finding-001").await?;
```

### Internal Modules

**`client.rs`**: Connection pool (milvus SDK wrapper), retry logic with exponential backoff, timeout handling.

**`collection.rs`**: Schema definition, `ensure_collection()` (idempotent create), index creation (IVF_FLAT for embeddings, inverted for scalars).

**`embedding.rs`**: OVHcloud AI endpoint client, `generate_embedding(text) -> Vec<f32>`, batch embedding for multiple texts.

**`query.rs`**: Fluent QueryBuilder, similarity search (cosine distance), metadata filters (AND/OR), pagination.

**`write.rs`**: Batch write (up to 100 findings per batch), upsert (update if finding_id exists), soft delete (mark as deleted, purge later).

**`error.rs`**: MilvusError variants (ConnectionError, SchemaError, QueryError, EmbeddingError), retryable vs non-retryable classification.

---

## 5. Integration with Cognition Layer

### LlmClient + Session Extension

Phase 1 cognition crate provides `LlmClient` and `Session`. Phase 2 adds milvus client to Session when in research mode.

```rust
// cognition crate (Phase 1)
pub struct Session {
    session_id: String,
    loop_type: String,
    context: Vec<Message>,
    // ... Phase 1 fields
}

// Phase 2 extension: research mode adds milvus client
pub struct ResearchSession {
    inner: Session,
    milvus: MilvusClient,
    research_topic: String,
}

impl ResearchSession {
    pub async fn new(session: Session, topic: String) -> Self {
        let milvus = MilvusClient::connect("http://localhost:19530").await.unwrap();
        Self { inner: session, milvus, research_topic: topic }
    }

    pub async fn write_finding(&self, summary: String, tags: Vec<String>) -> Result<()> {
        let embedding = self.milvus.generate_embedding(&summary).await?;
        let finding = ResearchFinding {
            finding_id: format!("{}-{}", self.inner.session_id, uuid()),
            source_agent: self.inner.loop_type.clone(),
            topic: self.research_topic.clone(),
            summary,
            embedding,
            tags,
            timestamp_ms: now_ms(),
        };
        self.milvus.write_finding(finding).await
    }

    pub async fn research_find(&self, query: &str, top_k: usize) -> Result<Vec<ResearchFinding>> {
        let query_builder = QueryBuilder::new()
            .similarity(query, top_k)
            .filter(&format!("topic = '{}'", self.research_topic));
        self.milvus.search(query_builder).await
    }
}
```

### ModelRouter Integration

ModelRouter (Phase 1) routes requests to appropriate models. Phase 2 adds embedding model routing:

```rust
// ModelRouter routes embedding requests to OVHcloud endpoint
let embedding_model = router.get_model("embedding", "ovhcloud-bge-m3");
let embedding = embedding_model.generate(text).await?;
```

### Research Mode Entry/Exit

```rust
// Yardmaster or loop spawns research agent
let session = Session::new("deep-research", task_id);
let mut research_session = ResearchSession::new(session, "tokio UDS pipelining").await;

// Agent does research, writes findings to milvus
research_session.write_finding("Found pattern: pipelined UDS".into(), vec!["rust".into()]).await?;

// Agent exits research mode, returns digest
let digest = research_session.summarize_findings().await?;
return_to_loop(digest);
// milvus retains all findings
```

---

## 6. Local Mock for Testing

### In-Memory Milvus Stub

```rust
// crates/milvus-brain/src/mock.rs

use std::collections::HashMap;
use crate::{ResearchFinding, QueryBuilder, MilvusError};

pub struct MockMilvusClient {
    findings: HashMap<String, ResearchFinding>,
}

impl MockMilvusClient {
    pub fn new() -> Self {
        Self { findings: HashMap::new() }
    }

    pub async fn write_finding(&mut self, finding: ResearchFinding) -> Result<(), MilvusError> {
        self.findings.insert(finding.finding_id.clone(), finding);
        Ok(())
    }

    pub async fn search(&self, query: QueryBuilder) -> Result<Vec<ResearchFinding>, MilvusError> {
        // Naive mock: return all findings, no similarity ranking
        let mut results: Vec<_> = self.findings.values().cloned().collect();
        // Apply metadata filters (simplified)
        if let Some(filter) = query.agent_filter {
            results.retain(|f| f.source_agent == filter);
        }
        Ok(results)
    }

    pub async fn delete_finding(&mut self, finding_id: &str) -> Result<(), MilvusError> {
        self.findings.remove(finding_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_write_and_search() {
        let mut mock = MockMilvusClient::new();
        let finding = ResearchFinding { /* ... */ };
        mock.write_finding(finding.clone()).await.unwrap();
        let results = mock.search(QueryBuilder::new()).await.unwrap();
        assert_eq!(results.len(), 1);
    }
}
```

### Unit Test Usage

```rust
// crates/milvus-brain/tests/mock_test.rs

#[tokio::test]
async fn test_research_session_with_mock() {
    let mock = MockMilvusClient::new();
    let session = ResearchSession::with_mock(mock, "test-topic");
    session.write_finding("test finding".into(), vec![]).await.unwrap();
    let results = session.research_find("test", 10).await.unwrap();
    assert_eq!(results.len(), 1);
}
```

---

## 7. Docker-Compose for Local milvus Dev

### `docker-compose.milvus.yml`

```yaml
version: '3.8'

services:
  etcd:
    image: quay.io/coreos/etcd:v3.5.11
    environment:
      - ETCD_AUTO_COMPACTION_MODE=revision
      - ETCD_AUTO_COMPACTION_RETENTION=1000
      - ETCD_QUOTA_BACKEND_BYTES=4294967296
    volumes:
      - etcd_data:/etcd
    command: etcd -advertise-client-urls=http://127.0.0.1:2379 -listen-client-urls http://0.0.0.0:2379 --data-dir /etcd
    healthcheck:
      test: ["CMD", "etcdctl", "endpoint", "health"]
      interval: 30s
      timeout: 10s
      retries: 3

  minio:
    image: minio/minio:RELEASE.2023-03-20T20-16-18Z
    environment:
      MINIO_ROOT_USER: minioadmin
      MINIO_ROOT_PASSWORD: minioadmin
    volumes:
      - minio_data:/minio_data
    command: minio server /minio_data --console-address ":9001"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/minio/health/live"]
      interval: 30s
      timeout: 10s
      retries: 3

  milvus:
    image: milvusdb/milvus:v2.4.0
    command: ["milvus", "run", "standalone"]
    environment:
      ETCD_ENDPOINTS: etcd:2379
      MINIO_ADDRESS: minio:9000
    volumes:
      - milvus_data:/var/lib/milvus
    ports:
      - "19530:19530"  # gRPC
      - "9091:9091"    # HTTP (for Attu UI)
    depends_on:
      etcd:
        condition: service_healthy
      minio:
        condition: service_healthy

  attu:
    image: zilliz/attu:v2.4.0
    environment:
      MILVUS_URL: http://milvus:19530
    ports:
      - "3000:3000"
    depends_on:
      - milvus

volumes:
  etcd_data:
  minio_data:
  milvus_data:
```

### Local Dev Workflow

```bash
# Start milvus stack
docker-compose -f docker-compose.milvus.yml up -d

# Wait for healthy
docker-compose -f docker-compose.milvus.yml ps  # all should be "healthy"

# Run integration tests
cargo test --manifest-path crates/milvus-brain/Cargo.toml --test client_test

# Open Attu UI for browsing collections
open http://localhost:3000

# Stop
docker-compose -f docker-compose.milvus.yml down
```

---

## 8. Build Order / Phasing

| Phase | What | Verify |
|-------|------|--------|
| **2.1** | milvus-brain crate skeleton + mock client | mock tests pass |
| **2.2** | Embedding client (OVHcloud endpoint) | embedding generation works |
| **2.3** | Collection schema + ensure_collection() | collection created in local milvus |
| **2.4** | Write API (single + batch) | integration test: write 100 findings |
| **2.5** | Query API (similarity + metadata filter) | integration test: search returns correct results |
| **2.6** | ResearchSession extension (cognition crate) | research session can write/find |
| **2.7** | Junior research burst integration | junior burst writes to milvus, parent coder receives digest |
| **2.8** | Learning spike integration | background loop reads milvus, writes patterns |

---

## 9. Open Questions (Deferred)

- milvus deployment topology (single node vs cluster, replication across mesh)
- Embedding model selection (OVHcloud `qwen-embedding` vs `bge-m3` — benchmark needed)
- Batch size tuning (how many findings per batch write)
- Index tuning (IVF_FLAT nlist, HNSW M/efConstruction for higher recall)
- Retention policy (when to purge old findings, archive to cold storage)
- mempalace + honcho integration timeline (Phase 3 or later)

---

## Appendix A: OVHcloud AI Endpoint

```
Base URL: https://oai.endpoints.kepler.ai.cloud.ovh.net/v1
Chat Completions: /chat/completions
Embeddings: /embeddings (if available) or use chat model for embedding generation

Auth: Bearer token from OVHcloud console
Headers:
  - Authorization: Bearer <token>
  - Content-Type: application/json

Example embedding request:
POST /v1/embeddings
{
  "model": "bge-m3",
  "input": ["tokio UDS pipelining", "async IPC rust"]
}

Response:
{
  "data": [
    { "embedding": [0.1, 0.2, ...], "index": 0 },
    { "embedding": [0.3, 0.4, ...], "index": 1 }
  ]
}
```

## Appendix B: milvus Collection DDL

```sql
-- For reference: milvus collection schema in SQL-like DDL

CREATE COLLECTION research_findings (
  finding_id VARCHAR(64) PRIMARY KEY,
  agent_id VARCHAR(32),
  topic VARCHAR(256),
  summary VARCHAR(4096),
  embedding FLOAT_VECTOR(1536),
  tags JSON_ARRAY,
  source_url VARCHAR(512),
  task_id VARCHAR(64),
  slice_id VARCHAR(64),
  created_at INT64,
  embedding_model VARCHAR(32)
);

CREATE INDEX embedding_idx ON research_findings (embedding)
  WITH (index_type=IVF_FLAT, nlist=1024, metric_type=COSINE);

CREATE INDEX created_at_idx ON research_findings (created_at);
CREATE INDEX agent_id_idx ON research_findings (agent_id);
CREATE INDEX tags_idx ON research_findings (tags);
```

## Appendix C: ResearchFinding Protobuf -> milvus Mapping

| Protobuf field | milvus field | Type | Notes |
|----------------|--------------|------|-------|
| finding_id | finding_id | VARCHAR(64) | Primary key |
| source_agent | agent_id | VARCHAR(32) | Agent type |
| topic | topic | VARCHAR(256) | Research topic |
| summary | summary | VARCHAR(4096) | Human-readable |
| embedding | embedding | FLOAT_VECTOR(1536) | Cosine similarity |
| tags | tags | JSON_ARRAY | Filterable |
| timestamp_ms | created_at | INT64 | Unix ms |
| (none) | source_url | VARCHAR(512) | Optional |
| (none) | task_id | VARCHAR(64) | Task reference |
| (none) | slice_id | VARCHAR(64) | Slice reference |
| (none) | embedding_model | VARCHAR(32) | Model name |

(End of file)
