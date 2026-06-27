use crate::traits::{Loop, LoopInput, LoopOutput, LoopStats, Result, LoopError};
use honcho::LearningPattern;
use loop_engineering_cognition::{LlmClient, Session, PromptDispatcher, ModelRouter};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

/// Execution mode for a slice
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Sequential execution with dependencies
    Pipeline,
    /// Parallel fan-out for independent slices
    Wave,
}

/// E2E slice representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct E2ESlice {
    pub slice_id: String,
    pub task_id: String,
    pub loop_type: String,
    pub spec: String,
    pub dependencies: Vec<String>,
    pub execution_mode: ExecutionMode,
}

/// Task decomposition result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDecomposition {
    pub task_id: String,
    pub slices: Vec<E2ESlice>,
    pub recommended_mode: ExecutionMode,
}

/// Slice graph for dependency tracking
pub struct SliceGraph {
    slices: HashMap<String, E2ESlice>,
    dependencies: HashMap<String, HashSet<String>>,
    resolved: HashSet<String>,
}

impl SliceGraph {
    pub fn new() -> Self {
        Self {
            slices: HashMap::new(),
            dependencies: HashMap::new(),
            resolved: HashSet::new(),
        }
    }

    pub fn add_slice(&mut self, slice: E2ESlice) {
        self.slices.insert(slice.slice_id.clone(), slice.clone());
        self.dependencies
            .entry(slice.slice_id.clone())
            .or_insert_with(|| slice.dependencies.into_iter().collect());
    }

    pub fn get_ready_slices(&self) -> Vec<&E2ESlice> {
        self.slices
            .values()
            .filter(|s| {
                self.dependencies
                    .get(&s.slice_id)
                    .map(|deps| deps.is_subset(&self.resolved))
                    .unwrap_or(true)
                    && !self.resolved.contains(&s.slice_id)
            })
            .collect()
    }

    pub fn mark_resolved(&mut self, slice_id: &str) {
        self.resolved.insert(slice_id.to_string());
    }

    pub fn detect_cycles(&self) -> bool {
        // Simple cycle detection via DFS
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for slice_id in self.slices.keys() {
            if self.has_cycle(slice_id, &mut visited, &mut rec_stack) {
                return true;
            }
        }
        false
    }

    fn has_cycle(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        if !visited.contains(node) {
            visited.insert(node.to_string());
            rec_stack.insert(node.to_string());

            if let Some(neighbors) = self.dependencies.get(node) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        if self.has_cycle(neighbor, visited, rec_stack) {
                            return true;
                        }
                    } else if rec_stack.contains(neighbor) {
                        return true;
                    }
                }
            }
        }

        rec_stack.remove(node);
        false
    }
}

impl Default for SliceGraph {
    fn default() -> Self {
        Self::new()
    }
}

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
    slice_graph: Arc<RwLock<SliceGraph>>,
}

impl YardmasterLoop {
    pub fn new() -> Self {
        let router = ModelRouter::default();
        // Use a valid loop type for the client, fallback to mock if needed
        let client = router.create_client("yardmaster", "mock-key", "http://localhost")
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
            slice_graph: Arc::new(RwLock::new(SliceGraph::new())),
        }
    }

    /// Decompose task into E2E slices
    pub async fn decompose_task(&self, task_id: &str, task_desc: &str) -> Result<TaskDecomposition> {
        // In production, would use LLM to decompose task
        // For now, use heuristic decomposition based on task description
        
        let slices = vec![
            E2ESlice {
                slice_id: format!("{}-slice-1", task_id),
                task_id: task_id.to_string(),
                loop_type: "deepwork".into(),
                spec: "Analyze requirements and create plan".into(),
                dependencies: vec![],
                execution_mode: ExecutionMode::Pipeline,
            },
            E2ESlice {
                slice_id: format!("{}-slice-2", task_id),
                task_id: task_id.to_string(),
                loop_type: "deep-research".into(),
                spec: "Research relevant patterns and APIs".into(),
                dependencies: vec![format!("{}-slice-1", task_id)],
                execution_mode: ExecutionMode::Pipeline,
            },
            E2ESlice {
                slice_id: format!("{}-slice-3", task_id),
                task_id: task_id.to_string(),
                loop_type: "coder".into(),
                spec: "Implement solution".into(),
                dependencies: vec![format!("{}-slice-2", task_id)],
                execution_mode: ExecutionMode::Pipeline,
            },
            E2ESlice {
                slice_id: format!("{}-slice-4", task_id),
                task_id: task_id.to_string(),
                loop_type: "tester".into(),
                spec: "Write and run tests".into(),
                dependencies: vec![format!("{}-slice-3", task_id)],
                execution_mode: ExecutionMode::Pipeline,
            },
        ];

        // Determine recommended mode based on dependencies
        let has_dependencies = slices.iter().any(|s| !s.dependencies.is_empty());
        let recommended_mode = if has_dependencies {
            ExecutionMode::Pipeline
        } else {
            ExecutionMode::Wave
        };

        Ok(TaskDecomposition {
            task_id: task_id.to_string(),
            slices,
            recommended_mode,
        })
    }

    /// Select execution mode based on slice graph
    pub async fn select_execution_mode(&self, slices: &[E2ESlice]) -> ExecutionMode {
        // Build dependency graph
        let mut graph = SliceGraph::new();
        for slice in slices {
            graph.add_slice(slice.clone());
        }

        // Check for cycles
        if graph.detect_cycles() {
            // Fall back to pipeline if cycles detected
            return ExecutionMode::Pipeline;
        }

        // Count slices with dependencies
        let dependent_count = slices.iter().filter(|s| !s.dependencies.is_empty()).count();
        let total = slices.len();

        // If >50% have dependencies, use pipeline; otherwise wave
        if dependent_count > total / 2 {
            ExecutionMode::Pipeline
        } else {
            ExecutionMode::Wave
        }
    }

    /// Query honcho for patterns and adjust slicing strategy
    pub async fn query_honcho_patterns(&mut self, honcho_db: &str) -> Result<()> {
        let patterns = self.mock_honcho_patterns();
        self.apply_patterns_to_strategy(patterns).await;
        Ok(())
    }

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

    async fn apply_patterns_to_strategy(&mut self, patterns: Vec<LearningPattern>) {
        let mut honcho_patterns = self.honcho_patterns.write().unwrap();
        *honcho_patterns = patterns.clone();

        for pattern in &patterns {
            match pattern.pattern_type.as_str() {
                "performance" => {
                    if pattern.confidence >= 0.8 {
                        if let Some(metadata) = pattern.metadata.as_object() {
                            if let Some(timeout) = metadata.get("timeout_ms").and_then(|v| v.as_u64()) {
                                self.strategy.timeout_ms = timeout;
                            }
                        }
                    }
                }
                "failure" => {
                    if pattern.confidence >= 0.8 {
                        for loop_type in &pattern.affected_loops {
                            if !self.strategy.avoided_loops.contains(loop_type) {
                                self.strategy.avoided_loops.push(loop_type.clone());
                            }
                        }
                    }
                }
                "success" => {
                    if pattern.confidence >= 0.8 {
                        for loop_type in &pattern.affected_loops {
                            if !self.strategy.preferred_loops.contains(loop_type) {
                                self.strategy.preferred_loops.push(loop_type.clone());
                            }
                        }
                    }
                }
                "cross-loop" => {
                    if pattern.confidence >= 0.5 {
                        self.strategy.pipeline_order = pattern.affected_loops.clone();
                    }
                }
                _ => {}
            }
        }
    }

    /// Add slices to the slice graph
    pub async fn add_slices_to_graph(&self, slices: Vec<E2ESlice>) {
        let mut graph = self.slice_graph.write().unwrap();
        for slice in slices {
            graph.add_slice(slice);
        }
    }

    /// Get ready slices (dependencies resolved)
    pub async fn get_ready_slices(&self) -> Vec<E2ESlice> {
        let graph = self.slice_graph.read().unwrap();
        graph.get_ready_slices().into_iter().cloned().collect()
    }

    /// Mark slice as resolved
    pub async fn mark_slice_resolved(&self, slice_id: &str) {
        let mut graph = self.slice_graph.write().unwrap();
        graph.mark_resolved(slice_id);
    }

    /// Get current slicing strategy
    pub fn get_strategy(&self) -> &SlicingStrategy {
        &self.strategy
    }

    /// Get honcho patterns
    pub async fn get_honcho_patterns(&self) -> Vec<LearningPattern> {
        self.honcho_patterns.read().unwrap().clone()
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
