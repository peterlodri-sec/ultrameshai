//! Actor-Based Loop Lifecycle (P1)
//!
//! Provides a lightweight Actor pattern using Tokio channels for clean
//! supervisor-worker lifecycles with graceful cancellation and resource tracking.

use crate::traits::{Loop, LoopInput, LoopOutput, LoopStats};
use crate::error::{LoopError, Result};
use tokio::sync::{mpsc, oneshot};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Messages that can be sent to an agent actor
#[derive(Debug)]
pub enum ActorMessage {
    /// Process a single input slice
    Process {
        input: LoopInput,
        respond_to: oneshot::Sender<Result<LoopOutput>>,
    },
    /// Get current statistics
    Stats {
        respond_to: oneshot::Sender<LoopStats>,
    },
    /// Gracefully shutdown the actor
    Shutdown,
}

/// Actor state tracking
#[derive(Debug, Clone, PartialEq)]
pub enum ActorState {
    Idle,
    Processing,
    ShuttingDown,
    Terminated,
}

/// Agent actor that wraps a Loop implementation
pub struct AgentActor {
    /// The underlying loop implementation
    /// Note: We use a boxed trait object to allow dynamic dispatch
    /// This enables spawning different loop types as actors
    loop_impl: Box<dyn Loop + Send + 'static>,
    /// Channel for receiving messages
    rx: mpsc::Receiver<ActorMessage>,
    /// Current state
    state: ActorState,
    /// Actor ID for logging/tracking
    actor_id: String,
}

impl AgentActor {
    /// Create a new agent actor wrapping a Loop implementation
    pub fn new(
        loop_impl: Box<dyn Loop + Send + 'static>,
        rx: mpsc::Receiver<ActorMessage>,
        actor_id: String,
    ) -> Self {
        Self {
            loop_impl,
            rx,
            state: ActorState::Idle,
            actor_id,
        }
    }

    /// Get the actor's current state
    pub fn state(&self) -> &ActorState {
        &self.state
    }

    /// Get the actor's ID
    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }

    /// Run the actor loop, processing messages until shutdown
    pub async fn run(&mut self) {
        tracing::info!("Actor {} starting", self.actor_id);
        self.state = ActorState::Idle;

        while let Some(msg) = self.rx.recv().await {
            match msg {
                ActorMessage::Process { input, respond_to } => {
                    self.state = ActorState::Processing;
                    tracing::debug!("Actor {} processing slice {}", self.actor_id, input.slice_id);
                    
                    let result = self.loop_impl.process(input).await;
                    
                    // Ignore send error if receiver dropped
                    let _ = respond_to.send(result);
                    
                    self.state = ActorState::Idle;
                }
                ActorMessage::Stats { respond_to } => {
                    let stats = self.loop_impl.stats();
                    let _ = respond_to.send(stats);
                }
                ActorMessage::Shutdown => {
                    tracing::info!("Actor {} shutting down", self.actor_id);
                    self.state = ActorState::ShuttingDown;
                    break;
                }
            }
        }

        self.state = ActorState::Terminated;
        tracing::info!("Actor {} terminated", self.actor_id);
    }
}

/// Handle to interact with an agent actor
#[derive(Clone)]
pub struct ActorHandle {
    /// Channel to send messages to the actor
    tx: mpsc::Sender<ActorMessage>,
    /// Actor ID
    actor_id: String,
    /// Actor's loop type
    loop_type: String,
}

impl ActorHandle {
    /// Create a new actor handle
    pub fn new(tx: mpsc::Sender<ActorMessage>, actor_id: String, loop_type: String) -> Self {
        Self {
            tx,
            actor_id,
            loop_type,
        }
    }

    /// Get the actor's ID
    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }

    /// Get the actor's loop type
    pub fn loop_type(&self) -> &str {
        &self.loop_type
    }

    /// Send a process request and wait for the result
    pub async fn process(&self, input: LoopInput) -> Result<LoopOutput> {
        let (respond_to, rx) = oneshot::channel();
        
        self.tx
            .send(ActorMessage::Process { input, respond_to })
            .await
            .map_err(|_| LoopError::StateViolation("Actor channel closed".into()))?;
        
        rx.await
            .map_err(|_| LoopError::StateViolation("Actor response channel closed".into()))?
    }

    /// Get current statistics
    pub async fn stats(&self) -> Result<LoopStats> {
        let (respond_to, rx) = oneshot::channel();
        
        self.tx
            .send(ActorMessage::Stats { respond_to })
            .await
            .map_err(|_| LoopError::StateViolation("Actor channel closed".into()))?;
        
        rx.await
            .map_err(|_| LoopError::StateViolation("Actor response channel closed".into()))
    }

    /// Send shutdown signal
    pub async fn shutdown(&self) -> Result<()> {
        self.tx
            .send(ActorMessage::Shutdown)
            .await
            .map_err(|_| LoopError::StateViolation("Actor channel closed".into()))
    }
}

/// Supervisor manages a pool of actor workers
pub struct Supervisor {
    /// Map of actor_id -> ActorHandle
    actors: HashMap<String, ActorHandle>,
    /// Channel capacity for actor mailboxes
    channel_capacity: usize,
}

impl Supervisor {
    /// Create a new supervisor
    pub fn new(channel_capacity: usize) -> Self {
        Self {
            actors: HashMap::new(),
            channel_capacity,
        }
    }

    /// Spawn a new actor wrapping a Loop implementation
    pub fn spawn_actor<L: Loop + Send + 'static>(
        &mut self,
        mut loop_impl: L,
        actor_id: String,
    ) -> ActorHandle {
        let (tx, rx) = mpsc::channel(self.channel_capacity);
        let loop_type = loop_impl.loop_type().to_string();
        
        let mut actor = AgentActor::new(
            Box::new(loop_impl),
            rx,
            actor_id.clone(),
        );

        // Spawn the actor on the tokio runtime
        let actor_id_clone = actor_id.clone();
        tokio::spawn(async move {
            actor.run().await;
        });

        let handle = ActorHandle::new(tx, actor_id.clone(), loop_type);
        self.actors.insert(actor_id, handle.clone());
        
        tracing::info!("Spawned actor {}", actor_id_clone);
        handle
    }

    /// Get an actor handle by ID
    pub fn get_actor(&self, actor_id: &str) -> Option<&ActorHandle> {
        self.actors.get(actor_id)
    }

    /// Get all actor IDs
    pub fn actor_ids(&self) -> Vec<&str> {
        self.actors.keys().map(|s| s.as_str()).collect()
    }

    /// Get the number of actors
    pub fn actor_count(&self) -> usize {
        self.actors.len()
    }

    /// Shutdown all actors
    pub async fn shutdown_all(&self) {
        for handle in self.actors.values() {
            let _ = handle.shutdown().await;
        }
    }

    /// Remove terminated actors
    pub fn cleanup_terminated(&mut self) {
        // Note: In a real implementation, we'd track termination state
        // For now, this is a placeholder for future enhancement
    }
}

/// Shared supervisor that can be accessed from multiple tasks
pub type SharedSupervisor = Arc<RwLock<Supervisor>>;

/// Create a new shared supervisor
pub fn new_shared_supervisor(channel_capacity: usize) -> SharedSupervisor {
    Arc::new(RwLock::new(Supervisor::new(channel_capacity)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bruteforce_coder::BruteforceCoderLoop;
    use crate::MotivationSummary;

    #[tokio::test]
    async fn test_actor_creation_and_process() {
        let loop_impl = BruteforceCoderLoop::new();
        let (tx, rx) = mpsc::channel(10);
        
        let mut actor = AgentActor::new(
            Box::new(loop_impl),
            rx,
            "test-actor-1".to_string(),
        );
        
        assert_eq!(actor.state(), &ActorState::Idle);
        assert_eq!(actor.actor_id(), "test-actor-1");
        
        // Spawn actor
        let actor_handle = tokio::spawn(async move {
            actor.run().await;
        });
        
        // Create handle
        let handle = ActorHandle::new(tx, "test-actor-1".to_string(), "bruteforce-coder-loop".to_string());
        
        // Process a task
        let input = LoopInput {
            slice_id: "slice-001".to_string(),
            task_desc: "Test task".to_string(),
            context: vec![],
            motivation: Some(MotivationSummary::default()),
        };
        
        let result = handle.process(input).await;
        assert!(result.is_ok());
        
        let output = result.unwrap();
        assert_eq!(output.slice_id, "slice-001");
        
        // Shutdown
        handle.shutdown().await.unwrap();
        actor_handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_supervisor_spawn_and_manage() {
        let mut supervisor = Supervisor::new(10);
        
        // Spawn actors
        let handle1 = supervisor.spawn_actor(
            BruteforceCoderLoop::new(),
            "bruteforce-1".to_string(),
        );
        
        let handle2 = supervisor.spawn_actor(
            BruteforceCoderLoop::new(),
            "bruteforce-2".to_string(),
        );
        
        assert_eq!(supervisor.actor_count(), 2);
        assert!(supervisor.get_actor("bruteforce-1").is_some());
        assert!(supervisor.get_actor("bruteforce-2").is_some());
        
        // Process tasks concurrently
        let input1 = LoopInput {
            slice_id: "slice-001".to_string(),
            task_desc: "Task 1".to_string(),
            context: vec![],
            motivation: Some(MotivationSummary::default()),
        };
        
        let input2 = LoopInput {
            slice_id: "slice-002".to_string(),
            task_desc: "Task 2".to_string(),
            context: vec![],
            motivation: Some(MotivationSummary::default()),
        };
        
        let (result1, result2) = tokio::join!(
            handle1.process(input1),
            handle2.process(input2),
        );
        
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        
        // Shutdown all
        supervisor.shutdown_all().await;
    }
}
