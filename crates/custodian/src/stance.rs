//! Stance — current stance derived from history.
//! REJECTS → ACCEPTS: the historical rejection stays; the stance changes.

use std::collections::HashMap;

pub struct Stance {
    /// target -> is currently accepted
    stances: HashMap<String, bool>,
    /// target -> history of (reject/accept) events
    history: HashMap<String, Vec<String>>,
}

impl Stance {
    pub fn new() -> Self {
        Self { stances: HashMap::new(), history: HashMap::new() }
    }

    pub fn reject(&mut self, target: &str) {
        self.stances.insert(target.to_string(), false);
        self.history.entry(target.to_string()).or_default().push("REJECT".to_string());
    }

    pub fn accept(&mut self, target: &str) {
        self.stances.insert(target.to_string(), true);
        self.history.entry(target.to_string()).or_default().push("ACCEPT".to_string());
    }

    pub fn is_accepted(&self, target: &str) -> bool {
        self.stances.get(target).copied().unwrap_or(true)
    }

    pub fn history(&self, target: &str) -> &[String] {
        self.history.get(target).map(|v| v.as_slice()).unwrap_or(&[])
    }
}
