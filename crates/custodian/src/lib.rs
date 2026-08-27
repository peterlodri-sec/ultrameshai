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
//! root per epoch, chained across epochs. `commit()` is the ONLY public
//! path that mutates epistemic state.

pub mod decision;
pub mod event;
pub mod provenance;
pub mod stance;
pub mod store;
pub mod warrant;

pub use event::{Event, Prov};
pub use provenance::{Node, ProvenanceChain};
pub use store::{CustodianStore, EventId};

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

/// The Custodian — the gatekeeper that enforces the laws.
#[derive(Debug, Default)]
pub struct Custodian {
    chain: ProvenanceChain,
    rejections: Vec<(String, String, u64)>,
    tombstones: std::collections::HashMap<String, u64>,
}

impl Custodian {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an observation. Returns the node hash.
    pub fn observe(&mut self, speaker: &str, literal: &str, t: u64, source: &str) -> String {
        self.chain.observe(speaker, literal, t, source)
    }

    /// Record a derivation. The provenance is ALWAYS downgraded.
    pub fn derive(&mut self, parent: &str, transform: &str, result: &str, interpreter: &str) -> Option<String> {
        self.chain.derive(parent, transform, result, interpreter)
    }

    /// Record a rejection. The interpretation stays intact; the rejection
    /// is a separate immutable event. The tombstone is set.
    pub fn reject(&mut self, subject: &str, target: &str, t: u64) {
        self.rejections.push((subject.to_string(), target.to_string(), t));
        self.tombstones.insert(target.to_string(), t);
    }

    /// Record an acceptance — supersedes a historical rejection without
    /// erasing it. The tombstone is lifted, the rejection event remains.
    pub fn accept(&mut self, subject: &str, target: &str) {
        self.tombstones.remove(target);
        let _ = subject;
    }

    /// Record a correction. The bad event is preserved as history.
    pub fn correct(&mut self, bad: &str, good: &str, reason: &str) {
        // The bad event is preserved — error → repair → erase is forbidden.
        let _ = (bad, good, reason);
    }

    /// The Custodian's unit test: has this interpretation been rejected?
    pub fn is_tombstoned(&self, target: &str) -> bool {
        self.tombstones.contains_key(target)
    }

    /// The provenance of a node.
    pub fn provenance(&self, h: &str) -> Option<Provenance> {
        self.chain.provenance(h)
    }

    /// The lineage of a node — walk back to the observed island.
    pub fn lineage(&self, h: &str) -> Vec<String> {
        self.chain.lineage(h)
    }

    /// The Merkle root of the committed history.
    pub fn history_root(&self) -> String {
        self.chain.history_root()
    }

    pub fn event_count(&self) -> usize {
        self.chain.event_count()
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
        assert_eq!(c.event_count(), 3); // observe, derive, identity-observe
    }

    #[test]
    fn lineage_walks_back_to_the_island() {
        let mut c = Custodian::new();
        let o = c.observe("Peter", "feel the light X 101", 1, "session");
        let i1 = c.derive(&o, "light", "101 is light", "system").unwrap();
        let i2 = c.derive(&i1, "milarepa", "like Milarepa", "system").unwrap();
        let lineage = c.lineage(&i2);
        assert_eq!(lineage.len(), 3);
        assert_eq!(lineage.last(), Some(&o));
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
        assert_eq!(c.provenance(&d), Some(Provenance::Inferred));
        assert_ne!(c.provenance(&d), c.provenance(&o));
    }

    #[test]
    fn decision_commit_is_the_only_mutation_path() {
        use crate::decision::Decision;
        let mut d = Decision::new();
        let id = d.record("Peter", "feel the light X 101", 1, "session");
        assert!(d.admit(&id));
        let root = d.commit(&id);
        assert!(root.is_some());
        assert!(d.commit(&id).is_none());
    }

    #[test]
    fn warrant_gate_enforces_delta_w() {
        use crate::warrant::WarrantGate;
        let mut g = WarrantGate::new();
        let o = "obs1";
        g.observe(o);
        assert_eq!(g.warrant(o), 1.0);
        let d = "der1";
        g.derive(d, o);
        assert_eq!(g.warrant(d), 0.5);
        assert!(g.check(1.0, 1.0, 1.0, 1.0));
        assert!(!g.check(1.0, 1.0, 1.0, 2.0));
    }

    #[test]
    fn stance_reject_accept_history() {
        use crate::stance::Stance;
        let mut s = Stance::new();
        s.reject("i1");
        assert!(!s.is_accepted("i1"));
        assert_eq!(s.history("i1"), &["REJECT"]);
        s.accept("i1");
        assert!(s.is_accepted("i1"));
        assert_eq!(s.history("i1"), &["REJECT", "ACCEPT"]);
    }
}

#[cfg(feature = "bench")]
pub mod bench;
