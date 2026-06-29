# dogfeed — E2E Spec & Design

> "Slop is data. The loop doesn't care about the provenance — it cares about the structure."

## What is dogfeed?

dogfeed is a **self-improving data generation loop** for LLM agent training. It runs indefinitely, generating Q&A pairs from free LLMs, storing them locally, compressing them with kompress-ultra, and publishing them to HuggingFace for model training.

The loop is recursive: the models trained on dogfeed data power the kompress-ultra compression that makes the loop cheaper to run.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        dogfeed loop                             │
│                                                                  │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐  │
│  │  Topic    │───▶│   LLM    │───▶│  Scrub   │───▶│   DB     │  │
│  │ Selector  │    │  Router  │    │  + Dedup │    │ (SQLite) │  │
│  └──────────┘    └──────────┘    └──────────┘    └─────┬────┘  │
│       │                                                 │        │
│       │         ┌──────────┐    ┌──────────┐          │        │
│       └─────────│  Ralph   │◀───│ Compress │◀─────────┘        │
│                 │Reflection│    │(kompress)│                    │
│                 └──────────┘    └──────────┘                    │
│                                              │                   │
│                                    ┌─────────▼─────────┐       │
│                                    │  HuggingFace Push  │       │
│                                    │  (JSONL batches)   │       │
│                                    └───────────────────┘       │
└─────────────────────────────────────────────────────────────────┘
```

## Pipeline Stages

### 1. Topic Selection
- **Static**: User-provided topic list via config
- **Dynamic**: Ralph reflection pass picks topics from recent data
- **Hybrid**: Static list + Ralph steering every N records

### 2. LLM Routing
- **OpenRouter**: Free models (qwen, llama, mistral)
- **HuggingFace Inference**: Free tier models
- **Local**: MLX on Apple Silicon (future)
- **Budget-aware**: Daily call/token limits, auto-rotation on 429

### 3. Generation
- **Question generation**: "What is the most important question about X?"
- **Answer generation**: Second LLM answers the question
- **Optional**: Multi-turn conversation generation for agent training

### 4. Scrubbing
- **PII removal**: Email, phone, IP, API keys
- **Deduplication**: SHA-256 hash of normalized answer
- **Quality gate**: Minimum answer length, language detection

### 5. Compression (kompress-ultra)
- **Inline**: Compress before storage to save DB space
- **Batch**: Compress on publish to reduce HF upload size
- **Labeling**: Kompress scoring provides eviction labels for training

### 6. Storage
- **SQLite**: Local persistent store (bun:sqlite)
- **Schema**: id, topic, question, answer, model, tokens, compressed, pushed, created_at
- **WAL mode**: Concurrent read/write

### 7. Publishing
- **Batched**: Push every N records (default: 50)
- **JSONL format**: One record per line, compatible with HF datasets
- **Timestamped**: Filename includes timestamp for dedup
- **Append-only**: Never overwrite, always append new batch

### 8. Ralph Reflection
- **Trigger**: Every 50 records (configurable)
- **Process**: Sample recent questions → ask LLM for next topic
- **Storage**: Topic stored in DB config table
- **Effect**: Loop self-steers toward uncovered areas

### 9. Telemetry
- **Stats**: records_generated, records_pushed, tokens_used, api_calls, errors
- **Events**: topic_change, model_switch, push_complete, error
- **Export**: JSON stats file, HF telemetry batch

## Data Schema

### SQLite Tables

```sql
CREATE TABLE records (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  topic TEXT NOT NULL,
  question TEXT NOT NULL,
  answer TEXT NOT NULL,
  model TEXT NOT NULL,
  tokens_in INTEGER DEFAULT 0,
  tokens_out INTEGER DEFAULT 0,
  compressed_answer TEXT,
  hash TEXT NOT NULL,
  pushed BOOLEAN DEFAULT 0,
  created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX idx_records_topic ON records(topic);
CREATE INDEX idx_records_hash ON records(hash);
CREATE INDEX idx_records_pushed ON records(pushed);

CREATE TABLE config (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  level TEXT NOT NULL,
  message TEXT NOT NULL,
  created_at TEXT DEFAULT (datetime('now'))
);
```

### JSONL Output Format

```json
{
  "id": "dogfeed-20260629-120000-001",
  "topic": "distributed systems",
  "question": "What is the CAP theorem and why does it matter?",
  "answer": "The CAP theorem states that a distributed system can provide at most two of three guarantees...",
  "model": "qwen/qwen-2.5-7b-instruct:free",
  "tokens_in": 150,
  "tokens_out": 620,
  "compressed_answer": "CAP theorem: consistency, availability, partition tolerance — pick 2...",
  "role": "pruner",
  "source": "dogfeed-loop",
  "topic_category": "distributed-systems",
  "created_at": "2026-06-29T12:00:00Z"
}
```

## Configuration

```typescript
interface DogfeedConfig {
  // LLM
  openrouterKey?: string;
  hfToken?: string;
  models: string[];           // Model IDs to rotate through
  maxTokens: number;          // Max tokens per answer (default: 512)
  
  // Loop
  intervalSec: number;        // Seconds between iterations (default: 30)
  topics: string[];           // Static topic list (empty = Ralph only)
  ralphEvery: number;         // Ralph reflection interval (default: 50)
  
  // Budget
  dailyCallLimit: number;     // Max API calls/day (default: 200)
  dailyTokenLimit: number;    // Max tokens/day (default: 50000)
  
  // Storage
  dbPath: string;             // SQLite path (default: ./dogfeed.db)
  
  // Publishing
  hfRepo: string;             // HuggingFace dataset repo
  pushEvery: number;          // Push after N records (default: 50)
  
  // Compression
  compress: boolean;          // Enable kompress-ultra (default: false)
  compressionLevel: 'lite' | 'ultra';  // Compression level
  
  // Telemetry
  statsPath: string;          // Stats export path (default: ./stats.json)
  telemetryEnabled: boolean;  // Enable telemetry export (default: true)
}
```

## Commands

```bash
# Run the loop
dogfeed run                    # Start with default config
dogfeed run --topics "ml,rl"   # Custom topics
dogfeed run --ralph            # Enable Ralph reflection

# Manage
dogfeed stats                  # Show loop statistics
dogfeed export                 # Export to HF-compatible JSONL
dogfeed push                   # Manual push to HuggingFace

# Debug
dogfeed doctor                 # Check config, API keys, connectivity
dogfeed test-llm               # Test LLM connection
dogfeed test-hf                # Test HuggingFace connection
```

## Recursive Loop (v1.1)

The recursive loop is the key insight: **the app that generates the data also consumes the model trained on that data.**

```
dogfeed loop generates data → publishes to HF → kompress-v8 trained on data
    ↑                                                    │
    └──── kompress-v8 compresses loop context ◀──────────┘
```

In practice:
1. dogfeed loop runs, generates Q&A pairs
2. Pairs published to HF dataset
3. kompress-v8 fine-tuned on dataset
4. kompress-v8 integrated into dogfeed loop via kompress-ultra
5. Loop context compressed → can hold 10x more history → better topic selection
6. Better topics → higher quality data → better model → better compression

This is the "entry app" — dogfeed is the first consumer of its own output.

## Growth Strategy

### Phase 1: Baseline (current)
- 300+ loop files, 9.5MB parquet, 45K+ turns
- 7 topics, free LLMs, no compression

### Phase 2: Scale
- 1000+ loop files, 50MB+ parquet
- 20+ topics, model rotation, kompress integration
- Quality gates, dedup, PII scrubbing

### Phase 3: Recursive
- kompress-v8 trained on dogfeed data
- Loop uses kompress for context compression
- Self-improving topic selection
- Multi-source: OpenRouter + HF + local MLX

### Phase 4: Ecosystem
- Multiple contributors running loops
- Shared topic steering via telemetry
- Cross-repo dogfeed (ultrameshai + kompress-ultra + proposal)
- Public dataset with 100K+ turns

## Metrics

| Metric | Phase 1 | Phase 2 | Phase 3 | Phase 4 |
|--------|---------|---------|---------|---------|
| Records | 45K | 200K | 500K | 1M+ |
| Topics | 7 | 20 | 50 | 100+ |
| Models | 3 | 10 | 10 + kompress | 20 + recursive |
| Compression | None | Lite | Ultra | Recursive |
| Contributors | 1 | 3 | 10 | 50+ |

## Files

```
packages/dogfeed/
├── src/
│   ├── index.ts          # Public API
│   ├── loop.ts           # Core loop engine
│   ├── topic.ts          # Topic selection + Ralph
│   ├── llm.ts            # LLM client (OpenRouter, HF)
│   ├── db.ts             # SQLite storage
│   ├── compress.ts       # kompress-ultra integration
│   ├── publish.ts        # HuggingFace publisher
│   ├── telemetry.ts      # Stats and events
│   ├── scrub.ts          # PII scrubbing
│   └── types.ts          # All types
├── test/
│   ├── loop.test.ts
│   ├── topic.test.ts
│   ├── scrub.test.ts
│   └── db.test.ts
├── examples/
│   ├── basic-loop.ts     # Minimal loop
│   ├── custom-topics.ts  # Custom topic list
│   ├── with-compression.ts
│   └── nix-flake.nix
├── SPEC.md               # This file
├── GUIDE.md              # Self-hosting guide
├── package.json
├── tsconfig.json
└── README.md
```
