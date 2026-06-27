# LiteLLM + Rig.rs Hybrid Integration Design

**Date:** 2026-06-27
**Status:** Approved (Design Phase)
**Target:** Hybrid LLM backend — LiteLLM for chat/completion, Rig.rs for structured extraction

---

## 1. System Identity & Goal

### 1.1 Problem Statement

Current architecture has two LLM integration paths:
- **LiteLLM proxy** — handles all LLM providers (OpenAI, Anthropic, etc.) via unified API, but only supports raw chat/completion (no schema enforcement)
- **Rig.rs** — provides type-safe `Extractor<T>` for structured outputs, but requires direct provider keys (bypasses LiteLLM)

Need unified approach that:
1. Uses LiteLLM proxy for all raw chat/completion tasks
2. Uses Rig.rs for structured extraction tasks (classification, decomposition, pattern detection)
3. Maintains backward compatibility with existing code
4. Keeps feature-gated Rig integration optional

### 1.2 Success Criteria

1. `ModelRouter::route(task_type, input)` automatically selects correct backend
2. Rig integration remains behind `rig` feature flag (optional dependency)
3. Existing `LlmClient` code unchanged — no breaking changes
4. Structured outputs guaranteed via Rig's `Extractor<T>` + JsonSchema
5. Raw text outputs via LiteLLM proxy (no schema overhead)

### 1.3 Scope Boundary

This design covers **backend-level hybrid** approach only:
- ✅ `ModelRouter` routes by task type
- ✅ `LlmClient` for LiteLLM proxy (chat, completion)
- ✅ `RigClient` for rig-core (structured extraction)
- ❌ LiteLLM proxy modifications (out of scope)
- ❌ Rig.rs service deployment (out of scope)

---

## 2. Architecture

### 2.1 High-Level Design

```
┌─────────────────────────────────────────────────────────────┐
│                    Cognition Layer                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐         ┌──────────────┐                 │
│  │  LlmClient   │         │  RigClient   │                 │
│  │  (LiteLLM)   │         │  (rig-core)  │                 │
│  │              │         │              │                 │
│  │ - chat()     │         │ - extractor()│                 │
│  │ - complete() │         │ - classify() │                 │
│  │              │         │              │                 │
│  │ Raw text     │         │ Structured   │                 │
│  │ in/out       │         │ JSON output  │                 │
│  └──────┬───────┘         └──────┬───────┘                 │
│         │                        │                          │
│         └──────────┬─────────────┘                          │
│                    │                                        │
│           ┌────────▼────────┐                               │
│           │   ModelRouter   │                               │
│           │                 │                               │
│           │ route_by_task() │                               │
│           └────────┬────────┘                               │
│                    │                                        │
└────────────────────┼────────────────────────────────────────┘
                     │
         ┌───────────┴───────────┐
         │                       │
    ┌────▼────┐           ┌──────▼──────┐
    │ LiteLLM │           │  Provider   │
    │ Proxy   │           │  (OpenAI,   │
    │         │           │  Anthropic) │
    └─────────┘           └─────────────┘
```

### 2.2 Task-Based Routing

| Task Type | Backend | Why |
|-----------|---------|-----|
| `classification` | Rig | Needs enum output, validated |
| `extraction` | Rig | Needs structured entities |
| `decomposition` | Rig | Needs `Vec<E2ESlice>` |
| `pattern_detection` | Rig | Needs `LearningPattern` struct |
| `chat` | LiteLLM | Free-form conversation |
| `completion` | LiteLLM | Raw text generation |
| `summarization` | LiteLLM | Narrative output |
| `research` | LiteLLM | Free-form findings |

### 2.3 Key Design Decisions

1. **Backend-level hybrid:** Rig and LiteLLM coexist in cognition crate — no proxy changes needed
2. **Feature-gated:** Rig integration behind `rig` feature flag (existing design doc)
3. **No breaking changes:** Existing `LlmClient` code continues unchanged
4. **Schema enforcement:** Rig's `Extractor<T>` guarantees JSON conforms to Rust type
5. **Fallback strategy:** Rig schema validation failure → retry with LiteLLM (log warning)

---

## 3. Component Design

### 3.1 ModelRouter Extension

```rust
// crates/cognition/src/model_router.rs

pub enum Backend {
    LiteLLM,  // Raw chat/completion via LiteLLM proxy
    Rig,      // Structured extraction via rig-core
}

impl ModelRouter {
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
        T: serde::de::DeserializeOwned + schemars::JsonSchema,
    {
        match self.get_backend(task_type) {
            Backend::Rig => {
                // Use Rig Extractor<T>
                let extractor = self.rig_client.extractor::<T>(preamble)?;
                let result = extractor.extract(input).await?;
                Ok(result)
            }
            Backend::LiteLLM => {
                // Use raw LLM client, parse JSON manually
                let response = self.llm_client.complete(input).await?;
                let result: T = serde_json::from_str(&response)?;
                Ok(result)
            }
        }
    }
}
```

### 3.2 LlmClient (LiteLLM Proxy)

```rust
// crates/cognition/src/llm_client.rs

pub struct LlmClient {
    http_client: reqwest::Client,
    base_url: String,  // LiteLLM proxy URL
    api_key: String,
    default_model: String,
}

impl LlmClient {
    /// Chat completion via LiteLLM proxy
    pub async fn chat(&self, messages: &[Message]) -> Result<String> {
        let response = self.http_client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&ChatRequest {
                model: &self.default_model,
                messages,
            })
            .send()
            .await?
            .json::<ChatResponse>()
            .await?;
        
        Ok(response.choices[0].message.content.clone())
    }

    /// Raw completion (no schema enforcement)
    pub async fn complete(&self, prompt: &str) -> Result<String> {
        self.chat(&[Message::user(prompt)]).await
    }
}
```

### 3.3 RigClient (rig-core Wrapper)

```rust
// crates/cognition/src/rig_client.rs

#[cfg(feature = "rig")]
pub struct RigClient {
    client: rig::providers::openai::Client,
    default_model: String,
}

#[cfg(feature = "rig")]
impl RigClient {
    /// Create typed extractor for structured output
    pub fn extractor<T: serde::Serialize + serde::de::DeserializeOwned + schemars::JsonSchema>(
        &self,
        preamble: &str,
    ) -> Result<RigExtractor<T>> {
        let extractor = self.client
            .extractor::<T>(&self.default_model)
            .preamble(preamble)
            .build();
        Ok(RigExtractor { extractor })
    }
}

#[cfg(feature = "rig")]
pub struct RigExtractor<T> {
    extractor: rig::extractor::Extractor<T>,
}

#[cfg(feature = "rig")]
impl<T> RigExtractor<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned + schemars::JsonSchema,
{
    pub async fn extract(&self, text: &str) -> Result<T> {
        self.extractor.extract(text).await
            .map_err(|e| CognitionError::Provider(e.to_string()))
    }
}
```

### 3.4 Integration Points

| Component | Uses LlmClient | Uses RigClient | Why |
|-----------|---------------|----------------|-----|
| `ResearchSession` | ✅ (research findings, free-form) | ❌ | Research output is narrative |
| `YardmasterLoop::decompose_task()` | ❌ | ✅ (E2ESlice struct) | Needs structured slice decomposition |
| `PatternDetector::detect_patterns()` | ❌ | ✅ (LearningPattern struct) | Needs structured pattern extraction |
| `ModelRouter::route()` | ✅ (chat, completion) | ✅ (classification, extraction) | Routes by task type |

---

## 4. Data Flow

### 4.1 Task Execution Flow

```
User Request
     │
     ▼
┌─────────────────────────────────────────┐
│  Yardmaster::decompose_task(task_desc)  │
│  → needs Vec<E2ESlice> (structured)     │
└────────────────┬────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────┐
│  ModelRouter::route("decomposition")    │
│  → get_backend() returns Backend::Rig   │
└────────────────┬────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────┐
│  RigClient::extractor::<TaskDecomp>()   │
│  → preamble: "Decompose into E2E slices"│
└────────────────┬────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────┐
│  Rig Extractor<T> calls OpenAI API      │
│  → response_format: { type: "json_schema" }
└────────────────┬────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────┐
│  Returns TaskDecomposition struct       │
│  → slices: Vec<E2ESlice>                │
│  → validated against JsonSchema         │
└─────────────────────────────────────────┘
```

### 4.2 Error Handling

```rust
// crates/cognition/src/error.rs

pub enum CognitionError {
    /// LiteLLM proxy errors
    LiteLLM(reqwest::Error),
    
    /// Rig.rs errors (feature-gated)
    #[cfg(feature = "rig")]
    Rig(String),
    
    /// JSON parsing errors (LiteLLM path only)
    JsonParse(serde_json::Error),
    
    /// Schema validation failed (Rig path only)
    SchemaValidation(String),
    
    /// Provider API errors (both paths)
    Provider(String),
}

impl ModelRouter {
    pub async fn route<T>(&self, task_type: &str, input: &str) -> Result<T> {
        match self.get_backend(task_type) {
            Backend::Rig => {
                self.rig_client
                    .extractor::<T>(preamble)
                    .map_err(|e| CognitionError::Rig(e.to_string()))?
                    .extract(input)
                    .await
                    .map_err(|e| CognitionError::Rig(e.to_string()))
            }
            Backend::LiteLLM => {
                let response = self.llm_client.complete(input).await
                    .map_err(CognitionError::LiteLLM)?;
                
                serde_json::from_str(&response)
                    .map_err(CognitionError::JsonParse)
            }
        }
    }
}
```

### 4.3 Error Recovery Strategy

| Error Type | Retry? | Fallback? | Action |
|------------|--------|-----------|--------|
| LiteLLM timeout | ✅ (3x) | ❌ | Exponential backoff |
| Rig schema validation | ❌ | ✅ LiteLLM | Log warning, retry with raw LLM |
| Provider API error | ✅ (3x) | ❌ | Check provider status |
| JSON parse failure | ❌ | ❌ | Return error to caller |

---

## 5. Feature Flag Strategy

### 5.1 Cargo.toml Configuration

```toml
# crates/cognition/Cargo.toml

[dependencies]
rig-core = { version = "0.11", optional = true }
schemars = { version = "0.8", optional = true }
reqwest = "0.11"  # Always needed for LiteLLM

[features]
rig = ["dep:rig-core", "dep:schemars"]
```

### 5.2 Conditional Compilation

```rust
// crates/cognition/src/lib.rs

#[cfg(feature = "rig")]
pub mod rig_client;

#[cfg(feature = "rig")]
pub use rig_client::{RigClient, RigExtractor};

pub struct ModelRouter {
    llm_client: LlmClient,  // Always present
    #[cfg(feature = "rig")]
    rig_client: Option<RigClient>,  // Only with rig feature
}
```

### 5.3 Build Commands

```bash
# Build without Rig (default)
cargo build --manifest-path crates/cognition/Cargo.toml

# Build with Rig support
cargo build --manifest-path crates/cognition/Cargo.toml --features rig

# Test with Rig
cargo test --manifest-path crates/cognition/Cargo.toml --features rig
```

---

## 6. Testing Strategy

### 6.1 Unit Tests

```rust
// crates/cognition/src/model_router_test.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_backend_classification() {
        let router = ModelRouter::new();
        assert_eq!(router.get_backend("classification"), Backend::Rig);
        assert_eq!(router.get_backend("extraction"), Backend::Rig);
    }

    #[test]
    fn test_get_backend_chat() {
        let router = ModelRouter::new();
        assert_eq!(router.get_backend("chat"), Backend::LiteLLM);
        assert_eq!(router.get_backend("completion"), Backend::LiteLLM);
    }

    #[tokio::test]
    #[cfg(feature = "rig")]
    async fn test_rig_extractor_sentiment() {
        let client = RigClient::from_env().unwrap();
        let extractor = client.extractor::<SentimentClassification>(
            "Classify sentiment: Positive, Negative, Neutral"
        ).unwrap();
        
        let result = extractor.extract("I love this!").await.unwrap();
        assert_eq!(result.sentiment, Sentiment::Positive);
        assert!(result.confidence > 0.8);
    }

    #[tokio::test]
    async fn test_llm_client_chat() {
        let client = LlmClient::new("http://localhost:4000", "test-key");
        let response = client.chat(&[Message::user("Say hello")]).await.unwrap();
        assert!(!response.is_empty());
    }
}
```

### 6.2 Integration Tests

```rust
// crates/cognition/tests/hybrid_routing.rs

#[tokio::test]
#[cfg(feature = "rig")]
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

#[tokio::test]
async fn test_pattern_detector_with_rig() {
    let detector = PatternDetector::with_rig().await.unwrap();
    
    let stats = vec![
        UnitStats { /* ... */ },
        UnitStats { /* ... */ },
    ];
    
    let pattern = detector.detect_patterns_rig(&stats).await.unwrap();
    assert!(pattern.confidence > 0.5);
}
```

### 6.3 Test Matrix

| Component | Test Type | Feature Flag | Provider Required |
|-----------|-----------|--------------|-------------------|
| `ModelRouter::get_backend()` | Unit | None | ❌ |
| `LlmClient::chat()` | Unit + Integration | None | ✅ LiteLLM proxy |
| `RigClient::extractor()` | Unit + Integration | `rig` | ✅ OpenAI/Anthropic |
| `Yardmaster::decompose_task_rig()` | Integration | `rig` | ✅ OpenAI |
| `PatternDetector::detect_patterns_rig()` | Integration | `rig` | ✅ OpenAI |

### 6.4 CI/CD Integration

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

---

## 7. Migration Path

### Phase 1: Add LiteLLM Client

- Create `LlmClient` wrapper for LiteLLM proxy
- Update `ModelRouter` to use `LlmClient` for raw chat
- No breaking changes — existing code continues

### Phase 2: Integrate Rig for Structured Tasks

- Implement `RigClient` wrapper (existing design doc)
- Update `ModelRouter::route()` to select backend by task type
- Feature flag: `rig`

### Phase 3: Migrate Existing Consumers

- `YardmasterLoop::decompose_task()` → use Rig
- `PatternDetector::detect_patterns()` → use Rig
- `ResearchSession` → stays with LiteLLM (narrative output)

### Phase 4: Optional Enhancements

- Fallback: Rig validation failure → retry with LiteLLM
- Caching: Cache Rig extraction results by input hash
- Metrics: Track Rig vs LiteLLM usage, latency, error rates

---

## 8. Open Questions

1. **Which OpenAI models support Rig's JsonSchema extraction?** (GPT-3.5-turbo, GPT-4, o1?)
2. **Should Rig support multiple providers via LiteLLM?** (Anthropic, Ollama, OVHcloud)
3. **Performance comparison:** Rig vs raw LiteLLM (latency, token usage, cost)?
4. **Fallback threshold:** How many Rig validation failures before switching to LiteLLM permanently?

---

## Appendix A: Example Usage

```rust
use cognition::{ModelRouter, LlmClient, RigClient};

#[tokio::main]
async fn main() {
    // Initialize clients
    let llm_client = LlmClient::new("http://localhost:4000", "test-key");
    
    #[cfg(feature = "rig")]
    let rig_client = RigClient::from_env().unwrap();
    
    let router = ModelRouter::new(llm_client, rig_client);
    
    // Chat completion (LiteLLM)
    let chat_response = router.route::<String>("chat", "Tell me a joke").await.unwrap();
    println!("Chat: {}", chat_response);
    
    // Structured classification (Rig)
    #[cfg(feature = "rig")]
    {
        let classification = router.route::<TaskClassification>(
            "classification",
            "Write unit tests for transport crate"
        ).await.unwrap();
        
        println!("Loop type: {:?}", classification.loop_type);
        println!("Confidence: {:.2}", classification.confidence);
    }
}
```

**Expected Output:**
```
Chat: Why don't programmers like nature? Too many bugs.

Loop type: Testers
Confidence: 0.95
```

---

## Appendix B: Related Documents

- `docs/superpowers/specs/2026-06-27-rig-integration-design.md` — Original Rig.rs integration design
- `docs/superpowers/specs/2026-06-27-tailscale-integration-design.md` — Tailscale mesh VPN design
- `docs/superpowers/plans/2026-06-27-tailscale-integration-plan.md` — Tailscale implementation plan

(End of file)
