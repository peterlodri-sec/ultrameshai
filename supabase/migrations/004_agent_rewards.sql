-- Agent rewards ledger
CREATE TABLE agent_rewards (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  agent_id UUID NOT NULL,
  loop_type TEXT NOT NULL,
  task_id UUID,
  reward_type TEXT NOT NULL,  -- 'points', 'priority', 'access_token'
  reward_amount FLOAT NOT NULL,
  motivation_score FLOAT DEFAULT 0.0,
  success BOOLEAN DEFAULT true,
  earned_at TIMESTAMPTZ DEFAULT NOW()
);

-- Agent motivations (aggregate view)
CREATE TABLE agent_motivations (
  agent_id UUID PRIMARY KEY,
  total_points FLOAT DEFAULT 0,
  completion_rate FLOAT DEFAULT 0,
  a2a_count INT DEFAULT 0,
  last_active TIMESTAMPTZ DEFAULT NOW(),
  motivation_tier TEXT DEFAULT 'bronze' CHECK (motivation_tier IN ('bronze', 'silver', 'gold', 'platinum')),
  intrinsic_drive TEXT DEFAULT 'curiosity',  -- 'curiosity', 'completion', 'optimization'
  social_pressure FLOAT DEFAULT 0.5,
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes
CREATE INDEX agent_rewards_agent_idx ON agent_rewards (agent_id);
CREATE INDEX agent_rewards_earned_at_idx ON agent_rewards (earned_at DESC);
CREATE INDEX agent_motivations_tier_idx ON agent_motivations (motivation_tier);

-- Function to update motivation tier
CREATE OR REPLACE FUNCTION update_motivation_tier() RETURNS TRIGGER AS $$
BEGIN
  NEW.motivation_tier := CASE
    WHEN NEW.total_points >= 1000 THEN 'platinum'
    WHEN NEW.total_points >= 500 THEN 'gold'
    WHEN NEW.total_points >= 100 THEN 'silver'
    ELSE 'bronze'
  END;
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER motivation_tier_trigger
  BEFORE INSERT OR UPDATE ON agent_motivations
  FOR EACH ROW EXECUTE FUNCTION update_motivation_tier();
