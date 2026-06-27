//! LLM client — wraps adk-model OpenAICompatible (DashScope/Qwen).

use crate::dashscope::dashscope_llm;
use crate::error::{AgentError, Result};
use adk_core::{Content, Llm, LlmRequest};
use adk_model::openai_compatible::OpenAICompatible;

/// Agent LLM client using adk-model.
pub struct LlmClient {
    llm: OpenAICompatible,
}

impl LlmClient {
    /// Create new client using DashScope (reads DASHSCOPE_API_KEY, DASHSCOPE_API_BASE env vars).
    pub fn new() -> Result<Self> {
        let llm = dashscope_llm().map_err(|e| AgentError::ApiError(e.to_string()))?;
        Ok(Self { llm })
    }

    /// Send a chat completion and return the response text (non-streaming).
    pub async fn chat(&self, messages: &[(&str, &str)]) -> Result<String> {
        let contents: Vec<Content> = messages
            .iter()
            .map(|(role, content)| Content::new(*role).with_text(*content))
            .collect();

        let request = LlmRequest::new("qwen-turbo", contents);
        let mut stream = self.llm.generate_content(request, false).await
            .map_err(|e| AgentError::ApiError(e.to_string()))?;

        // For non-streaming, expect exactly one LlmResponse
        use futures::StreamExt;
        let response = stream.next().await
            .ok_or_else(|| AgentError::ApiError("no response from model".into()))?
            .map_err(|e| AgentError::ApiError(e.to_string()))?;

        // Extract text from response content
        let text = response.content.as_ref().and_then(|c| c.text()).unwrap_or_default().to_string();
        Ok(text)
    }
}
