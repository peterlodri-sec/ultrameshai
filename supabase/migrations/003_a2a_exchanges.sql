-- A2A exchange logging (mandatory for all agents)
CREATE TABLE a2a_exchanges (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  from_node UUID NOT NULL,
  to_node UUID NOT NULL,
  from_loop TEXT NOT NULL,
  to_loop TEXT,
  request JSONB NOT NULL,
  response JSONB,
  latency_ms INT,
  success BOOLEAN DEFAULT true,
  error_message TEXT,
  logged_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for audit queries
CREATE INDEX a2a_from_node_idx ON a2a_exchanges (from_node);
CREATE INDEX a2a_to_node_idx ON a2a_exchanges (to_node);
CREATE INDEX a2a_logged_at_idx ON a2a_exchanges (logged_at DESC);
CREATE INDEX a2a_success_idx ON a2a_exchanges (success);

-- Function to log A2A exchange
CREATE OR REPLACE FUNCTION log_a2a(
  p_from_node UUID,
  p_to_node UUID,
  p_from_loop TEXT,
  p_to_loop TEXT,
  p_request JSONB,
  p_response JSONB,
  p_latency_ms INT,
  p_success BOOLEAN,
  p_error_message TEXT
) RETURNS UUID AS $$
DECLARE
  v_id UUID;
BEGIN
  INSERT INTO a2a_exchanges (from_node, to_node, from_loop, to_loop, request, response, latency_ms, success, error_message)
  VALUES (p_from_node, p_to_node, p_from_loop, p_to_loop, p_request, p_response, p_latency_ms, p_success, p_error_message)
  RETURNING id INTO v_id;
  RETURN v_id;
END;
$$ LANGUAGE plpgsql;
