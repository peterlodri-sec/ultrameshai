use crate::client::{ChatMessage, Role};
use milvus_brain::{MemoryStore, ResearchFinding, QueryBuilder};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct SessionStats {
    pub loop_id: String,
    pub unit_id: String,
    pub message_count: usize,
}

pub struct Session {
    loop_id: String,
    unit_id: String,
    messages: Vec<ChatMessage>,
    created_at: u64,
    last_activity: u64,
}

impl Session {
    pub fn new(loop_id: &str, unit_id: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            loop_id: loop_id.to_string(),
            unit_id: unit_id.to_string(),
            messages: Vec::new(),
            created_at: now,
            last_activity: now,
        }
    }

    pub fn add_message(&mut self, role: Role, content: String) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.last_activity = now;
        self.messages.push(ChatMessage { role, content });
    }

    pub fn get_messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }

    pub fn stats(&self) -> SessionStats {
        SessionStats {
            loop_id: self.loop_id.clone(),
            unit_id: self.unit_id.clone(),
            message_count: self.messages.len(),
        }
    }
}

/// ResearchSession - wraps Session + MemoryStore for research workflows
/// Generic over storage backend (Milvus, mempalace, honcho, in-memory)
pub struct ResearchSession<S: MemoryStore> {
    session: Session,
    store: S,
    source_agent: String,
}

impl<S: MemoryStore> ResearchSession<S> {
    pub fn new(session: Session, store: S, source_agent: &str) -> Self {
        Self {
            session,
            store,
            source_agent: source_agent.to_string(),
        }
    }

    /// Write finding with auto-generated embedding
    pub async fn write_finding(&self, summary: &str, tags: Vec<String>) -> crate::error::Result<()> {
        // Generate embedding via store (auto-embedding enabled on server)
        let finding = ResearchFinding::with_uuid(
            &self.source_agent,
            &self.session.loop_id,
            summary,
            vec![], // Empty - store will embed if configured
            tags,
        );
        self.store.write_finding(finding)
            .await
            .map_err(|e| crate::error::CognitionError::Provider(e.to_string()))?;
        Ok(())
    }

    /// Similarity search + metadata filter
    pub async fn research_find(&self, query: &str, top_k: usize) -> crate::error::Result<Vec<ResearchFinding>> {
        let query_builder = QueryBuilder::new()
            .similarity(query, top_k)
            .filter_agent(&self.source_agent)
            .build();
        self.store.search(query_builder)
            .await
            .map_err(|e| crate::error::CognitionError::Provider(e.to_string()))
    }

    /// Return digest of all findings for this session
    pub async fn summarize_findings(&self) -> crate::error::Result<String> {
        let findings = self.research_find(&self.session.loop_id, 100).await?;
        let digest: Vec<String> = findings.iter()
            .map(|f| format!("- [{}] {}", f.topic, f.summary))
            .collect();
        Ok(digest.join("\n"))
    }

    /// Access underlying session
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Access underlying session (mutable)
    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }
}
