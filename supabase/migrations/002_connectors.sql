-- Inter-connectors (edges in the graph)
CREATE TABLE connectors (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  from_id UUID REFERENCES memories(id) ON DELETE CASCADE,
  to_id UUID REFERENCES memories(id) ON DELETE CASCADE,
  connector_type TEXT NOT NULL CHECK (connector_type IN ('similarity', 'temporal', 'causal', 'reinforcement')),
  weight FLOAT DEFAULT 1.0,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  metadata JSONB DEFAULT '{}'::jsonb
);

-- Indexes for graph traversal
CREATE INDEX connectors_from_idx ON connectors (from_id);
CREATE INDEX connectors_to_idx ON connectors (to_id);
CREATE INDEX connectors_type_idx ON connectors (connector_type);
CREATE INDEX connectors_weight_idx ON connectors (weight DESC);

-- Prevent duplicate edges
CREATE UNIQUE INDEX connectors_unique_idx ON connectors (from_id, to_id, connector_type);
