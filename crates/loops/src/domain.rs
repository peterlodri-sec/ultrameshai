/// Domain types for loop trait - decoupled from memory-mesh crate
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MotivationTier {
    Bronze,
    Silver,
    Gold,
    Platinum,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotivationSummary {
    pub tier: MotivationTier,
    pub points: f32,
    pub drive: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardSummary {
    pub amount: f32,
    pub success: bool,
}

impl Default for MotivationTier {
    fn default() -> Self { Self::Bronze }
}

impl Default for MotivationSummary {
    fn default() -> Self {
        Self { tier: MotivationTier::Bronze, points: 0.0, drive: "curiosity".to_string() }
    }
}

impl Default for RewardSummary {
    fn default() -> Self { Self { amount: 0.0, success: false } }
}
