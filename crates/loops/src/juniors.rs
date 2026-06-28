use crate::traits::{Loop, LoopInput, LoopOutput, LoopStats, Result};
use honcho::LearningPattern;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct JuniorBurstRecommendation {
    pub should_spawn: bool,
    pub recommended_approach: Option<String>,
    pub warnings: Vec<String>,
}

pub struct JuniorsLoop {
    stats: LoopStats,
    honcho_patterns: Arc<RwLock<Vec<LearningPattern>>>,
}

impl JuniorsLoop {
    pub fn new() -> Self {
        Self {
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
        self.stats.slices_processed += 1;
        
        Ok(LoopOutput {
            slice_id: input.slice_id,
            result: input.task_desc,
            tool_calls: vec![],
            stats: self.stats.clone(),
        })
    }

    fn stats(&self) -> LoopStats {
        self.stats.clone()
    }
}
