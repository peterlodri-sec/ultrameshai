-- Enable pgvector extension
CREATE EXTENSION IF NOT EXISTS vector;

-- Memories table (nodes in the graph)
CREATE TABLE memories (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  embedding vector(1536),  -- Semantic space (X/Y/Z compressed)
  content TEXT NOT NULL,
  loop_type TEXT NOT NULL,
  slice_id TEXT,
  encoded_at TIMESTAMPTZ DEFAULT NOW(),  -- Temporal dimension
  accessed_at TIMESTAMPTZ[],  -- Access history
  access_count INT DEFAULT 0,  -- Reinforcement dimension
  decay_rate FLOAT DEFAULT 0.01,
  strength FLOAT DEFAULT 1.0,  -- Reinforcement score
  metadata JSONB DEFAULT '{}'::jsonb
);

-- Indexes for fast lookup
CREATE INDEX memories_embedding_idx ON memories USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);
CREATE INDEX memories_encoded_at_idx ON memories (encoded_at DESC);
CREATE INDEX memories_loop_type_idx ON memories (loop_type);
CREATE INDEX memories_strength_idx ON memories (strength DESC);

-- Function to update access tracking
CREATE OR REPLACE FUNCTION touch_memory(mem_id UUID) RETURNS void AS $$
BEGIN
  UPDATE memories 
  SET accessed_at = array_append(accessed_at, NOW()),
      access_count = access_count + 1,
      strength = strength + 0.1  -- Reinforcement
  WHERE id = mem_id;
END;
$$ LANGUAGE plpgsql;
