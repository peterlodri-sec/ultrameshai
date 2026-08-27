//! Decision — record → admit → commit.
//! commit() is the ONLY public path that mutates epistemic state.

use crate::event::Event;
use crate::provenance::ProvenanceChain;

pub struct Decision {
    chain: ProvenanceChain,
    /// admitted-but-not-committed candidates
    pending: Vec<String>,
}

impl Decision {
    pub fn new() -> Self {
        Self { chain: ProvenanceChain::new(), pending: Vec::new() }
    }

    /// Record: an observation enters the log. No epistemic commitment yet.
    pub fn record(&mut self, speaker: &str, literal: &str, t: u64, source: &str) -> String {
        self.chain.observe(speaker, literal, t, source)
    }

    /// Admit: a recorded event is proposed for commitment.
    pub fn admit(&mut self, id: &str) -> bool {
        if self.chain.provenance(id).is_some() {
            self.pending.push(id.to_string());
            true
        } else {
            false
        }
    }

    /// Commit: the ONLY public path that mutates epistemic state.
    /// A committed event is sealed into the history root.
    pub fn commit(&mut self, id: &str) -> Option<String> {
        if !self.pending.iter().any(|p| p == id) {
            return None;
        }
        self.pending.retain(|p| p != id);
        Some(self.chain.history_root())
    }

    pub fn history_root(&self) -> String {
        self.chain.history_root()
    }

    pub fn event_count(&self) -> usize {
        self.chain.event_count()
    }
}
