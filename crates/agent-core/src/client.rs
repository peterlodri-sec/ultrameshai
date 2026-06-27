//! LLM client — wraps adk-model OpenAICompatible (DashScope/Qwen).

use crate::dashscope::dashscope_llm;
use crate::error::{AgentError, Result};
use adk_core::{Content, Llm, LlmRequest};
use adk_model::openai_compatible::{OpenAICompatible, OpenAICompatibleConfig};

/// Agent LLM client using adk-model.
pub struct LlmClient {
    llm: OpenAICompatible,
    model_name: String,
}

impl LlmClient {
    /// Create new client using DashScope (reads DASHSCOPE_API_KEY, DASHSCOPE_API_BASE env vars).
    pub fn new() -> Result<Self> {
        let llm = dashscope_llm().map_err(|e| AgentError::ApiError(e.to_string()))?;
        Ok(Self { llm, model_name: "qwen-turbo".to_string() })
    }

    /// Create a new client with custom configuration (model, api_key, base_url).
    pub fn with_config(model_name: &str, api_key: &str, base_url: &str) -> Result<Self> {
        let endpoint = if base_url.ends_with("/chat/completions") {
            base_url.to_string()
        } else {
            format!("{base_url}/chat/completions")
        };
        let config = OpenAICompatibleConfig::new(api_key.to_string(), model_name.to_string())
            .with_base_url(endpoint)
            .with_provider_name("openai-compatible");
        let llm = OpenAICompatible::new(config).map_err(|e| AgentError::ApiError(e.to_string()))?;
        Ok(Self { llm, model_name: model_name.to_string() })
    }

    /// Send a chat completion and return the response text (non-streaming).
    pub async fn chat(&self, messages: &[(&str, &str)]) -> Result<String> {
        let contents: Vec<Content> = messages
            .iter()
            .map(|(role, content)| Content::new(*role).with_text(*content))
            .collect();

        let request = LlmRequest::new(&self.model_name, contents);

        let mut stream = self.llm.generate_content(request, false).await
            .map_err(|e| AgentError::ApiError(e.to_string()))?;

        // For non-streaming, expect exactly one LlmResponse
        use futures::StreamExt;
        let response = stream.next().await
            .ok_or_else(|| AgentError::ApiError("no response from model".into()))?
            .map_err(|e| AgentError::ApiError(e.to_string()))?;

        // Extract text from response content
        let text = response
            .content
            .as_ref()
            .and_then(|c| c.parts.iter().find_map(|p| p.text()))
            .unwrap_or_default()
            .to_string();
        Ok(text)
    }
}

