//! Warrant gate — ΔW = 0 implies ΔA ≤ 0.
//! Persistence is not warrant; repetition is not evidence.

/// The warrant gate: a node's authority cannot increase without new
/// observation. Derived material cannot increase the warrant of its source.
pub struct WarrantGate {
    /// warrant per node hash
    warrant: std::collections::HashMap<String, f64>,
}

impl WarrantGate {
    pub fn new() -> Self {
        Self { warrant: std::collections::HashMap::new() }
    }

    /// Observe: a genuine observation grants warrant 1.0.
    pub fn observe(&mut self, h: &str) {
        self.warrant.insert(h.to_string(), 1.0);
    }

    /// Derive: warrant is capped at a fraction of the parent — derived
    /// material can never match its source's epistemic status.
    pub fn derive(&mut self, h: &str, parent: &str) {
        let pw = self.warrant.get(parent).copied().unwrap_or(0.0);
        self.warrant.insert(h.to_string(), pw * 0.5);
    }

    pub fn warrant(&self, h: &str) -> f64 {
        self.warrant.get(h).copied().unwrap_or(0.0)
    }

    /// The law: ΔW = 0 implies ΔA ≤ 0. If warrant did not change, authority
    /// must not increase.
    pub fn check(&self, before_w: f64, after_w: f64, before_a: f64, after_a: f64) -> bool {
        if (after_w - before_w).abs() < 1e-9 {
            after_a <= before_a + 1e-9
        } else {
            true
        }
    }
}
