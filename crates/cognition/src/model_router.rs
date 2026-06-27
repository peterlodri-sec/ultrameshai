use std::collections::HashMap;
use crate::client::LlmClient;

pub struct ModelRouter {
    default_models: HashMap<String, String>,
    custom_models: HashMap<String, String>,
}

impl ModelRouter {
    pub fn new() -> Self {
        let mut default_models = HashMap::new();
        
        // Frontier tier (cloud)
        default_models.insert("deepwork".to_string(), "anthropic/claude-3-5-sonnet".to_string());
        default_models.insert("deep-research".to_string(), "openai/gpt-4o".to_string());
        default_models.insert("yardmaster".to_string(), "anthropic/claude-3-5-sonnet".to_string());
        default_models.insert("ui".to_string(), "openai/gpt-4o".to_string());
        
        // Local/cheap tier
        default_models.insert("bruteforce-coder".to_string(), "ollama/llama3.1:8b".to_string());
        default_models.insert("testers".to_string(), "ollama/codellama:7b".to_string());
        default_models.insert("devops".to_string(), "ollama/llama3.1:8b".to_string());
        
        // Red-team: local + frontier
        default_models.insert("red-team".to_string(), "ollama/codellama:13b".to_string());
        
        // Juniors: OSS 8-20B pool
        default_models.insert("juniors".to_string(), "ollama/llama-3.1-8b".to_string());
        
        // Ralph: small local model
        default_models.insert("ralph".to_string(), "ollama/phi-3:3.8b".to_string());
        
        Self {
            default_models,
            custom_models: HashMap::new(),
        }
    }
    
    pub fn get_model(&self, loop_type: &str) -> Option<&String> {
        self.custom_models.get(loop_type).or_else(|| self.default_models.get(loop_type))
    }
    
    pub fn register_custom(&mut self, loop_type: &str, model_id: &str) {
        self.custom_models.insert(loop_type.to_string(), model_id.to_string());
    }
    
    pub fn create_client(&self, loop_type: &str, api_key: &str, base_url: &str) -> Option<LlmClient> {
        self.get_model(loop_type).map(|model| {
            LlmClient::new(model, api_key, base_url)
        })
    }
    
    pub fn create_client_for_deepwork(&self, api_key: &str, base_url: &str) -> LlmClient {
        let model = self.default_models.get("deepwork").unwrap();
        LlmClient::new(model, api_key, base_url)
    }
    
    pub fn create_client_for_bruteforce_coder(&self, api_key: &str, base_url: &str) -> LlmClient {
        let model = self.default_models.get("bruteforce-coder").unwrap();
        LlmClient::new(model, api_key, base_url)
    }
    
    pub fn create_client_for_juniors(&self, api_key: &str, base_url: &str) -> LlmClient {
        let model = self.default_models.get("juniors").unwrap();
        LlmClient::new(model, api_key, base_url)
    }
    
    pub fn all_loops_mapped(&self) -> bool {
        let loops = ["deepwork", "bruteforce-coder", "deep-research", "testers", 
                     "yardmaster", "devops", "ui", "red-team", "juniors", "ralph"];
        loops.iter().all(|l| self.default_models.contains_key(*l))
    }
}

impl Default for ModelRouter {
    fn default() -> Self {
        Self::new()
    }
}
