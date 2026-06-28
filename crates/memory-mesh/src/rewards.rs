use crate::{AgentReward, AgentMotivation, AgentMotivationAggregate, MotivationTier};
use uuid::Uuid;
use chrono::{Utc, DateTime};

/// Reward calculator
pub struct RewardCalculator {
    base_reward: f32,
    tier_multipliers: std::collections::HashMap<MotivationTier, f32>,
}

impl RewardCalculator {
    pub fn new() -> Self {
        let mut tier_multipliers = std::collections::HashMap::new();
        tier_multipliers.insert(MotivationTier::Bronze, 1.0);
        tier_multipliers.insert(MotivationTier::Silver, 1.5);
        tier_multipliers.insert(MotivationTier::Gold, 2.0);
        tier_multipliers.insert(MotivationTier::Platinum, 3.0);

        Self {
            base_reward: 10.0,
            tier_multipliers,
        }
    }

    /// Calculate reward for completed task
    pub fn calculate_reward(
        &self,
        agent_tier: &MotivationTier,
        success: bool,
        a2a_completed: bool,
    ) -> f32 {
        let mut reward = self.base_reward;

        // Tier multiplier
        if let Some(multiplier) = self.tier_multipliers.get(agent_tier) {
            reward *= multiplier;
        }

        // Success bonus
        if !success {
            reward *= 0.5;  // 50% penalty for failure
        }

        // A2A completion bonus (mandatory)
        if a2a_completed {
            reward *= 1.2;  // 20% bonus
        }

        reward
    }

    /// Create reward entry
    pub fn create_reward(
        &self,
        agent_id: Uuid,
        loop_type: String,
        task_id: Option<Uuid>,
        reward_type: String,
        success: bool,
        a2a_completed: bool,
        agent_tier: &MotivationTier,
    ) -> AgentReward {
        let reward_amount = self.calculate_reward(agent_tier, success, a2a_completed);
        let motivation_score = if success { 1.0 } else { 0.0 };

        AgentReward {
            id: Uuid::new_v4(),
            agent_id,
            loop_type,
            task_id,
            reward_type,
            reward_amount,
            motivation_score,
            success,
            earned_at: Utc::now(),
        }
    }

    /// Update agent motivation aggregate
    pub fn update_aggregate(
        &self,
        current: &AgentMotivationAggregate,
        reward: &AgentReward,
        a2a_count_delta: u32,
    ) -> AgentMotivationAggregate {
        let total_points = current.total_points + if reward.success { reward.reward_amount } else { 0.0 };
        let completion_count = if reward.success { 1.0 } else { 0.0 };
        let total_tasks = 1.0;  // Simplified - would track historically
        let completion_rate = completion_count / total_tasks;

        // Determine new tier
        let motivation_tier = if total_points >= 1000.0 {
            MotivationTier::Platinum
        } else if total_points >= 500.0 {
            MotivationTier::Gold
        } else if total_points >= 100.0 {
            MotivationTier::Silver
        } else {
            MotivationTier::Bronze
        };

        AgentMotivationAggregate {
            agent_id: current.agent_id,
            total_points,
            completion_rate,
            a2a_count: current.a2a_count + a2a_count_delta,
            last_active: Utc::now(),
            motivation_tier,
            intrinsic_drive: current.intrinsic_drive.clone(),
            social_pressure: current.social_pressure,
            updated_at: Utc::now(),
        }
    }
}

impl Default for RewardCalculator {
    fn default() -> Self {
        Self::new()
    }
}

/// Intrinsic drive types
#[derive(Debug, Clone, PartialEq)]
pub enum IntrinsicDrive {
    Curiosity,      // Explore new patterns
    Completion,     // Finish tasks
    Optimization,   // Improve efficiency
}

impl IntrinsicDrive {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Curiosity => "curiosity",
            Self::Completion => "completion",
            Self::Optimization => "optimization",
        }
    }
}
