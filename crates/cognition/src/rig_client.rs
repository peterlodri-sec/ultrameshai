#[cfg(feature = "rig")]
use crate::error::CognitionError;
#[cfg(feature = "rig")]
use rig::{providers::openai, extractor::Extractor};
#[cfg(feature = "rig")]
use serde::{Deserialize, Serialize, de::DeserializeOwned};
#[cfg(feature = "rig")]
use schemars::JsonSchema;

/// RigClient wrapper for structured extraction using OVHcloud AI
#[cfg(feature = "rig")]
pub struct RigClient {
    client: openai::Client,
    default_model: String,
}

#[cfg(feature = "rig")]
impl RigClient {
    /// Create new RigClient from environment variables
    /// 
    /// Required env vars:
    /// - OVHCLOUD_AI_API_KEY: Your OVHcloud AI API key
    /// 
    /// Optional env vars:
    /// - OVHCLOUD_AI_BASE_URL: Defaults to OVHcloud AI Endpoints
    /// - OVHCLOUD_AI_MODEL: Defaults to Meta-Llama-3_3-70B-Instruct
    pub fn from_env() -> std::result::Result<Self, CognitionError> {
        let api_key = std::env::var("OVHCLOUD_AI_API_KEY")
            .map_err(|_| CognitionError::Rig("OVHCLOUD_AI_API_KEY not set".to_string()))?;
        
        let base_url = std::env::var("OVHCLOUD_AI_BASE_URL")
            .unwrap_or_else(|_| "https://oai.endpoints.kepler.ai.cloud.ovh.net/v1".to_string());
        
        let client = openai::Client::from_url(&api_key, &base_url);
        
        let default_model = std::env::var("OVHCLOUD_AI_MODEL")
            .unwrap_or_else(|_| "Meta-Llama-3_3-70B-Instruct".to_string());
        
        Ok(Self {
            client,
            default_model,
        })
    }

    /// Set default model for extraction
    pub fn with_model(mut self, model: &str) -> Self {
        self.default_model = model.to_string();
        self
    }

    /// Create typed extractor for structured output
    pub fn extractor<T: Serialize + DeserializeOwned + JsonSchema + Send + Sync + 'static>(
        &self,
        preamble: &str,
    ) -> std::result::Result<RigExtractor<T>, CognitionError> {
        let extractor = self.client
            .extractor::<T>(&self.default_model)
            .preamble(preamble)
            .build();
        
        Ok(RigExtractor { extractor })
    }
}

/// Wrapper for Rig's Extractor
#[cfg(feature = "rig")]
pub struct RigExtractor<T> 
where
    T: Serialize + DeserializeOwned + JsonSchema + Send + Sync,
{
    extractor: Extractor<openai::CompletionModel, T>,
}

#[cfg(feature = "rig")]
impl<T> RigExtractor<T>
where
    T: Serialize + DeserializeOwned + JsonSchema + Send + Sync,
{
    /// Extract structured data from text
    pub async fn extract(&self, text: &str) -> std::result::Result<T, CognitionError> {
        self.extractor
            .extract(text)
            .await
            .map_err(|e| CognitionError::Rig(e.to_string()))
    }
}