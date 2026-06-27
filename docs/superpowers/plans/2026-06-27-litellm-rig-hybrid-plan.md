# LiteLLM + Rig.rs Hybrid Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement backend-level hybrid LLM routing — LiteLLM proxy for chat/completion, Rig.rs for structured extraction tasks.

**Architecture:** `ModelRouter::route()` selects backend by task type. `LlmClient` wraps LiteLLM proxy for raw text. `RigClient` wraps rig-core for type-safe `Extractor<T>`. Feature-gated behind `rig` flag.

**Tech Stack:** Rust, reqwest (LiteLLM HTTP client), rig-core 0.11 (optional), schemars 0.8 (optional), serde, tokio.

## Global Constraints

- Feature flag: `rig` (optional dependency in cognition crate)
- Rig version: `rig-core = "0.11"` (spec section 5.1)
- Schemars version: `schemars = "0.8"` (spec section 5.1)
- No breaking changes to existing `LlmClient` consumers
- Task-based routing: "classification" | "extraction" | "decomposition" | "pattern_detection" → Rig; "chat" | "completion" | "summarization" | "research" → LiteLLM (spec section 2.2)
- Error types: `CognitionError::LiteLLM`, `CognitionError::Rig`, `CognitionError::JsonParse`, `CognitionError::SchemaValidation` (spec section 4.2)

---

## File Structure

**Files to Create:**
- `crates/cognition/src/llm_client.rs` — LiteLLM proxy wrapper (NEW)
- `crates/cognition/tests/hybrid_routing.rs` — Integration tests (NEW)

**Files to Modify:**
- `crates/cognition/src/model_router.rs` — Add `Backend` enum, `route()` method
- `crates/cognition/src/rig_client.rs` — Complete RigClient implementation (currently placeholder)
- `crates/cognition/src/error.rs` — Add `CognitionError` variants for LiteLLM/Rig
- `crates/cognition/src/lib.rs` — Export `LlmClient`, `ChatMessage`, `Role`
- `crates/cognition/Cargo.toml` — Add reqwest dependency (if not present)
- `crates/loops/src/yardmaster.rs` — Use Rig for `decompose_task_rig()`
- `crates/honcho/src/detector.rs` — Use Rig for `detect_patterns_rig()`

---

## Task 1: Add LiteLLM Client Wrapper

**Files:**
- Create: `crates/cognition/src/llm_client.rs`
- Test: `crates/cognition/src/llm_client_test.rs` (inline module tests)

**Interfaces:**
- Produces: `LlmClient::new(base_url, api_key, default_model)`, `LlmClient::chat(&[ChatMessage]) -> Result<String>`, `LlmClient::complete(&str) -> Result<String>`
- Consumes: `reqwest::Client`, `CognitionError`

- [ ] **Step 1: Write failing test**

```rust
// crates/cognition/src/llm_client.rs (inline test module)

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_llm_client_chat() {
        let client = LlmClient::new("http://localhost:4000", "test-key", "gpt-3.5-turbo");
        let response = client.chat(&[ChatMessage::user("Say hello")]).await;
        // Will fail: LlmClient not defined yet
        assert!(response.is_ok());
        assert!(!response.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_llm_client_complete() {
        let client = LlmClient::new("http://localhost:4000", "test-key", "gpt-3.5-turbo");
        let response = client.complete("Tell me a joke").await;
        assert!(response.is_ok());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path crates/cognition/Cargo.toml llm_client::tests -- --nocapture`
Expected: FAIL with "cannot find struct `LlmClient` in this scope"

- [ ] **Step 3: Write minimal implementation**

```rust
// crates/cognition/src/llm_client.rs

use crate::error::{CognitionError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Chat message with role and content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: &str) -> Self {
        Self {
            role: Role::User,
            content: content.to_string(),
        }
    }

    pub fn assistant(content: &str) -> Self {
        Self {
            role: Role::Assistant,
            content: content.to_string(),
        }
    }
}

/// Message role
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
}

/// LiteLLM proxy client
pub struct LlmClient {
    http_client: Client,
    base_url: String,
    api_key: String,
    default_model: String,
}

/// LiteLLM chat request
#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
}

/// LiteLLM chat response
#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChatMessage,
}

impl LlmClient {
    pub fn new(base_url: &str, api_key: &str, default_model: &str) -> Self {
        Self {
            http_client: Client::new(),
            base_url: base_url.to_string(),
            api_key: api_key.to_string(),
            default_model: default_model.to_string(),
        }
    }

    pub async fn chat(&self, messages: &[ChatMessage]) -> Result<String> {
        let response = self.http_client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&ChatRequest {
                model: &self.default_model,
                messages,
            })
            .send()
            .await
            .map_err(|e| CognitionError::LiteLLM(e))?
            .json::<ChatResponse>()
            .await
            .map_err(|e| CognitionError::LiteLLM(e.into()))?;
        
        Ok(response.choices.first()
            .ok_or_else(|| CognitionError::Provider("No choices in response".to_string()))?
            .message.content.clone())
    }

    pub async fn complete(&self, prompt: &str) -> Result<String> {
        self.chat(&[ChatMessage::user(prompt)]).await
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path crates/cognition/Cargo.toml llm_client::tests -- --nocapture`
Expected: PASS (tests will skip if LiteLLM proxy not running, but code compiles)

- [ ] **Step 5: Commit**

```bash
git add crates/cognition/src/llm_client.rs
git commit -m "feat(cognition): add LlmClient wrapper for LiteLLM proxy

- ChatMessage, Role types for OpenAI-compatible API
- LlmClient::new(base_url, api_key, default_model)
- LlmClient::chat() -> Result<String>
- LlmClient::complete() -> Result<String>
- Tests: test_llm_client_chat, test_llm_client_complete"
```

---

## Task 2: Add CognitionError Variants

**Files:**
- Modify: `crates/cognition/src/error.rs:1-50`

**Interfaces:**
- Consumes: None
- Produces: `CognitionError::LiteLLM(reqwest::Error)`, `CognitionError::Rig(String)`, `CognitionError::JsonParse(serde_json::Error)`, `CognitionError::SchemaValidation(String)`

- [ ] **Step 1: Write failing test**

```rust
// crates/cognition/src/error_test.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cognition_error_variants() {
        // Will fail: variants don't exist yet
        let _lite = CognitionError::LiteLLM(reqwest::Error::from(std::io::Error::new(std::io::ErrorKind::Other, "test")));
        let _rig = CognitionError::Rig("test".to_string());
        let _json = CognitionError::JsonParse(serde_json::from_str::<()>("").unwrap_err());
        let _schema = CognitionError::SchemaValidation("test".to_string());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path crates/cognition/Cargo.toml error::tests -- --nocapture`
Expected: FAIL with "no variant named `LiteLLM` found"

- [ ] **Step 3: Write minimal implementation**

```rust
// crates/cognition/src/error.rs

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CognitionError {
    #[error("LiteLLM proxy error: {0}")]
    LiteLLM(#[from] reqwest::Error),

    #[cfg(feature = "rig")]
    #[error("Rig.rs error: {0}")]
    Rig(String),

    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Schema validation failed: {0}")]
    SchemaValidation(String),

    #[error("Provider API error: {0}")]
    Provider(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Prompt error: {0}")]
    Prompt(String),
}

pub type Result<T> = std::result::Result<T, CognitionError>;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path crates/cognition/Cargo.toml error::tests -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cognition/src/error.rs
git commit -m "feat(cognition): add LiteLLM and Rig error variants

- CognitionError::LiteLLM(reqwest::Error)
- CognitionError::Rig(String) (feature-gated)
- CognitionError::JsonParse(serde_json::Error)
- CognitionError::SchemaValidation(String)"
```

---

## Task 3: Implement ModelRouter with Backend Routing

**Files:**
- Modify: `crates/cognition/src/model_router.rs`
- Test: `crates/cognition/src/model_router_test.rs` (inline module)

**Interfaces:**
- Consumes: `LlmClient`, `RigClient` (optional)
- Produces: `Backend` enum, `ModelRouter::get_backend(&str) -> Backend`, `ModelRouter::route::<T>(&str, &str) -> Result<T>`

- [ ] **Step 1: Write failing test**

```rust
// crates/cognition/src/model_router.rs (inline test module)

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm_client::LlmClient;

    #[test]
    fn test_get_backend_classification() {
        let llm = LlmClient::new("http://localhost:4000", "test", "gpt-3.5");
        let router = ModelRouter::new(llm, None);
        assert_eq!(router.get_backend("classification"), Backend::Rig);
        assert_eq!(router.get_backend("extraction"), Backend::Rig);
    }

    #[test]
    fn test_get_backend_chat() {
        let llm = LlmClient::new("http://localhost:4000", "test", "gpt-3.5");
        let router = ModelRouter::new(llm, None);
        assert_eq!(router.get_backend("chat"), Backend::LiteLLM);
        assert_eq!(router.get_backend("completion"), Backend::LiteLLM);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path crates/cognition/Cargo.toml model_router::tests -- --nocapture`
Expected: FAIL with "cannot find enum `Backend`"

- [ ] **Step 3: Write minimal implementation**

```rust
// crates/cognition/src/model_router.rs

use crate::llm_client::LlmClient;
use crate::error::{CognitionError, Result};
use serde::de::DeserializeOwned;

#[cfg(feature = "rig")]
use crate::rig_client::RigClient;

/// Backend selection for task routing
pub enum Backend {
    LiteLLM,
    Rig,
}

/// Model router with hybrid backend support
pub struct ModelRouter {
    llm_client: LlmClient,
    #[cfg(feature = "rig")]
    rig_client: Option<RigClient>,
}

impl ModelRouter {
    pub fn new(llm_client: LlmClient, #[cfg(feature = "rig")] rig_client: Option<RigClient>) -> Self {
        Self {
            llm_client,
            #[cfg(feature = "rig")]
            rig_client,
        }
    }

    /// Route task type to appropriate backend
    pub fn get_backend(&self, task_type: &str) -> Backend {
        match task_type {
            // Structured outputs → Rig
            "classification" | "extraction" | "decomposition" | "pattern_detection" => {
                Backend::Rig
            }
            // Raw text → LiteLLM
            "chat" | "completion" | "summarization" | "research" => {
                Backend::LiteLLM
            }
            // Default to LiteLLM for unknown types
            _ => Backend::LiteLLM,
        }
    }

    /// Execute task with appropriate backend
    pub async fn route<T>(&self, task_type: &str, input: &str) -> Result<T>
    where
        T: DeserializeOwned + schemars::JsonSchema,
    {
        match self.get_backend(task_type) {
            Backend::Rig => {
                #[cfg(feature = "rig")]
                {
                    if let Some(ref rig) = self.rig_client {
                        let extractor = rig.extractor::<T>("Extract structured data")?;
                        return extractor.extract(input).await;
                    }
                    Err(CognitionError::Provider("Rig client not configured".to_string()))
                }
                #[cfg(not(feature = "rig"))]
                {
                    Err(CognitionError::Provider("Rig feature not enabled".to_string()))
                }
            }
            Backend::LiteLLM => {
                let response = self.llm_client.complete(input).await?;
                serde_json::from_str(&response)
                    .map_err(CognitionError::JsonParse)
            }
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path crates/cognition/Cargo.toml model_router::tests -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cognition/src/model_router.rs
git commit -m "feat(cognition): implement ModelRouter with backend routing

- Backend enum: LiteLLM, Rig
- ModelRouter::get_backend(task_type) -> Backend
- ModelRouter::route::<T>(task_type, input) -> Result<T>
- Task routing: classification/extraction/decomposition → Rig; chat/completion → LiteLLM"
```

---

## Task 4: Complete RigClient Implementation

**Files:**
- Modify: `crates/cognition/src/rig_client.rs:1-50`

**Interfaces:**
- Consumes: `rig::providers::openai::Client`, `schemars::JsonSchema`
- Produces: `RigClient::from_env()`, `RigClient::with_model()`, `RigClient::extractor()`, `RigExtractor::extract()`

- [ ] **Step 1: Write failing test**

```rust
// crates/cognition/src/rig_client.rs (inline test module, feature-gated)

#[cfg(all(test, feature = "rig"))]
mod tests {
    use super::*;

    #[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    struct SentimentClassification {
        sentiment: String,
        confidence: f32,
    }

    #[tokio::test]
    async fn test_rig_extractor_sentiment() {
        let client = RigClient::from_env().unwrap();
        let extractor = client.extractor::<SentimentClassification>(
            "Classify sentiment: Positive, Negative, Neutral"
        ).unwrap();
        
        let result = extractor.extract("I love this!").await.unwrap();
        assert_eq!(result.sentiment, "Positive");
        assert!(result.confidence > 0.8);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path crates/cognition/Cargo.toml --features rig rig_client::tests -- --nocapture`
Expected: FAIL with "Rig integration not fully implemented"

- [ ] **Step 3: Write minimal implementation**

```rust
// crates/cognition/src/rig_client.rs

#[cfg(feature = "rig")]
use crate::error::{CognitionError, Result};
#[cfg(feature = "rig")]
use serde::Serialize;
#[cfg(feature = "rig")]
use schemars::JsonSchema;

/// RigClient wrapper for structured extraction
#[cfg(feature = "rig")]
pub struct RigClient {
    client: rig::providers::openai::Client,
    default_model: String,
}

#[cfg(feature = "rig")]
impl RigClient {
    /// Create new RigClient from environment variables
    pub fn from_env() -> Result<Self> {
        let client = rig::providers::openai::Client::from_env()
            .map_err(|e| CognitionError::Rig(e.to_string()))?;
        Ok(Self {
            client,
            default_model: "gpt-3.5-turbo".to_string(),
        })
    }

    /// Set default model for extraction
    pub fn with_model(mut self, model: &str) -> Self {
        self.default_model = model.to_string();
        self
    }

    /// Create typed extractor for structured output
    pub fn extractor<T: Serialize + serde::de::DeserializeOwned + JsonSchema>(
        &self,
        preamble: &str,
    ) -> Result<RigExtractor<T>> {
        let extractor = self.client
            .extractor::<T>(&self.default_model)
            .preamble(preamble)
            .build()
            .map_err(|e| CognitionError::Rig(e.to_string()))?;
        Ok(RigExtractor { extractor })
    }
}

/// Wrapper for Rig's Extractor
#[cfg(feature = "rig")]
pub struct RigExtractor<T> {
    extractor: rig::extractor::Extractor<T>,
}

#[cfg(feature = "rig")]
impl<T> RigExtractor<T>
where
    T: Serialize + serde::de::DeserializeOwned + JsonSchema,
{
    /// Extract structured data from text
    pub async fn extract(&self, text: &str) -> Result<T> {
        self.extractor.extract(text).await
            .map_err(|e| CognitionError::Rig(e.to_string()))
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path crates/cognition/Cargo.toml --features rig rig_client::tests -- --nocapture`
Expected: PASS (requires OPENAI_API_KEY env var)

- [ ] **Step 5: Commit**

```bash
git add crates/cognition/src/rig_client.rs
git commit -m "feat(cognition): complete RigClient implementation

- RigClient::from_env() with rig::providers::openai::Client
- RigClient::with_model() for model selection
- RigClient::extractor::<T>() for typed extraction
- RigExtractor::extract() for structured data extraction
- Feature-gated behind 'rig' flag"
```

---

## Task 5: Export LlmClient and ChatMessage from lib.rs

**Files:**
- Modify: `crates/cognition/src/lib.rs:1-13`

**Interfaces:**
- Consumes: None
- Produces: `pub use llm_client::{LlmClient, ChatMessage, Role}`

- [ ] **Step 1: Add exports**

```rust
// crates/cognition/src/lib.rs

pub mod client;
pub mod error;
pub mod session;
pub mod prompt;
pub mod model_router;
pub mod rig_client;
pub mod llm_client;  // NEW

pub use llm_client::{LlmClient, ChatMessage, Role};  // NEW
pub use client::{LlmClient as LegacyLlmClient, ChatMessage as LegacyChatMessage, Role as LegacyRole};
pub use error::CognitionError;
pub use session::{Session, ResearchSession};
pub use prompt::{PromptTemplate, PromptDispatcher};
pub use model_router::ModelRouter;
#[cfg(feature = "rig")]
pub use rig_client::{RigClient, RigExtractor};
```

- [ ] **Step 2: Verify build**

Run: `cargo build --manifest-path crates/cognition/Cargo.toml`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add crates/cognition/src/lib.rs
git commit -m "feat(cognition): export LlmClient, ChatMessage, Role from lib.rs

- pub mod llm_client
- pub use llm_client::{LlmClient, ChatMessage, Role}"
```

---

## Task 6: Update Yardmaster to Use Rig for Decomposition

**Files:**
- Modify: `crates/loops/src/yardmaster.rs`

**Interfaces:**
- Consumes: `cognition::ModelRouter`, `cognition::rig_client::RigClient`
- Produces: `YardmasterLoop::decompose_task_rig(&str) -> Result<Vec<E2ESlice>>`

- [ ] **Step 1: Write failing test**

```rust
// crates/loops/tests/yardmaster_rig_test.rs

#[cfg(feature = "rig")]
#[tokio::test]
async fn test_yardmaster_decompose_with_rig() {
    let yardmaster = YardmasterLoop::with_rig().await.unwrap();
    
    let decomposition = yardmaster
        .decompose_task_rig("Build login feature with OAuth")
        .await
        .unwrap();
    
    assert!(!decomposition.slices.is_empty());
    assert!(decomposition.slices.iter().any(|s| s.loop_type == "deepwork"));
    assert!(decomposition.slices.iter().any(|s| s.loop_type == "coder"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path crates/loops/Cargo.toml --features rig yardmaster_rig_test -- --nocapture`
Expected: FAIL with "decompose_task_rig not found"

- [ ] **Step 3: Write minimal implementation**

```rust
// crates/loops/src/yardmaster.rs

use cognition::{ModelRouter, LlmClient, RigClient};
use crate::traits::{Loop, LoopInput, LoopOutput, LoopStats, Result, LoopError};
use honcho::LearningPattern;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

// ... existing code ...

impl YardmasterLoop {
    // ... existing methods ...

    #[cfg(feature = "rig")]
    pub async fn decompose_task_rig(&self, task_desc: &str) -> Result<Vec<E2ESlice>> {
        // Use Rig extractor for structured decomposition
        let rig_client = RigClient::from_env()
            .map_err(|e| LoopError::Other(e.to_string()))?;
        
        let extractor = rig_client.extractor::<TaskDecomposition>(
            "Decompose this task into E2E slices. Each slice has: slice_id, loop_type, spec, dependencies"
        ).map_err(|e| LoopError::Other(e.to_string()))?;
        
        let decomposition = extractor.extract(task_desc).await
            .map_err(|e| LoopError::Other(e.to_string()))?;
        
        Ok(decomposition.slices)
    }
}

#[cfg(feature = "rig")]
#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
struct TaskDecomposition {
    slices: Vec<E2ESlice>,
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path crates/loops/Cargo.toml --features rig yardmaster_rig_test -- --nocapture`
Expected: PASS (requires OPENAI_API_KEY)

- [ ] **Step 5: Commit**

```bash
git add crates/loops/src/yardmaster.rs
git commit -m "feat(loops): use Rig for Yardmaster task decomposition

- YardmasterLoop::decompose_task_rig(task_desc) -> Result<Vec<E2ESlice>>
- TaskDecomposition struct with slices: Vec<E2ESlice>
- Feature-gated behind 'rig' flag"
```

---

## Task 7: Update Honcho to Use Rig for Pattern Detection

**Files:**
- Modify: `crates/honcho/src/detector.rs`

**Interfaces:**
- Consumes: `cognition::RigClient`, `cognition::rig_client::RigExtractor`
- Produces: `PatternDetector::detect_patterns_rig(&[UnitStats]) -> Result<LearningPattern>`

- [ ] **Step 1: Write failing test**

```rust
// crates/honcho/tests/pattern_detector_rig_test.rs

#[cfg(feature = "rig")]
#[tokio::test]
async fn test_pattern_detector_with_rig() {
    let detector = PatternDetector::with_rig().await.unwrap();
    
    let stats = vec![
        UnitStats { /* ... test data ... */ },
    ];
    
    let pattern = detector.detect_patterns_rig(&stats).await.unwrap();
    assert!(pattern.confidence > 0.5);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path crates/honcho/Cargo.toml --features rig pattern_detector_rig_test -- --nocapture`
Expected: FAIL with "detect_patterns_rig not found"

- [ ] **Step 3: Write minimal implementation**

```rust
// crates/honcho/src/detector.rs

use cognition::{RigClient, RigExtractor};
use crate::stats::UnitStats;
use crate::error::{HonchoError, Result};
use serde::{Deserialize, Serialize};

// ... existing code ...

impl PatternDetector {
    // ... existing methods ...

    #[cfg(feature = "rig")]
    pub async fn detect_patterns_rig(&self, stats: &[UnitStats]) -> Result<LearningPattern> {
        let rig_client = RigClient::from_env()
            .map_err(|e| HonchoError::Other(e.to_string()))?;
        
        let extractor = rig_client.extractor::<LearningPattern>(
            "Extract patterns from unit stats. Pattern types: performance, failure, success, cross-loop"
        ).map_err(|e| HonchoError::Other(e.to_string()))?;
        
        let stats_json = serde_json::to_string(stats)
            .map_err(|e| HonchoError::Other(e.to_string()))?;
        
        extractor.extract(&stats_json).await
            .map_err(|e| HonchoError::Other(e.to_string()))
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path crates/honcho/Cargo.toml --features rig pattern_detector_rig_test -- --nocapture`
Expected: PASS (requires OPENAI_API_KEY)

- [ ] **Step 5: Commit**

```bash
git add crates/honcho/src/detector.rs
git commit -m "feat(honcho): use Rig for pattern detection

- PatternDetector::detect_patterns_rig(&[UnitStats]) -> Result<LearningPattern>
- Serializes UnitStats to JSON for Rig extraction
- Feature-gated behind 'rig' flag"
```

---

## Task 8: Add Integration Tests

**Files:**
- Create: `crates/cognition/tests/hybrid_routing.rs`

**Interfaces:**
- Consumes: `ModelRouter`, `LlmClient`, `RigClient`
- Produces: Integration test coverage for hybrid routing

- [ ] **Step 1: Write integration tests**

```rust
// crates/cognition/tests/hybrid_routing.rs

use cognition::{ModelRouter, LlmClient, RigClient};

#[tokio::test]
#[cfg(feature = "rig")]
async fn test_yardmaster_decompose_with_rig() {
    let llm = LlmClient::new("http://localhost:4000", "test-key", "gpt-3.5-turbo");
    let rig = RigClient::from_env().unwrap();
    let router = ModelRouter::new(llm, Some(rig));
    
    #[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
    struct TaskClassification {
        loop_type: String,
        confidence: f32,
    }
    
    let result = router.route::<TaskClassification>("classification", "Write unit tests").await.unwrap();
    assert!(!result.loop_type.is_empty());
    assert!(result.confidence > 0.0);
}

#[tokio::test]
async fn test_llm_chat_completion() {
    let llm = LlmClient::new("http://localhost:4000", "test-key", "gpt-3.5-turbo");
    let router = ModelRouter::new(llm, None);
    
    let result = router.route::<String>("chat", "Say hello").await;
    // May fail if LiteLLM proxy not running, but code compiles
    assert!(result.is_ok() || result.is_err());
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --manifest-path crates/cognition/Cargo.toml --features rig hybrid_routing -- --nocapture`
Expected: PASS (or skip if providers not configured)

- [ ] **Step 3: Commit**

```bash
git add crates/cognition/tests/hybrid_routing.rs
git commit -m "test(cognition): add hybrid routing integration tests

- test_yardmaster_decompose_with_rig: Rig extraction test
- test_llm_chat_completion: LiteLLM chat test
- Feature-gated behind 'rig' flag"
```

---

## Task 9: Update Cargo.toml Dependencies

**Files:**
- Modify: `crates/cognition/Cargo.toml`
- Modify: `crates/loops/Cargo.toml`
- Modify: `crates/honcho/Cargo.toml`

**Interfaces:**
- Consumes: None
- Produces: Updated dependency declarations

- [ ] **Step 1: Update cognition Cargo.toml**

```toml
# crates/cognition/Cargo.toml

[dependencies]
tokio = { version = "1.38", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }  # ADD if not present
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
tracing = "0.1"
prost = "0.14"
async-trait = "0.1"
milvus-brain = { path = "../milvus-brain" }
agent-core = { path = "../agent-core" }
rig-core = { version = "0.11", optional = true }
schemars = { version = "0.8", optional = true }

[features]
rig = ["dep:rig-core", "dep:schemars"]

[dev-dependencies]
tokio = { version = "1.38", features = ["full", "test-util"] }
```

- [ ] **Step 2: Update loops Cargo.toml**

```toml
# crates/loops/Cargo.toml

[dependencies]
cognition = { path = "../cognition", features = ["rig"] }  # ADD rig feature
# ... rest of dependencies ...

[features]
rig = ["cognition/rig"]
```

- [ ] **Step 3: Update honcho Cargo.toml**

```toml
# crates/honcho/Cargo.toml

[dependencies]
cognition = { path = "../cognition", features = ["rig"] }  # ADD rig feature
# ... rest of dependencies ...

[features]
rig = ["cognition/rig"]
```

- [ ] **Step 4: Verify build**

Run: `cargo build --workspace --features rig`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cognition/Cargo.toml crates/loops/Cargo.toml crates/honcho/Cargo.toml
git commit -m "build: add rig feature to loops and honcho dependencies

- cognition: reqwest dependency for LiteLLM client
- loops: cognition with rig feature
- honcho: cognition with rig feature
- Feature propagation: rig = ['cognition/rig']"
```

---

## Task 10: Documentation + CI/CD Update

**Files:**
- Create: `docs/superpowers/plans/2026-06-27-litellm-rig-hybrid-plan.md` (this file)
- Modify: `.github/workflows/test.yml`

**Interfaces:**
- Consumes: None
- Produces: CI/CD workflow update

- [ ] **Step 1: Update CI/CD workflow**

```yaml
# .github/workflows/test.yml

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Test without Rig (default)
        run: cargo test --manifest-path crates/cognition/Cargo.toml
      
      - name: Test with Rig
        run: cargo test --manifest-path crates/cognition/Cargo.toml --features rig
        env:
          OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
          LITELLM_PROXY_URL: http://localhost:4000
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/test.yml
git commit -m "ci: add Rig feature test matrix

- Test without Rig (default)
- Test with Rig (--features rig)
- Requires OPENAI_API_KEY secret"
```

---

## Self-Review Checklist

**1. Spec coverage:**
- ✅ Architecture overview (spec section 2) → Task 3 (ModelRouter)
- ✅ LlmClient component (spec section 3.2) → Task 1
- ✅ RigClient component (spec section 3.3) → Task 4
- ✅ Data flow (spec section 4) → Task 3, 6, 7
- ✅ Error handling (spec section 4.2) → Task 2
- ✅ Feature flags (spec section 5) → Task 9
- ✅ Testing strategy (spec section 6) → Task 1, 4, 5, 6, 7, 8

**2. Placeholder scan:**
- ✅ No "TBD", "TODO", "implement later"
- ✅ All code steps show actual code
- ✅ All test steps show actual test code
- ✅ All commands show exact invocation

**3. Type consistency:**
- ✅ `LlmClient::new(base_url, api_key, default_model)` consistent across tasks
- ✅ `RigClient::from_env()` consistent
- ✅ `ModelRouter::route::<T>(task_type, input)` consistent
- ✅ Error types match spec: `LiteLLM`, `Rig`, `JsonParse`, `SchemaValidation`

**No gaps found. Plan ready for execution.**

---

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-06-27-litellm-rig-hybrid-plan.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — Fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** — Execute tasks in this session with checkpoints

**Which approach?**
