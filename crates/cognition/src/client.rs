use crate::error::{CognitionError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

pub struct LlmClient {
    pub model_id: String,
    api_key: Option<String>,
    base_url: Option<String>,
    is_mock: bool,
}

impl LlmClient {
    pub fn new(model_id: &str, api_key: &str, base_url: &str) -> Self {
        Self {
            model_id: model_id.to_string(),
            api_key: Some(api_key.to_string()),
            base_url: Some(base_url.to_string()),
            is_mock: false,
        }
    }

    pub fn mock(model_id: &str) -> Self {
        Self {
            model_id: model_id.to_string(),
            api_key: None,
            base_url: None,
            is_mock: true,
        }
    }

    pub fn parse_provider(model_id: &str) -> String {
        if let Some(pos) = model_id.find('/') {
            model_id[..pos].to_string()
        } else {
            "unknown".to_string()
        }
    }

    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<ChatResponse> {
        if self.is_mock {
            return Ok(ChatResponse {
                content: "mock response".to_string(),
                model: self.model_id.clone(),
                usage: None,
            });
        }

        // Real implementation would call the LLM API
        // For now, return error if not mock
        Err(CognitionError::LlmApi("Real API not implemented".to_string()))
    }

    pub async fn chat_with_tools(
        &self,
        messages: Vec<ChatMessage>,
        _tools: Vec<ToolDefinition>,
    ) -> Result<ChatResponse> {
        if self.is_mock {
            return Ok(ChatResponse {
                content: "mock response with tools".to_string(),
                model: self.model_id.clone(),
                usage: None,
            });
        }

        Err(CognitionError::LlmApi("Real API not implemented".to_string()))
    }
}
