//! Event types — the immutable history log.
//! OBSERVE / DERIVE / REJECT / ACCEPT / CORRECT.
//! Each event is hashed with BLAKE3; the hash is its identity.

use crate::Provenance;

/// A historical event — the only thing that is hashed and committed.
#[derive(Debug, Clone)]
pub enum Event {
    Observe {
        speaker: String,
        literal: String,
        timestamp: u64,
        source: String,
    },
    Derive {
        parent: String,
        transform: String,
        result: String,
        interpreter: String,
    },
    Reject {
        subject: String,
        target: String,
    },
    Accept {
        subject: String,
        target: String,
    },
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

    /// The BLAKE3 content hash as a hex string.
    pub fn hash(&self) -> String {
        blake3::hash(&self.canonical()).to_hex().to_string()
    }
}

/// The provenance of an event — the epistemic type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prov {
    Observed,
    Inferred,
    Synthetic,
    Interpretive,
    Simulated,
}

impl From<Prov> for Provenance {
    fn from(p: Prov) -> Self {
        match p {
            Prov::Observed => Provenance::Observed,
            Prov::Inferred => Provenance::Inferred,
            Prov::Synthetic => Provenance::Synthetic,
            Prov::Interpretive => Provenance::Interpretive,
            Prov::Simulated => Provenance::Simulated,
        }
    }
}
