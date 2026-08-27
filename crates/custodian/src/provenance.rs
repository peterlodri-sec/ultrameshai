//! Provenance — ancestry + hash-chain verification.
//! The laws: derived meaning never inherits observed provenance;
//! transform(x) != x => prov(transform(x)) != prov(x).

use crate::event::Event;
use std::collections::HashMap;

/// A memory node — a materialized view of an event.
#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub provenance: crate::Provenance,
    pub parents: Vec<String>,
    pub content: String,
    pub agent: String,
}

/// The provenance verifier — walks lineage, checks the laws.
#[derive(Debug, Default)]
pub struct ProvenanceChain {
    nodes: HashMap<String, Node>,
    log: Vec<Event>,
}

impl ProvenanceChain {
    pub fn new() -> Self {
        Self { nodes: HashMap::new(), log: Vec::new() }
    }

    pub fn observe(&mut self, speaker: &str, literal: &str, t: u64, source: &str) -> String {
        let ev = Event::Observe {
            speaker: speaker.to_string(),
            literal: literal.to_string(),
            timestamp: t,
            source: source.to_string(),
        };
        let h = ev.hash();
        self.log.push(ev);
        self.nodes.insert(
            h.clone(),
            Node {
                id: h.clone(),
                provenance: crate::Provenance::Observed,
                parents: vec![],
                content: literal.to_string(),
                agent: speaker.to_string(),
            },
        );
        h
    }

    /// Derive — provenance ALWAYS downgraded.
    pub fn derive(&mut self, parent: &str, transform: &str, result: &str, interpreter: &str) -> Option<String> {
        let parent_node = self.nodes.get(parent)?;
        let ev = Event::Derive {
            parent: parent.to_string(),
            transform: transform.to_string(),
            result: result.to_string(),
            interpreter: interpreter.to_string(),
        };
        let h = ev.hash();
        self.log.push(ev);
        self.nodes.insert(
            h.clone(),
            Node {
                id: h.clone(),
                provenance: parent_node.provenance.downgrade(),
                parents: vec![parent.to_string()],
                content: result.to_string(),
                agent: interpreter.to_string(),
            },
        );
        Some(h)
    }

    pub fn provenance(&self, h: &str) -> Option<crate::Provenance> {
        self.nodes.get(h).map(|n| n.provenance)
    }

    /// Walk the lineage back to the observed island.
    pub fn lineage(&self, h: &str) -> Vec<String> {
        let mut path = vec![];
        let mut cur = h.to_string();
        while let Some(node) = self.nodes.get(&cur) {
            path.push(cur.clone());
            match node.parents.first() {
                Some(p) => cur = p.clone(),
                None => break,
            }
        }
        path
    }

    /// Merkle root of the committed history.
    pub fn history_root(&self) -> String {
        if self.log.is_empty() {
            return blake3::hash(b"empty").to_hex().to_string();
        }
        let mut level: Vec<String> = self.log.iter().map(|e| e.hash()).collect();
        while level.len() > 1 {
            let mut next = Vec::with_capacity(level.len() / 2 + 1);
            for chunk in level.chunks(2) {
                let mut hasher = blake3::Hasher::new();
                hasher.update(chunk[0].as_bytes());
                if let Some(second) = chunk.get(1) {
                    hasher.update(second.as_bytes());
                }
                next.push(hasher.finalize().to_hex().to_string());
            }
            level = next;
        }
        level[0].clone()
    }

    pub fn event_count(&self) -> usize {
        self.log.len()
    }
}
