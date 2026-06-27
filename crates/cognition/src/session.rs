use crate::client::{ChatMessage, Role};
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
