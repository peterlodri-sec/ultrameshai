use crate::traits::{Loop, LoopInput, LoopOutput, LoopStats, Result, LoopError};
use honcho::{HonchoDaemon, LearningPattern};
use loop_engineering_cognition::{LlmClient, Session, PromptDispatcher, ModelRouter};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SlicingStrategy {
    pub pipeline_order: Vec<String>,
    pub timeout_ms: u64,
    pub preferred_loops: Vec<String>,
    pub avoided_loops: Vec<String>,
}

impl Default for SlicingStrategy {
    fn default() -> Self {
        Self {
            pipeline_order: vec!["deepwork".into(), "coder".into(), "tester".into()],
            timeout_ms: 300000,
            preferred_loops: vec![],
            avoided_loops: vec![],
        }
    }
}

pub struct YardmasterLoop {
    client: LlmClient,
    session: Session,
    dispatcher: PromptDispatcher,
    stats: LoopStats,
    honcho_patterns: Arc<RwLock<Vec<LearningPattern>>>,
    strategy: SlicingStrategy,
}

impl YardmasterLoop {
    pub fn new() -> Self {
        let router = ModelRouter::default();
        let client = router.create_client("coordinator", "mock-key", "http://localhost")
            .unwrap_or_else(|| LlmClient::mock("anthropic/claude-3-5-sonnet"));
        let session = Session::new("yardmaster-loop", "unit-000");
        let dispatcher = PromptDispatcher::default();
        Self {
            client,
            session,
            dispatcher,
            stats: LoopStats::default(),
            honcho_patterns: Arc::new(RwLock::new(vec![])),
            strategy: SlicingStrategy::default(),
        }
    }

    /// Query honcho for patterns and adjust slicing strategy
    pub async fn query_honcho_patterns(&mut self, honcho_db: &str) -> Result<()> {
        // In production, would connect to honcho daemon or PatternStore
        // For now, use mock patterns for testing
        let patterns = self.mock_honcho_patterns();
        
        self.apply_patterns_to_strategy(patterns).await;
        Ok(())
    }

    /// Mock honcho patterns for testing
    fn mock_honcho_patterns(&self) -> Vec<LearningPattern> {
        vec![
            LearningPattern::new(
                "performance",
                0.85,
                "Coder loops timeout after 5min",
                vec!["coder".into()],
            )
            .with_metadata(serde_json::json!({"timeout_ms": 600000})),
            LearningPattern::new(
                "failure",
                0.75,
                "Red-team loops have high failure rate",
                vec!["red-team".into()],
            ),
            LearningPattern::new(
                "success",
                0.9,
                "Deepwork loops succeed with async patterns",
                vec!["deepwork".into()],
            ),
        ]
    }

    /// Apply honcho patterns to slicing strategy
    async fn apply_patterns_to_strategy(&mut self, patterns: Vec<LearningPattern>) {
        let mut honcho_patterns = self.honcho_patterns.write().await;
        *honcho_patterns = patterns.clone();

        for pattern in &patterns {
            match pattern.pattern_type.as_str() {
                "performance" => {
                    // High confidence performance patterns adjust timeout
                    if pattern.confidence >= 0.8 {
                        if let Some(metadata) = pattern.metadata.as_object() {
                            if let Some(timeout) = metadata.get("timeout_ms").and_then(|v| v.as_u64()) {
                                self.strategy.timeout_ms = timeout;
                            }
                        }
                    }
                }
                "failure" => {
                    // High confidence failure patterns avoid certain loops
                    if pattern.confidence >= 0.8 {
                        for loop_type in &pattern.affected_loops {
                            if !self.strategy.avoided_loops.contains(loop_type) {
                                self.strategy.avoided_loops.push(loop_type.clone());
                            }
                        }
                    }
                }
                "success" => {
                    // High confidence success patterns prefer certain loops
                    if pattern.confidence >= 0.8 {
                        for loop_type in &pattern.affected_loops {
                            if !self.strategy.preferred_loops.contains(loop_type) {
                                self.strategy.preferred_loops.push(loop_type.clone());
                            }
                        }
                    }
                }
                "cross-loop" => {
                    // Cross-loop patterns adjust pipeline order
                    if pattern.confidence >= 0.5 {
                        self.strategy.pipeline_order = pattern.affected_loops.clone();
                    }
                }
                _ => {}
            }
        }
    }

    /// Get current slicing strategy (adjusted by honcho patterns)
    pub fn get_strategy(&self) -> &SlicingStrategy {
        &self.strategy
    }

    /// Get honcho patterns
    pub async fn get_honcho_patterns(&self) -> Vec<LearningPattern> {
        self.honcho_patterns.read().await.clone()
    }
}

impl Default for YardmasterLoop {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Loop for YardmasterLoop {
    fn loop_type(&self) -> &str {
        "yardmaster-loop"
    }

    async fn process(&mut self, input: LoopInput) -> Result<LoopOutput> {
        let mut variables = HashMap::new();
        variables.insert("task".to_string(), input.task_desc.clone());
        
        let prompt = self.dispatcher
            .dispatch("coordinator", &variables)
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
