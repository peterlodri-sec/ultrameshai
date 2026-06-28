use crate::traits::{Loop, LoopInput, LoopOutput, LoopStats};
use crate::error::{LoopError};
use crate::error::{Result};
use loop_engineering_cognition::{LlmClient, Session, PromptDispatcher, Role};
use memory_mesh::{Memory, connectors};
use uuid::Uuid;

pub struct MemoryIndexerLoop {
    client: LlmClient,
    session: Session,
    dispatcher: PromptDispatcher,
    stats: LoopStats,
}

impl MemoryIndexerLoop {
    pub fn new() -> Self {
        let client = LlmClient::mock("memory-indexer");
        let session = Session::new("memory-indexer-loop", "unit-000");
        let dispatcher = PromptDispatcher::default();
        Self { client, session, dispatcher, stats: LoopStats::default() }
    }

    /// Write memory to 4D mesh (Supabase + Milvus)
    async fn write_to_mesh(&self, content: String, loop_type: String) -> Result<Memory> {
        // Generate embedding via Milvus
        let embedding = vec![0.0f32; 1536]; // TODO: call Milvus embedding API
        
        let mut memory = Memory::new(content, loop_type, embedding);
        memory.slice_id = Some(Uuid::new_v4().to_string());
        
        // TODO: Write to Supabase via memory-mesh connector
        // TODO: Write to Milvus for semantic search
        
        Ok(memory)
    }
}

impl Default for MemoryIndexerLoop {
    fn default() -> Self { Self::new() }
}

#[async_trait::async_trait]
impl Loop for MemoryIndexerLoop {
    fn loop_type(&self) -> &str { "memory-indexer" }

    async fn process(&mut self, input: LoopInput) -> Result<LoopOutput> {
        let mut variables = std::collections::HashMap::new();
        variables.insert("task".to_string(), input.task_desc.clone());
        let prompt = self.dispatcher
            .dispatch("memory-indexer", &variables)
            .unwrap_or_else(|| input.task_desc.clone());

        self.session.add_message(Role::User, prompt.clone());
        let messages = self.session.get_messages().to_vec();
        let response = self.client.chat(messages)
            .await
            .map_err(|e| LoopError::LlmPermanent(e.to_string()))?;

        // Write to 4D memory mesh
        let _memory = self.write_to_mesh(response.content.clone(), self.loop_type().to_string()).await?;

        self.session.add_message(Role::Assistant, response.content.clone());
        self.stats.slices_processed += 1;

        Ok(LoopOutput {
            slice_id: input.slice_id,
            result: response.content,
            tool_calls: vec![],
            stats: self.stats.clone(),
            reward_earned: None,
            a2a_completed: false,
            reward_earned: None,
            a2a_completed: false,
        })
    }

    fn stats(&self) -> LoopStats { self.stats.clone() }
}

