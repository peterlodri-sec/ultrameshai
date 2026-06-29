# dogfeed — Self-Hosting Guide

> Run your own data generation loop. Train models on your own output. Close the loop.

## Prerequisites

- [Bun](https://bun.sh) runtime (or Nix — see below)
- An OpenRouter API key (free tier works) and/or a HuggingFace token
- (Optional) A HuggingFace dataset repo for publishing

## Nix Setup (Recommended)

The easiest way to get a reproducible environment:

```bash
# From the ultrameshai repo root
nix develop .#dogfeed

# Or standalone (from packages/dogfeed/)
nix develop

# Both give you: bun, nodejs, sqlite, jq, curl, git
```

### Nushell Harness

The project includes a nushell harness for all dogfeed operations:

```bash
nu scripts/dogfeed.nu help             # show all commands
nu scripts/dogfeed.nu doctor           # check config + connectivity
nu scripts/dogfeed.nu run              # start the loop
nu scripts/dogfeed.nu run --ralph      # with Ralph reflection
nu scripts/dogfeed.nu stats            # show loop statistics
nu scripts/dogfeed.nu test-llm         # test LLM connection
nu scripts/dogfeed.nu export           # export to JSONL
nu scripts/dogfeed.nu push             # manual push to HuggingFace
```

## Quick Start

```bash
cd packages/dogfeed
bun install

# Run with defaults (OpenRouter free models, no HF push)
OPENROUTER_KEY=sk-... bun src/index.ts
```

## Configuration

All config is via environment variables or a `dogfeed.config.ts` file:

```typescript
// dogfeed.config.ts
import type { DogfeedConfig } from "./src/index";

export default {
  openrouterKey: process.env.OPENROUTER_KEY,
  hfToken: process.env.HF_TOKEN,
  models: ["qwen/qwen-2.5-7b-instruct:free", "meta-llama/llama-3.1-8b-instruct:free"],
  maxTokens: 512,
  intervalSec: 30,
  topics: ["distributed systems", "machine learning", "networking"],
  ralphEvery: 50,
  dailyCallLimit: 200,
  dailyTokenLimit: 50_000,
  dbPath: "./dogfeed.db",
  hfRepo: "your-org/your-dataset",
  pushEvery: 50,
  compress: false,
  compressionLevel: "lite",
  statsPath: "./stats.json",
  telemetryEnabled: true,
} satisfies DogfeedConfig;
```

## Running the Loop

```bash
# Default config
bun src/index.ts

# Custom topics
TOPICS="ml,rl,transformers" bun src/index.ts

# With HF publishing
HF_TOKEN=hf_... HF_REPO=your-org/your-dataset bun src/index.ts

# With compression (requires kompress-ultra)
COMPRESS=true bun src/index.ts
```

## Pipeline Stages

### 1. Topic Selection
The loop picks topics from your configured list using weighted random selection (under-represented topics get picked more often). With `ralphEvery` enabled, the loop asks an LLM to suggest new topics based on recent questions.

### 2. Question + Answer Generation
For each topic, the loop generates a question via one LLM and an answer via another (or the same). Free models from OpenRouter and HuggingFace Inference are supported.

### 3. Scrubbing
PII (emails, phones, IPs, API keys) is redacted. Duplicate answers are caught via content hashing. Quality gates reject short or evasive answers.

### 4. Storage
All records are stored in a local SQLite database (WAL mode). The schema tracks topic, question, answer, model, tokens, hash, and push status.

### 5. Publishing
When `pushEvery` records accumulate, the loop pushes a JSONL batch to your HuggingFace dataset repo. Each batch is timestamped and append-only.

### 6. Ralph Reflection
Every N records, the loop samples recent questions and asks an LLM to suggest an under-covered topic. This self-steers the loop toward uncovered areas.

### 7. Telemetry
Stats are exported to a JSON file every 10 iterations. Events are logged to the SQLite events table.

## Growing the Dataset

### Phase 1: Baseline
- Run with 3-5 free models
- 10-20 topics
- No compression
- Push every 50 records

### Phase 2: Scale
- Add more topics (20+)
- Enable model rotation on 429
- Enable Ralph reflection
- Push every 100 records

### Phase 3: Recursive (v1.1)
- Enable kompress-ultra compression
- Train kompress-v8 on dogfeed data
- Use kompress to compress loop context
- Better context → better topics → better data

## Contributing

### Adding Topics
Edit the default topic list in `src/topic.ts` or pass them via config.

### Adding Models
Add model IDs to the `models` array. OpenRouter model IDs follow the format `provider/model-name:free` for free models.

### Running Multiple Loops
You can run multiple loops on different machines with different topic sets. The dedup hash prevents duplicate records across loops if they share the same SQLite DB.

### Dataset Format
The JSONL output is compatible with HuggingFace Datasets:
```json
{
  "id": "dogfeed-20260629-120000-001",
  "topic": "distributed systems",
  "question": "What is CAP theorem?",
  "answer": "The CAP theorem states...",
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

## Troubleshooting

### "Daily call limit" / "Daily token limit"
Increase `dailyCallLimit` / `dailyTokenLimit` in config, or wait until tomorrow (limits reset at midnight UTC).

### 429 Rate Limits
The loop auto-retries on 429 with a 10s backoff. Add more models to rotate through.

### Duplicate Records
The content hash catches near-duplicates. If you're seeing too many, the quality gate may be too lenient — increase `minLen` in `isQualityAnswer`.

### HF Publish Fails
Check your HF token has write access to the repo. The loop logs publish errors to the events table.

## Architecture

```
Topic → LLM → Scrub → DB → Publish → HuggingFace
  ↑                                    │
  └──── Ralph Reflection ←─────────────┘
```

The loop is recursive: dogfeed generates data → publishes to HF → kompress-v8 trained on data → compresses loop context → better topic selection → higher quality data.
