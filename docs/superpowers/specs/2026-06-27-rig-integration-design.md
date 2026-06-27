# Rig Integration Design — Alternative LLM Backend

**Date:** 2026-06-27
**Status:** Draft (design phase)
**Target:** Add Rig.rs as optional alternative to current LLM client for structured extraction tasks

---

## 1. System Identity & Goal

Rig.rs is a Rust LLM library with type-safe `Extractor<T>` API for structured text classification/extraction. Unlike raw chat completions, Rig produces typed outputs via serde + JsonSchema.

Phase 1-4 completed: cognition crate (LLM client, Session, PromptDispatcher, ModelRouter), loops crate (10 loops including yardmaster), milvus-brain (semantic memory), mempalace (short-term telemetry), honcho (long-term patterns).

This design adds Rig as **optional alternative backend** — existing LLM client stays, Rig used for structured extraction tasks.

### Success Criteria

1. Rig added as optional dependency (`rig-core` crate with `rig` feature flag)
2. `RigClient` wrapper implements same trait as `LlmClient`
3. `Extractor<T>` used for structured outputs: classification, entity extraction, slice decomposition
4. ModelRouter can route to Rig or raw LLM based on task type
5. Backward compatible — existing code unchanged unless Rig explicitly used

### Scope Boundary

This design covers Rig integration only. Migration of existing tasks to Rig is optional future work.

---

## 2. Architecture

```
+------------------+     +------------------+
|  LlmClient       |     |  RigClient       |
|  (current)       |     |  (new)           |
|  - chat()        |     |  - extractor<T>()|
|  - complete()    |     |  - classify()    |
+--------+---------+     +--------+---------+
         |                        |
         +-----------+------------+
                     |
          +----------v----------+
          |  ModelRouter        |
          |  - route by task    |
          |  - rig for structs  |
          +----------+----------+
                     |
          +----------v----------+
          |  Cognition Layer    |
          |  - Session          |
          |  - ResearchSession  |
          +---------------------+
```

### When to Use Rig

| Task Type | Recommended Backend | Why |
|-----------|-------------------|-----|
| Chat completion | LlmClient (raw) | Simple text-in/text-out |
| Structured classification | Rig Extractor | Type-safe, validated output |
| Entity extraction | Rig Extractor | Structured entities with positions |
| Slice decomposition | Rig Extractor | E2ESlice struct output |
| Pattern detection | Rig Extractor | LearningPattern struct |
| Research findings | LlmClient (raw) | Free-form text acceptable |

---

## 3. RigClient Wrapper

### Crate: `cognition` (extension)

```rust
// crates/cognition/src/rig_client.rs

use rig::{providers::openai, extractor::Extractor};
use serde::Deserialize;

pub struct RigClient {
    client: openai::Client,
    default_model: String,
}

impl RigClient {
    pub fn from_env() -> Result<Self, CognitionError> {
        let client = openai::Client::from_env()
            .map_err(|e| CognitionError::Provider(e.to_string()))?;
        Ok(Self {
            client,
            default_model: std::env::var("OVHCLOUD_AI_MODEL")
                .unwrap_or_else(|_| "Meta-Llama-3_3-70B-Instruct".into()),
        })
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.default_model = model.to_string();
        self
    }

    /// Create typed extractor for structured output
    pub fn extractor<T: serde::Serialize + serde::de::DeserializeOwned + schemars::JsonSchema>(
        &self,
        preamble: &str,
    ) -> Result<RigExtractor<T>, CognitionError> {
        let extractor = self.client
            .extractor::<T>(&self.default_model)
            .preamble(preamble)
            .build();
        Ok(RigExtractor { extractor })
    }
}

pub struct RigExtractor<T> {
    extractor: Extractor<T>,
}

impl<T> RigExtractor<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned + schemars::JsonSchema,
{
    pub async fn extract(&self, text: &str) -> Result<T, CognitionError> {
        self.extractor
            .extract(text)
            .await
            .map_err(|e| CognitionError::Provider(e.to_string()))
    }
}
```

### Dependencies

```toml
# crates/cognition/Cargo.toml

[dependencies]
rig-core = { version = "0.11", optional = true }
schemars = { version = "0.8", optional = true }

[features]
rig = ["dep:rig-core", "dep:schemars"]
```

---

## 4. Integration Points

### ModelRouter Extension

```rust
// crates/cognition/src/model_router.rs

pub enum Backend {
    Raw,      // Current LlmClient
    Rig,      // Rig Extractor
}

impl ModelRouter {
    pub fn get_backend(&self, task_type: &str) -> Backend {
        match task_type {
            "classification" | "extraction" | "decomposition" => Backend::Rig,
            _ => Backend::Raw,
        }
    }
}
```

### Yardmaster Slice Decomposition

```rust
// crates/loops/src/yardmaster.rs

#[cfg(feature = "rig")]
use cognition::rig_client::RigClient;

pub struct Yardmaster {
    #[cfg(feature = "rig")]
    rig_client: Option<RigClient>,
    // ... existing fields
}

impl Yardmaster {
    #[cfg(feature = "rig")]
    pub async fn decompose_task_rig(&self, task_desc: &str) -> Result<Vec<E2ESlice>> {
        let extractor = self.rig_client
            .as_ref()
            .unwrap()
            .extractor::<TaskDecomposition>("
                Decompose this task into E2E slices.
                Each slice has: slice_id, dependencies, loop_type, execution_mode
            ")?;
        
        let decomposition = extractor.extract(task_desc).await?;
        Ok(decomposition.slices)
    }
}

#[derive(Deserialize, JsonSchema)]
struct TaskDecomposition {
    slices: Vec<E2ESlice>,
}
```

### Honcho Pattern Extraction

```rust
// crates/honcho/src/detector.rs

#[cfg(feature = "rig")]
pub struct PatternExtractor {
    extractor: rig::extractor::Extractor<LearningPattern>,
}

#[cfg(feature = "rig")]
impl PatternExtractor {
    pub fn new(client: &RigClient) -> Result<Self> {
        let extractor = client.extractor::<LearningPattern>(
            "Extract patterns from unit stats and research findings.
            Pattern types: performance, failure, success, cross-loop"
        )?;
        Ok(Self { extractor })
    }

    pub async fn extract_pattern(&self, stats: &[UnitStats]) -> Result<LearningPattern> {
        // Serialize stats to JSON text
        let stats_text = serde_json::to_string(stats)?;
        self.extractor.extract(&stats_text).await
    }
}
```

---

## 5. Feature Flags

### Workspace Cargo.toml

```toml
[workspace]
members = [
  "crates/cognition",
  "crates/loops",
  "crates/honcho",
  # ... other crates
]

[workspace.dependencies]
rig-core = "0.11"
schemars = "0.8"

[workspace.features]
rig = ["cognition/rig"]
```

### Per-Crate Features

```toml
# crates/cognition/Cargo.toml
[features]
rig = ["dep:rig-core", "dep:schemars"]

# crates/loops/Cargo.toml
[features]
rig = ["cognition/rig"]

# crates/honcho/Cargo.toml
[features]
rig = ["cognition/rig"]
```

### Usage

```bash
# Build without Rig (default)
cargo build

# Build with Rig support
cargo build --features rig

# Test with Rig
cargo test --features rig
```

---

## 6. Migration Path

### Phase 1: Add Rig as Optional Backend

- Add `rig-core` + `schemars` as optional deps
- Implement `RigClient` wrapper
- Feature flag: `rig`

### Phase 2: Use Rig for New Features

- Yardmaster slice decomposition (Rig extractor)
- Honcho pattern extraction (Rig extractor)
- Junior burst classification (Rig extractor)

### Phase 3: Optional Migration of Existing Code

- Migrate classification tasks to Rig (optional)
- Keep raw LLM client for chat completion

---

## 7. Testing Strategy

### Unit Tests

```rust
#[cfg(feature = "rig")]
#[tokio::test]
async fn test_rig_extractor_sentiment() {
    let client = RigClient::from_env().unwrap();
    let extractor = client.extractor::<SentimentClassification>(
        "Classify sentiment: Positive, Negative, Neutral"
    ).unwrap();
    
    let result = extractor.extract("I love this!").await.unwrap();
    assert_eq!(result.sentiment, Sentiment::Positive);
}
```

### Integration Tests

```rust
#[cfg(feature = "rig")]
#[tokio::test]
async fn test_yardmaster_decompose_rig() {
    let yardmaster = Yardmaster::with_rig().await.unwrap();
    let slices = yardmaster.decompose_task_rig("Build login feature").await.unwrap();
    assert!(slices.len() > 0);
}
```

---

## 8. Build Order

| Phase | What | Verify |
|-------|------|--------|
| **Rig.1** | Add rig-core + schemars as optional deps | cargo check --features rig passes |
| **Rig.2** | Implement RigClient wrapper | Unit tests pass |
| **Rig.3** | Add RigExtractor for structured types | Extraction tests pass |
| **Rig.4** | Yardmaster integration (slice decomposition) | Integration tests pass |
| **Rig.5** | Honcho integration (pattern extraction) | Pattern detection tests pass |
| **Rig.6** | ModelRouter backend selection | Routing tests pass |

---

## 9. Model Configuration

### OVHcloud AI Endpoints

Default model: `Meta-Llama-3_3-70B-Instruct` (70B parameters, instruction-tuned)

```bash
# Environment variables for Rig + OVHcloud
OVHCLOUD_AI_API_KEY=<your-api-key>
OVHCLOUD_AI_BASE_URL=https://ai-endpoints.api.ovh.com/v1
OVHCLOUD_AI_MODEL=Meta-Llama-3_3-70B-Instruct
```

### Alternative Models

| Model | Params | Use Case |
|-------|--------|----------|
| `Meta-Llama-3_3-70B-Instruct` | 70B | Default — general reasoning, structured extraction |
| `Mistral-Small-3.2-24B-Instruct-2506` | 24B | Faster, lower cost — classification tasks |
| `DeepSeek-R1-Distill-Llama-70B` | 70B distilled | Complex reasoning, multi-step decomposition |
| `Mistral-Nemo-Instruct-2407` | 12B | Lightweight tasks, fast iteration |

### Open Questions

- Should Rig support multiple providers (Anthropic, Ollama, OVHcloud)?
- Performance comparison: Rig vs raw LLM client (latency, token usage)?
- Cost implications: Rig may use more tokens for structured output

---

## Appendix A: Example Rig Usage

```rust
use cognition::rig_client::RigClient;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
enum LoopType {
    Deepwork,
    BruteforceCoder,
    DeepResearch,
    Testers,
    Yardmaster,
    Devops,
    UI,
    RedTeam,
    Juniors,
    Ralph,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct TaskClassification {
    loop_type: LoopType,
    confidence: f32,
    reasoning: String,
}

#[tokio::main]
async fn main() {
    // Requires: OVHCLOUD_AI_API_KEY, OVHCLOUD_AI_BASE_URL, OVHCLOUD_AI_MODEL
    let client = RigClient::from_env().unwrap();
    let classifier = client.extractor::<TaskClassification>(
        "Classify this task into appropriate loop type."
    ).unwrap();
    
    let task = "Write unit tests for the transport crate";
    let result = classifier.extract(task).await.unwrap();
    
    println!("Task: {}", task);
    println!("Loop: {:?}", result.loop_type);
    println!("Confidence: {:.2}", result.confidence);
    println!("Reasoning: {}", result.reasoning);
}
```

**Expected Output:**
```
Task: Write unit tests for the transport crate
Loop: Testers
Confidence: 0.95
Reasoning: Task involves writing and running tests to verify code correctness
```

(End of file)
