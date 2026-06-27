//! DashScope provider via adk-model OpenAICompatible.
//! Endpoint: https://dashscope.aliyuncs.com/api/v1
//! Auth: DASHSCOPE_API_KEY env var.

use adk_model::openai_compatible::{OpenAICompatible, OpenAICompatibleConfig};

/// Build a DashScope LLM using adk-model's OpenAICompatible preset.
pub fn dashscope_llm() -> Result<OpenAICompatible, adk_core::AdkError> {
    let api_key = std::env::var("DASHSCOPE_API_KEY")
        .expect("DASHSCOPE_API_KEY must be set");
    let base_url = std::env::var("DASHSCOPE_API_BASE")
        .unwrap_or_else(|_| "https://dashscope.aliyuncs.com/api/v1".into());

    // DashScope uses /chat/completions like standard OpenAI-compatible APIs
    let endpoint = format!("{base_url}/chat/completions");
    let config = OpenAICompatibleConfig::new(api_key, "qwen-turbo")
        .with_base_url(endpoint)
        .with_provider_name("dashscope");

    OpenAICompatible::new(config)
}
