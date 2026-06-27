use crate::traits::{Loop, LoopInput, LoopOutput, LoopStats, Result, LoopError};
use honcho::LearningPattern;
use loop_engineering_cognition::{LlmClient, Session, PromptDispatcher, ModelRouter};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct JuniorBurstRecommendation {
    pub should_spawn: bool,
    pub recommended_approach: Option<String>,
    pub warnings: Vec<String>,
}

pub struct JuniorsLoop {
    client: LlmClient,
    session: Session,
    dispatcher: PromptDispatcher,
    stats: LoopStats,
    honcho_patterns: Arc<RwLock<Vec<LearningPattern>>>,
}

impl JuniorsLoop {
    pub fn new() -> Self {
        let router = ModelRouter::default();
        let client = router.create_client_for_juniors("mock-key", "http://localhost");
        let session = Session::new("juniors-loop", "unit-000");
        let dispatcher = PromptDispatcher::default();
        Self {
            client,
            session,
            dispatcher,
            stats: LoopStats::default(),
            honcho_patterns: Arc::new(RwLock::new(vec![])),
        }
    }

    /// Check honcho patterns before spawning junior research burst
    pub async fn check_burst_recommendation(&mut self, task_topic: &str) -> JuniorBurstRecommendation {
        let patterns = self.honcho_patterns.read().await.clone();
        let mut recommendation = JuniorBurstRecommendation {
            should_spawn: true,
            recommended_approach: None,
            warnings: Vec::new(),
        };

        for pattern in &patterns {
            // Check if pattern is relevant to task topic
            let is_relevant = pattern.summary.to_lowercase().contains(&task_topic.to_lowercase())
                || pattern.affected_loops.iter().any(|l| l == "junior-burst");

            if !is_relevant {
                continue;
            }

            match pattern.pattern_type.as_str() {
                "success" if pattern.confidence >= 0.7 => {
                    // High confidence success pattern → recommend burst
                    recommendation.should_spawn = true;
                    recommendation.recommended_approach = Some(format!(
                        "Use junior burst: {} (confidence: {:.2})",
                        pattern.summary, pattern.confidence
                    ));
                }
                "failure" if pattern.confidence >= 0.7 => {
                    // High confidence failure pattern → warn or avoid
                    recommendation.warnings.push(format!(
                        "Caution: {} (confidence: {:.2})",
                        pattern.summary, pattern.confidence
                    ));
                    if pattern.confidence >= 0.9 {
                        recommendation.should_spawn = false;
                    }
                }
                "performance" if pattern.confidence >= 0.6 => {
                    // Performance pattern → adjust approach
                    recommendation.recommended_approach = Some(format!(
                        "Optimize: {} (confidence: {:.2})",
                        pattern.summary, pattern.confidence
                    ));
                }
                _ => {}
            }
        }

        recommendation
    }

    /// Load honcho patterns (called at loop startup)
    pub async fn load_honcho_patterns(&mut self, patterns: Vec<LearningPattern>) {
        let mut honcho_patterns = self.honcho_patterns.write().await;
        *honcho_patterns = patterns;
    }

    /// Get cached patterns
    pub async fn get_patterns(&self) -> Vec<LearningPattern> {
        self.honcho_patterns.read().await.clone()
    }
}

impl Default for JuniorsLoop {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Loop for JuniorsLoop {
    fn loop_type(&self) -> &str {
        "juniors-loop"
    }

    async fn process(&mut self, input: LoopInput) -> Result<LoopOutput> {
        let mut variables = HashMap::new();
        variables.insert("task".to_string(), input.task_desc.clone());
        
        let prompt = self.dispatcher
            .dispatch("coder", &variables)
            .unwrap_or_else(|| input.task_desc.clone());
        
        self.stats.slices_processed += 1;
        
        Ok(LoopOutput {
            slice_id: input.slice_id,
            result: prompt,
            tool_calls: vec![],
            stats: self.stats.clone(),
        })
    }

    fn stats(&self) -> LoopStats {
        self.stats.clone()
    }
}
