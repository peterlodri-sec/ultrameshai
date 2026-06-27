//! Tests for agent-core LLM client with DashScope/Qwen.

use agent_core::client::LlmClient;
use agent_core::error::AgentError;

#[tokio::test]
async fn llm_client_chat() -> Result<(), AgentError> {
    // Requires DASHSCOPE_API_KEY env var
    let client = match LlmClient::new() {
        Ok(c) => c,
        Err(AgentError::ApiError(_)) => return Ok(()), // skip if no API key
        Err(e) => return Err(e),
    };

    let response = client
        .chat(&[
            ("system", "You are a helpful assistant."),
            ("user", "What is 2+2?"),
        ])
        .await?;

    assert!(!response.is_empty(), "response should not be empty");
    Ok(())
}
