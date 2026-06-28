use std::collections::HashMap;
use crate::client::LlmClient;

/// Model tier labels for OVHcloud AI Endpoints.
/// Uses virtual model queries (`tag@ranker?conditions`) that auto-resolve
/// to the best available model.
///
/// Tiers and loop mappings are loaded from `config/models.toml` at compile time.
/// Base URL: https://oai.endpoints.kepler.ai.cloud.ovh.net/v1
pub struct ModelRouter {
    default_models: HashMap<String, String>,
    custom_models: HashMap<String, String>,
}

#[derive(serde::Deserialize)]
struct ModelsConfig {
    tiers: HashMap<String, String>,
    loops: HashMap<String, String>,
}

impl ModelRouter {
    pub fn new() -> Self {
        let config: ModelsConfig = toml::from_str(include_str!("../config/models.toml"))
            .expect("Failed to parse config/models.toml");
        let mut m = HashMap::new();
        for (loop_type, tier_name) in &config.loops {
            let model = config.tiers.get(tier_name.as_str())
                .unwrap_or_else(|| panic!("Model tier '{tier_name}' referenced by loop '{loop_type}' not found in [tiers]"));
            m.insert(loop_type.clone(), model.clone());
        }
        Self {
            default_models: m,
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
        let model = self.default_models.get("deepwork").expect("deepwork model missing from defaults");
        LlmClient::new(model, api_key, base_url)
    }

    pub fn create_client_for_bruteforce_coder(&self, api_key: &str, base_url: &str) -> LlmClient {
        let model = self.default_models.get("bruteforce-coder").expect("bruteforce-coder model missing from defaults");
        LlmClient::new(model, api_key, base_url)
    }

    pub fn create_client_for_juniors(&self, api_key: &str, base_url: &str) -> LlmClient {
        let model = self.default_models.get("juniors").expect("juniors model missing from defaults");
        LlmClient::new(model, api_key, base_url)
    }

    pub fn all_loops_mapped(&self) -> bool {
        let loops = [
            "deepwork", "bruteforce-coder", "deep-research", "testers",
            "yardmaster", "devops", "ui", "red-team", "juniors", "ralph",
            "coder-planner", "coder-editor", "coder-reviewer",
            "deepwork-decomposer", "deepwork-verifier",
            "redteam-fuzzer", "redteam-analyzer",
            "librarian",
            "research-web", "research-docs", "research-patterns",
            "issue-analyzer", "codebase-explorer", "fix-planner",
            "fix-implementer", "edge-case-analyzer", "regression-checker",
            "diff-builder",
            "tester-unit", "tester-integration", "tester-benchmark",
            "tester-property", "tester-mutation",
            "devops-build", "devops-package", "devops-deploy", "devops-cache",
            "quality-lint", "quality-audit", "quality-coverage",
            "quality-typecheck", "quality-style",
            "memory-indexer", "memory-pattern-miner", "memory-summarizer",
            "ralph-coder", "ralph-research", "ralph-meta",
            "infra-provisioner", "infra-monitor", "infra-balancer",
            "reporter", "validator", "notebook",
            "math-solver", "math-verify",
        ];
        loops.iter().all(|l| self.default_models.contains_key(*l))
    }
}

impl Default for ModelRouter {
    fn default() -> Self {
        Self::new()
    }
}
