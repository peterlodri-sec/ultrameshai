use crate::traits::{Loop, LoopInput, LoopOutput, LoopStats};
use crate::error::{Result};
use honcho::LearningPattern;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct DeepResearchLoop {
    stats: LoopStats,
    honcho_patterns: Arc<RwLock<Vec<LearningPattern>>>,
}

impl DeepResearchLoop {
    pub fn new() -> Self {
        Self {
            stats: LoopStats::default(),
            honcho_patterns: Arc::new(RwLock::new(vec![])),
        }
    }

    /// Query honcho patterns mid-execution when hitting uncertainty
    pub async fn query_patterns_mid_execution(&mut self, topic: &str) -> Result<Vec<LearningPattern>> {
        // In production, would query PatternStore by topic similarity
        // For now, return cached patterns filtered by topic
        let patterns = self.honcho_patterns.read().await.clone();
        
        // Filter patterns relevant to current topic
        let relevant: Vec<_> = patterns
            .into_iter()
            .filter(|p| p.summary.to_lowercase().contains(&topic.to_lowercase()))
            .collect();

        Ok(relevant)
    }

    /// Load honcho patterns (called at loop startup)
    pub async fn load_honcho_patterns(&mut self, patterns: Vec<LearningPattern>) {
        let mut honcho_patterns = self.honcho_patterns.write().await;
        *honcho_patterns = patterns;
    }

    /// Get recommendations based on patterns
    pub async fn get_recommendations(&self, topic: &str) -> Vec<String> {
        let patterns = self.honcho_patterns.read().await.clone();
        let mut recommendations = Vec::new();

        for pattern in &patterns {
            if pattern.summary.to_lowercase().contains(&topic.to_lowercase()) {
                match pattern.pattern_type.as_str() {
                    "success" => {
                        recommendations.push(format!(
                            "Recommended: {} (confidence: {:.2})",
                            pattern.summary, pattern.confidence
                        ));
                    }
                    "failure" => {
                        recommendations.push(format!(
                            "Avoid: {} (confidence: {:.2})",
                            pattern.summary, pattern.confidence
                        ));
                    }
                    "performance" => {
                        recommendations.push(format!(
                            "Performance tip: {} (confidence: {:.2})",
                            pattern.summary, pattern.confidence
                        ));
                    }
                    _ => {}
                }
            }
        }

        recommendations
    }
}

impl Default for DeepResearchLoop {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Loop for DeepResearchLoop {
    fn loop_type(&self) -> &str {
        "deep-research-loop"
    }

    async fn process(&mut self, input: LoopInput) -> Result<LoopOutput> {
        self.stats.slices_processed += 1;
        
        Ok(LoopOutput {
            slice_id: input.slice_id,
            result: input.task_desc,
            tool_calls: vec![],
            stats: self.stats.clone(),
            reward_earned: None,
            a2a_completed: false,
        })
    }

    fn stats(&self) -> LoopStats {
        self.stats.clone()
    }
}
