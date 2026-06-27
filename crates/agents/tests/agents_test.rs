use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use loop_engineering_agents::{BaseAgent, AgentContext, AgentResponse, AgentError};
use loop_engineering_agents::{LoopAgent, SequentialAgent, ConditionalAgent, LlmConditionalAgent};
use loop_engineering_cognition::LlmClient;

// A simple mock agent for testing pipelines.
struct IncrementAgent {
    name: String,
    counter: Arc<AtomicUsize>,
}

impl IncrementAgent {
    fn new(name: &str) -> (Self, Arc<AtomicUsize>) {
        let counter = Arc::new(AtomicUsize::new(0));
        (
            Self {
                name: name.to_string(),
                counter: counter.clone(),
            },
            counter,
        )
    }
}

#[async_trait]
impl BaseAgent for IncrementAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Increments an atomic counter in the agent struct"
    }

    async fn execute(&self, input: &str, _context: &mut AgentContext) -> Result<AgentResponse, AgentError> {
        let prev = self.counter.fetch_add(1, Ordering::SeqCst);
        Ok(AgentResponse::new(&format!("{}: prev={}", input, prev)))
    }
}

// A mock agent that modifies context state.
struct ContextModifyingAgent {
    name: String,
    key: String,
    value: String,
}

impl ContextModifyingAgent {
    fn new(name: &str, key: &str, value: &str) -> Self {
        Self {
            name: name.to_string(),
            key: key.to_string(),
            value: value.to_string(),
        }
    }
}

#[async_trait]
impl BaseAgent for ContextModifyingAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Modifies the execution context"
    }

    async fn execute(&self, input: &str, context: &mut AgentContext) -> Result<AgentResponse, AgentError> {
        context.set(&self.key, self.value.clone());
        Ok(AgentResponse::new(input))
    }
}

#[tokio::test]
async fn test_sequential_agent() {
    let (agent_a, count_a) = IncrementAgent::new("AgentA");
    let (agent_b, count_b) = IncrementAgent::new("AgentB");

    let pipeline = SequentialAgent::new(
        "SequentialPipeline",
        vec![Arc::new(agent_a), Arc::new(agent_b)],
    );

    let mut context = AgentContext::new("test-session");
    let response = pipeline.execute("start", &mut context).await.unwrap();

    assert_eq!(response.content, "start: prev=0: prev=0");
    assert_eq!(count_a.load(Ordering::SeqCst), 1);
    assert_eq!(count_b.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_loop_agent() {
    let (agent, count) = IncrementAgent::new("LoopWorker");
    let loop_agent = LoopAgent::new("LoopExecutor", vec![Arc::new(agent)])
        .with_max_iterations(3);

    let mut context = AgentContext::new("test-session");
    let response = loop_agent.execute("init", &mut context).await.unwrap();

    // Since the IncrementAgent returns done: true, it will exit early if iteration > 1
    // Let's verify that it ran exactly 2 iterations (since the loop exits early if response.done && iteration > 1)
    assert_eq!(count.load(Ordering::SeqCst), 2);
    assert!(response.done);
}

#[tokio::test]
async fn test_conditional_agent() {
    let true_agent = Arc::new(ContextModifyingAgent::new("TrueAgent", "flag", "true_path"));
    let false_agent = Arc::new(ContextModifyingAgent::new("FalseAgent", "flag", "false_path"));

    let condition = |ctx: &AgentContext| {
        ctx.get("trigger").map(|v| v == "yes").unwrap_or(false)
    };

    let router = ConditionalAgent::new("Router", condition, true_agent, false_agent);

    // Test false branch
    let mut context = AgentContext::new("session-1");
    router.execute("run", &mut context).await.unwrap();
    assert_eq!(context.get("flag").unwrap(), "false_path");

    // Test true branch
    let mut context = AgentContext::new("session-2");
    context.set("trigger", "yes".to_string());
    router.execute("run", &mut context).await.unwrap();
    assert_eq!(context.get("flag").unwrap(), "true_path");
}

#[tokio::test]
async fn test_llm_conditional_agent() {
    let mock_client = LlmClient::mock("anthropic/claude-3-5-sonnet");
    let target_agent = Arc::new(ContextModifyingAgent::new("TargetAgent", "routed", "success"));

    // The mock LLM client always returns "mock response".
    // We map the route for "mock response" to our target agent.
    let llm_router = LlmConditionalAgent::new(
        "LlmRouter",
        mock_client,
        "Classify input",
    )
    .route("mock response", target_agent);

    let mut context = AgentContext::new("session-llm");
    llm_router.execute("query", &mut context).await.unwrap();

    assert_eq!(context.get("routed").unwrap(), "success");
}
