# dogfeed

> Self-improving data generation loop for LLM agent training. Slop is data.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## What is dogfeed?

dogfeed runs indefinitely, generating Q&A pairs from free LLMs, scrubbing PII, deduplicating, and publishing to HuggingFace. The models trained on dogfeed data power the compression that makes the loop cheaper to run.

```
Topic → LLM → Scrub → DB → Publish → HuggingFace
  ↑                                    │
  └──── Ralph Reflection ←─────────────┘
```

## Quick Start

```bash
# Option 1: Bun
cd packages/dogfeed
bun install
OPENROUTER_KEY=sk-... bun src/index.ts

# Option 2: Nix (reproducible)
nix develop .#dogfeed
OPENROUTER_KEY=sk-... bun src/index.ts

# Option 3: Standalone flake
cd packages/dogfeed
nix develop
OPENROUTER_KEY=sk-... bun src/index.ts
```

## Features

- **Multi-provider LLM routing** — OpenRouter free models, HuggingFace Inference, local MLX (future)
- **PII scrubbing** — Email, phone, IP, API key redaction
- **Content dedup** — SHA-256 normalized content hashing
- **Quality gates** — Reject short, evasive, or non-English answers
- **Weighted topic selection** — Under-represented topics get picked more often
- **Ralph reflection** — LLM steers topic selection toward uncovered areas
- **SQLite storage** — WAL mode, zero external dependencies
- **HF publishing** — Timestamped JSONL batches, append-only
- **Compression** — Optional kompress-ultra integration
- **Budget controls** — Daily call and token limits
- **Telemetry** — Stats export and event logging

## Configuration

```typescript
import type { DogfeedConfig } from "dogfeed";

const config: DogfeedConfig = {
  models: ["qwen/qwen-2.5-7b-instruct:free"],
  maxTokens: 512,
  intervalSec: 30,
  topics: ["distributed systems", "machine learning"],
  ralphEvery: 50,
  dailyCallLimit: 200,
  dailyTokenLimit: 50_000,
  dbPath: "./dogfeed.db",
  hfRepo: "your-org/your-dataset",
  pushEvery: 50,
  compress: false,
};
```

## API

```typescript
import {
  DogfeedDB,
  iteration,
  runLoop,
  pickTopic,
  scrubPII,
  contentHash,
  recordsToJSONL,
  publishBatch,
} from "dogfeed";

// Single iteration
const conn = new DogfeedDB("./dogfeed.db");
const result = await iteration(config, conn);

// Full loop
await runLoop(config);

// Utilities
const clean = scrubPII("email: alice@example.com"); // "email: [REDACTED]"
const hash = contentHash("some text");               // "dh-..."
const jsonl = recordsToJSONL(records);                // JSONL string
```

## Examples

- [`examples/basic-loop.ts`](examples/basic-loop.ts) — Minimal loop with simulated LLM
- [`examples/custom-topics.ts`](examples/custom-topics.ts) — Custom topic list with weighted selection
- [`examples/with-compression.ts`](examples/with-compression.ts) — kompress-ultra integration
- [`examples/nix-flake.nix`](examples/nix-flake.nix) — Standalone flake for self-hosting

## The Recursive Loop (v1.1)

The loop is recursive: the app that generates the data also consumes the model trained on that data.

```
dogfeed generates data → publishes to HF → kompress-v8 trained
    ↑                                          │
    └── kompress-v8 compresses loop context ◀──┘
```

Better compression → more history → better topic selection → higher quality data → better model.

## Self-Hosting

See [GUIDE.md](GUIDE.md) for the full self-hosting and contribution guide.

## Package Structure

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
│   ├── scrub.ts          # PII scrubbing + dedup
│   └── types.ts          # All types
├── test/                 # Test suite
├── examples/             # Usage examples
├── GUIDE.md              # Self-hosting guide
├── SPEC.md               # Full design spec
└── package.json
```

## License

MIT
