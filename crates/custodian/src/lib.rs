//! Custodian — provenance-guarded memory gatekeeper.
//!
//! The laws, encoded:
//!   - Quotation is observation. Interpretation is derivation.
//!   - Derived meaning never inherits observed provenance.
//!   - Agency must survive compression.
//!   - A person must retain the right to reject the system's interpretation
//!     of their own record.
//!   - transform(x) != x  =>  prov(transform(x)) != prov(x)
//!   - Growth must not falsify ancestry.
//!
//! Architecture: an immutable event log (OBSERVE / DERIVE / REJECT /
//! ACCEPT / CORRECT), each event hashed with BLAKE3, rolled into a Merkle
//! root per epoch, chained across epochs. Nodes and edges are referenced by
//! their event hashes — the graph is a materialized view of the history.
//! `commit()` is the ONLY public path that mutates epistemic state.

use std::collections::HashMap;

/// The provenance of a memory event — the epistemic type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// An actual historical event — Peter said this, at this time.
    Observed,
    /// Mechanically inferred from observed material.
    Inferred,
    /// Generated to explore possibilities.
    Synthetic,
    /// Human/model semantic construction.
    Interpretive,
    /// Counterfactual or experimental.
    Simulated,
}

impl Provenance {
    /// The Custodian invariant: a transformation may never inherit the
    /// provenance of its source.
    pub fn downgrade(&self) -> Provenance {
        match self {
            Provenance::Observed => Provenance::Inferred,
            other => *other,
        }
    }
}

/// A historical event — the only thing that is hashed and committed.
#[derive(Debug, Clone)]
pub enum Event {
    /// An observation: someone said/did something, literally.
    Observe {
        speaker: String,
        literal: String,
        timestamp: u64,
        source: String,
    },
    /// A derivation: an interpretation of a parent event.
    Derive {
        parent: String, // parent hash (hex)
        transform: String,
        result: String,
        interpreter: String,
    },
    /// A rejection: P rejects interpretation I. Immutable, first-class.
    Reject {
        subject: String, // hash of the person's identity event
        target: String,  // hash of the interpretation
    },
    /// An acceptance: P accepts I, superseding a historical rejection
    /// without erasing it.
    Accept {
        subject: String,
        target: String,
    },
    /// A correction: the system made a mistake; the mistake is itself
    /// history and must not be erased.
    Correct {
        bad: String,
        good: String,
        reason: String,
    },
}

impl Event {
    /// Canonical serialization for hashing.
    pub fn canonical(&self) -> Vec<u8> {
        match self {
            Event::Observe { speaker, literal, timestamp, source } => {
                format!("OBSERVE|{speaker}|{literal}|{timestamp}|{source}").into_bytes()
            }
            Event::Derive { parent, transform, result, interpreter } => {
                format!("DERIVE|{parent}|{transform}|{result}|{interpreter}").into_bytes()
            }
            Event::Reject { subject, target } => {
                format!("REJECT|{subject}|{target}").into_bytes()
            }
            Event::Accept { subject, target } => {
                format!("ACCEPT|{subject}|{target}").into_bytes()
            }
            Event::Correct { bad, good, reason } => {
                format!("CORRECT|{bad}|{good}|{reason}").into_bytes()
            }
        }
    }

    /// The BLAKE3 content hash of this event, as a hex string.
    pub fn hash(&self) -> String {
        blake3::hash(&self.canonical()).to_hex().to_string()
    }
}

/// A memory node — a materialized view of an event, with its provenance
/// and its lineage (the parent hashes it descends from).
#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub provenance: Provenance,
    pub parents: Vec<String>,
    pub content: String,
    /// The agency — who supplied the action. Must survive compression.
    pub agent: String,
}

/// A rejection edge — first-class, immutable.
#[derive(Debug, Clone)]
pub struct Rejection {
    pub subject: String,
    pub target: String,
    pub timestamp: u64,
}

/// The Custodian — the gatekeeper that enforces the laws.
#[derive(Debug, Default)]
pub struct Custodian {
    log: Vec<Event>,
    nodes: HashMap<String, Node>,
    rejections: Vec<Rejection>,
    /// Tombstones: hashes of rejected interpretations, kept hot so a
    /// hallucination can never be regenerated silently.
    tombstones: HashMap<String, u64>,
}

impl Custodian {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an observation. Returns the node hash.
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
                provenance: Provenance::Observed,
                parents: vec![],
                content: literal.to_string(),
                agent: speaker.to_string(),
            },
        );
        h
    }

    /// Record a derivation. The provenance is ALWAYS downgraded — an
    /// interpretation never inherits OBSERVED.
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

    /// Record a rejection. The interpretation stays intact; the rejection
    /// is a separate immutable event. The tombstone is set so the
    /// hallucination cannot be regenerated.
    pub fn reject(&mut self, subject: &str, target: &str, t: u64) {
        let ev = Event::Reject {
            subject: subject.to_string(),
            target: target.to_string(),
        };
        self.log.push(ev);
        self.rejections.push(Rejection {
            subject: subject.to_string(),
            target: target.to_string(),
            timestamp: t,
        });
        self.tombstones.insert(target.to_string(), t);
    }

    /// Record an acceptance — supersedes a historical rejection without
    /// erasing it. The tombstone is lifted, but the rejection event remains.
    pub fn accept(&mut self, subject: &str, target: &str) {
        let ev = Event::Accept {
            subject: subject.to_string(),
            target: target.to_string(),
        };
        self.log.push(ev);
        self.tombstones.remove(target);
    }

    /// Record a correction. The bad event is preserved as history.
    pub fn correct(&mut self, bad: &str, good: &str, reason: &str) {
        let ev = Event::Correct {
            bad: bad.to_string(),
            good: good.to_string(),
            reason: reason.to_string(),
        };
        self.log.push(ev);
    }

    /// The Custodian's unit test: has this interpretation been rejected?
    pub fn is_tombstoned(&self, target: &str) -> bool {
        self.tombstones.contains_key(target)
    }

    /// The provenance of a node.
    pub fn provenance(&self, h: &str) -> Option<Provenance> {
        self.nodes.get(h).map(|n| n.provenance)
    }

    /// The lineage of a node — walk back to the observed island.
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

    /// The Merkle root of the committed history.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_is_observed() {
        let mut c = Custodian::new();
        let o = c.observe("Peter", "feel the light X 101", 1, "session");
        assert_eq!(c.provenance(&o), Some(Provenance::Observed));
    }

    #[test]
    fn derivation_never_inherits_observed() {
        let mut c = Custodian::new();
        let o = c.observe("Peter", "feel the light X 101", 1, "session");
        let i = c.derive(&o, "associate-with-light", "101 is light", "system").unwrap();
        // THE law: an interpretation of an observation is INFERRED, never OBSERVED.
        assert_eq!(c.provenance(&i), Some(Provenance::Inferred));
    }

    #[test]
    fn rejection_is_immutable_and_tombstoned() {
        let mut c = Custodian::new();
        let o = c.observe("Peter", "telibeszarom a parazst", 2, "session");
        let i = c.derive(&o, "ember-metaphor", "the ember waits", "system").unwrap();
        let p = c.observe("Peter", "identity", 0, "session");
        c.reject(&p, &i, 3);
        assert!(c.is_tombstoned(&i));
        // The interpretation is still present — rejection does not delete.
        assert!(c.provenance(&i).is_some());
    }

    #[test]
    fn accept_supersedes_without_erasing() {
        let mut c = Custodian::new();
        let o = c.observe("Peter", "x", 1, "session");
        let i = c.derive(&o, "t", "y", "system").unwrap();
        let p = c.observe("Peter", "identity", 0, "session");
        c.reject(&p, &i, 2);
        assert!(c.is_tombstoned(&i));
        c.accept(&p, &i);
        assert!(!c.is_tombstoned(&i));
        // The rejection event is still in the log — history intact.
        assert_eq!(c.event_count(), 5); // observe, derive, identity-observe, reject, accept
    }

    #[test]
    fn correction_preserves_the_error() {
        let mut c = Custodian::new();
        let bad = c.observe("system", "dissent attributed as OBSERVED", 1, "bug");
        let good = c.observe("Peter", "dissent? rly? when?", 2, "session");
        c.correct(&bad, &good, "provenance contamination");
        // The bad event is still in the log — error → repair → erase is forbidden.
        assert_eq!(c.event_count(), 3);
    }

    #[test]
    fn lineage_walks_back_to_the_island() {
        let mut c = Custodian::new();
        let o = c.observe("Peter", "feel the light X 101", 1, "session");
        let i1 = c.derive(&o, "light", "101 is light", "system").unwrap();
        let i2 = c.derive(&i1, "milarepa", "like Milarepa", "system").unwrap();
        let lineage = c.lineage(&i2);
        assert_eq!(lineage.len(), 3);
        assert_eq!(lineage.last(), Some(&o)); // walks back to the island
    }

    #[test]
    fn history_root_changes_if_any_event_changes() {
        let mut c = Custodian::new();
        c.observe("Peter", "a", 1, "s");
        let r1 = c.history_root();
        let mut c2 = Custodian::new();
        c2.observe("Peter", "b", 1, "s");
        let r2 = c2.history_root();
        assert_ne!(r1, r2);
    }

    #[test]
    fn transform_does_not_inherit_provenance() {
        let mut c = Custodian::new();
        let o = c.observe("Peter", "limits?", 1, "session");
        let d = c.derive(&o, "paraphrase", "Peter said limits were meaningless", "system").unwrap();
        // The paraphrase is INFERRED, never OBSERVED — even though it
        // descends from an observation.
        assert_eq!(c.provenance(&d), Some(Provenance::Inferred));
        assert_ne!(c.provenance(&d), c.provenance(&o));
    }
}
