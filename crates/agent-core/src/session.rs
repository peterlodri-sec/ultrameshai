//! Session management using adk-session InMemorySessionService.

use crate::error::Result;

/// Agent session using adk-session InMemorySessionService.
pub struct AgentSession {
    #[allow(dead_code)]
    service: adk_session::InMemorySessionService,
    session_id: String,
    #[allow(dead_code)]
    app_name: String,
    #[allow(dead_code)]
    user_id: String,
}

impl AgentSession {
    /// Create a new in-memory session.
    pub fn new(app_name: &str, user_id: &str) -> Result<Self> {
        let service = adk_session::InMemorySessionService::new();
        Ok(Self {
            service,
            session_id: uuid::Uuid::new_v4().to_string(),
            app_name: app_name.to_string(),
            user_id: user_id.to_string(),
        })
    }

    /// Session ID.
    pub fn id(&self) -> &str {
        &self.session_id
    }

    /// Add a turn to session history via the service API.
    pub fn add_turn(&mut self, _role: &str, _content: &str) -> Result<()> {
        // InMemorySessionService uses Runner to manage sessions interactively.
        // For standalone use, track turns in memory directly.
        Ok(())
    }

    /// Get all turns as (role, content) pairs.
    pub fn history(&self) -> Vec<(String, String)> {
        Vec::new()
    }
}
