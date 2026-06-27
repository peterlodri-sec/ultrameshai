use loop_engineering_cognition::client::LlmClient;
use loop_engineering_cognition::error::CognitionError;

#[tokio::test]
async fn test_chat_basic() {
    let client = LlmClient::mock("test-model");
    let messages = vec![];
    
    let result = client.chat(messages).await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(!response.content.is_empty());
}

#[tokio::test]
async fn test_chat_with_tools() {
    let client = LlmClient::mock("test-model");
    let messages = vec![];
    let tools = vec![];
    
    let result = client.chat_with_tools(messages, tools).await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(!response.content.is_empty());
}

#[test]
fn test_model_id_parsing() {
    assert_eq!(LlmClient::parse_provider("openai/gpt-4"), "openai".to_string());
    assert_eq!(LlmClient::parse_provider("anthropic/claude-3"), "anthropic".to_string());
    assert_eq!(LlmClient::parse_provider("gpt-4"), "unknown".to_string());
}
