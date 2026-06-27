use crate::error::{CognitionError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

impl Role {
    pub fn as_str(&self) -> &str {
        match self {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
        }
    }
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

        let api_key = self.api_key.as_deref().unwrap_or("");
        let base_url = self.base_url.as_deref().unwrap_or("https://dashscope.aliyuncs.com/api/v1");

        let core_client = agent_core::client::LlmClient::with_config(
            &self.model_id,
            api_key,
            base_url,
        ).map_err(|e| CognitionError::LlmApi(format!("Failed to create agent-core client: {}", e)))?;

        let core_messages: Vec<(String, String)> = messages
            .iter()
            .map(|m| (m.role.as_str().to_string(), m.content.clone()))
            .collect();

        let refs: Vec<(&str, &str)> = core_messages
            .iter()
            .map(|(r, c)| (r.as_str(), c.as_str()))
            .collect();

        let response = core_client.chat(&refs).await
            .map_err(|e| CognitionError::LlmApi(e.to_string()))?;

        Ok(ChatResponse {
            content: response,
            model: self.model_id.clone(),
            usage: None,
        })
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
