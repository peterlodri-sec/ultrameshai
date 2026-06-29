---
title: Ultrawhale Dogfood
license: apache-2.0
language:
- en
tags:
- token-compression
- context-pruning
- kompress
- kompress-ultra
- agent-loops
- modernbert
- reinforcement-learning
- asymmetric-loss
- voting-ensemble-paradox
- llm-efficiency
- prompt-compression
- dataset-viewer
- token-classification
- text-generation
pretty_name: Ultrawhale Dogfood Corpus
size_categories:
- 10K<n<100K
task_categories:
- token-classification
- text-generation
pipeline_tag: token-classification
viewer: true
configs:
- config_name: default
  data_files:
  - split: train
    path: dogfeed.parquet
---

<div align="center">

# Ultrawhale Dogfood

### A Silver-Label Corpus for Asymmetric Context-Pruning in LLM Agent Loops

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg?style=flat-square)](https://www.apache.org/licenses/LICENSE-2.0)
[![Format: Parquet](https://img.shields.io/badge/Format-Parquet-8B5CF6?style=flat-square)](#dataset-structure)
[![Size: 45K Turns](https://img.shields.io/badge/45K-Turns-10B981?style=flat-square)](#overview)
[![Model: kompress-v8](https://img.shields.io/badge/Model-kompress--v8-06B6D4?style=flat-square)](https://huggingface.co/PeetPedro/kompress-v8)
[![Paper](https://img.shields.io/badge/Paper-kompress.vaked.dev-EC4899?style=flat-square)](https://kompress.vaked.dev/paper/main.pdf)
[![Proposal](https://img.shields.io/badge/Proposal-proposal.vaked.dev-F59E0B?style=flat-square)](https://proposal.vaked.dev)

*Training data for the [kompress-v8](https://huggingface.co/PeetPedro/kompress-v8) model — 149M-parameter ModernBERT achieving **78.5% token savings** with **0.993 exact-keep rate** on critical tokens.*

</div>

---

## Overview

**Ultrawhale Dogfood** is a high-fidelity silver-label corpus of verbose agent conversation histories paired with their pruned counterparts. Each example contains:

- **Raw input**: Multi-turn agent logs, compiler output, tool execution results, code diffs
- **Pruned target**: Non-essential tokens evicted, $T_{\text{crit}}$ safety-floor tokens preserved
- **Token-level labels**: Binary eviction flags per token (keep=1, evict=0)

Generated during self-improving dogfeeding runs of the [opencode](https://opencode.ai) agent framework, this corpus trains token-level context pruners that power [kompress-ultra](https://github.com/peterlodri-sec/kompress-ultra).

### Key Metrics

| Metric | Value |
|--------|-------|
| Total turns | 45,000+ |
| Unique topics | 7 (code_diff, log_stream, json_tool_output, agent_error, bash_output, file_read, error_trace) |
| Avg. compression ratio | 3.2x |
| Critical token preservation rate | 99.3% |
| Source framework | opencode agent loops |

---

## Interactive Data Studio

Explore the dataset splits, filter by pipeline role, and inspect token-level eviction labels:

**[Open Data Studio →](https://huggingface.co/datasets/PeetPedro/ultrawhale-dogfood/viewer)**

---

## Mathematical Foundation

The training targets in this dataset are generated under an asymmetric loss formulation that resolves the **Voting Ensemble Paradox**:

### The Paradox

Under unanimity-to-keep (AND) voting, the ensemble eviction indicator equals the pointwise maximum of individual voter indicators:

$$I_{\text{ens}}(x) = \bigvee_{i=1}^N I_i(x) = I_{i^*_k}(x)$$

> **Notation:** $i^*_k = \arg\min_{i \in [N]} \text{recall}_i$ — the weakest voter on each stratum sets the ensemble recall floor.

### The Fix

An asymmetric loss penalty ($\lambda = 3.0$) on false eviction of critical-syntactic tokens ($T_{\text{crit}}$):

$$\mathcal{L}_i = \mathcal{L}_{\text{base}}(\theta_i) + \lambda \cdot \frac{1}{|T_{\text{crit}}|} \sum_{x \in T_{\text{crit}}} I^{\text{fe}}_i(x)$$

### What is $T_{\text{crit}}$?

The **critical-syntactic safety floor** protects tokens that are essential for code correctness:

| Token Type | Examples |
|------------|----------|
| File paths | `src/auth.rs`, `lib/utils.ts` |
| Commands | `cargo test`, `bun install` |
| IP addresses | `100.64.0.1` |
| Secrets | `{{SECRET_KEY}}` |
| Docker hashes | `sha256:d8a5a...` |
| Memory addresses | `0x7ffee3...` |

---

## Dataset Structure

### Splits

| Split | Format | Description |
|-------|--------|-------------|
| `train` | Parquet | Main training split (dogfeed.parquet) |
| `loop-*.jsonl` | JSONL | Individual dogfeeding loop generations |

### Schema

Each row contains:

| Field | Type | Description |
|-------|------|-------------|
| `text` | `string` | Raw verbose input (stack traces, terminal output, multi-turn history) |
| `reference` | `string` | Target pruned text with $T_{\text{crit}}$ tokens preserved |
| `role` | `string` | Pipeline role: `pruner`, `rewriter`, or `composer` |
| `source` | `string` | Domain source of the interaction |
| `topic` | `string` | Technical category (see topics above) |

### Example

```json
{
  "text": "Compiling auth module...\nwarning: unused variable `temp`\n  --> src/auth.rs:42:9\n   |\n42 |     let temp = verify_token(token);\n   |         ^^^^ help: if this is intentional, prefix it with _\n\nerror[E0597]: `token` does not live long enough\n  --> src/auth.rs:58:20\n   |\n58 |     let session = Session::new(&token);\n   |                    ^^^^^^^^^^^^^^ borrowed value does not live long enough\nFor more information, try `--help E0597`.",
  "reference": "error[E0597]: `token` does not live long enough --> src/auth.rs:58",
  "role": "pruner",
  "source": "opencode-agent",
  "topic": "error_trace"
}
```

---

## Quick Start

### Load with HuggingFace Datasets

```python
from datasets import load_dataset

# Load the full dataset
dataset = load_dataset("PeetPedro/ultrawhale-dogfood")

# Access training split
train = dataset["train"]

# Inspect first example
print(train[0])

# Filter by topic
code_diffs = train.filter(lambda x: x["topic"] == "code_diff")

# Filter by role
pruned = train.filter(lambda x: x["role"] == "pruner")
```

### Compute Compression Stats

```python
def compression_ratio(example):
    original_len = len(example["text"].split())
    pruned_len = len(example["reference"].split())
    return {"ratio": original_len / max(pruned_len, 1)}

ratios = train.map(compression_ratio)
avg_ratio = sum(r["ratio"] for r in ratios) / len(ratios)
print(f"Average compression ratio: {avg_ratio:.1f}x")
```

### Fine-tune kompress-v8

```bash
# Clone the training repo
git clone https://github.com/peterlodri-sec/ultrameshai
cd ultrameshai

# Run fine-tuning (requires GPU)
cargo run --release --bin kompress-train -- \
  --dataset PeetPedro/ultrawhale-dogfood \
  --base-model AnswerDotAI/ModernBERT-base \
  --lambda-penalty 3.0 \
  --epochs 3
```

---

## Ecosystem

This dataset is part of the **UltrameshAI** research ecosystem:

| Component | Description | Link |
|-----------|-------------|------|
| **kompress-v8** | Trained model (149M ModernBERT) | [HuggingFace](https://huggingface.co/PeetPedro/kompress-v8) |
| **kompress-ultra** | Context compression middleware | [GitHub](https://github.com/peterlodri-sec/kompress-ultra) |
| **Proposal** | Integration proposal for Headroom | [proposal.vaked.dev](https://proposal.vaked.dev) |
| **Paper** | Mathematical proof & methodology | [PDF](https://kompress.vaked.dev/paper/main.pdf) |
| **Log Vault** | Experiment telemetry | [pocoo.vaked.dev](https://pocoo.vaked.dev) |
| **UltrameshAI** | Core monorepo | [GitHub](https://github.com/peterlodri-sec/ultrameshai) |

---

## Licensing & Citation

This dataset is released under the **[Apache 2.0 License](https://www.apache.org/licenses/LICENSE-2.0)**.

If you use this dataset in your research, please cite:

```bibtex
@article{lodri2026kompress,
  title={Asymmetric Loss Modulation Resolves the Voting Ensemble Paradox in Learned Context-Pruning Ensembles},
  author={Lodri, Peter},
  journal={Vaked Research Preprints},
  volume={2},
  pages={112--128},
  year={2026},
  url={https://kompress.vaked.dev/paper/main.pdf}
}
```

---

## Privacy & Security

- **Zero PII**: Generation pipeline actively redacts credentials, private keys, and personal data
- **No Tracking**: Cloudflare-hosted with zero advertising scripts or third-party cookies
- **Cryptographic Attestation**: Dataset card backed by P-256 ECDSA signatures via Web Crypto API
- **Open Telemetry**: Runtime metrics publicly logged at [pocoo.vaked.dev](https://pocoo.vaked.dev)

---

## Contribute / Self-Host

This dataset and the full kompress pipeline are open-source. Here's how to run your own dogfeeding loop:

### 1. Generate Your Own Dogfood

```bash
# Clone the ultrameshai repo
git clone https://github.com/peterlodri-sec/ultrameshai
cd ultrameshai

# Install Nushell (for the dev harness)
curl -sSf https://raw.githubusercontent.com/nushell/nushell/main/install.sh | sh

# Run the dogfeeding loop (generates agent conversation logs)
nu mesh.nu status          # Check environment
nu mesh.nu test             # Verify everything works
```

### 2. Label with kompress-ultra

```bash
cd packages/kompress-ultra

# Install dependencies
bun install

# Score and label your conversation logs
import { scoreMessage, classifyMessage } from 'kompress-ultra';

const yourLogs = [...]; // Your agent conversation history
const labeled = yourLogs.map(msg => ({
  text: msg.content,
  score: scoreMessage(msg.content, msg.role, yourLogs.length),
  label: classifyMessage(msg.content), // FACT, EVENT, INSTRUCTION, TASK
}));
```

### 3. Fine-tune Your Own Model

```bash
# Using the HF dataset as reference format
python train.py \
  --dataset your-labeled-data.parquet \
  --base-model AnswerDotAI/ModernBERT-base \
  --lambda-penalty 3.0 \
  --output-dir ./my-kompress-model
```

### 4. Deploy as MCP Server

```bash
cd packages/kompress-ultra/server

# Configure Cloudflare Worker
cp wrangler.toml.example wrangler.toml
# Edit with your Cloudflare account ID

# Deploy to your own domain
bunx wrangler deploy

# Your API is now live at:
# POST /v1/compress  — compress conversations
# POST /v1/score     — score message importance
# GET  /v1/health    — circuit breaker status
```

### 5. Integrate into Your Agent

```typescript
// Add to your agent framework
import { compressMessage, CompressionLevel } from 'kompress-ultra';

// Before sending to LLM
const compressed = compressMessage(
  conversationHistory,
  CompressionLevel.Lite
);

// Send compressed context to LLM
await llm.chat(compressed);
```

### Docker Self-Host (Full Stack)

```bash
# Clone the full stack
git clone https://github.com/peterlodri-sec/ultrameshai
cd ultrameshai

# Start all services
docker-compose up -d

# Services:
# - Portail Gateway: http://localhost:8787
# - Kompress API: http://localhost:8788
# - Second Brain MCP: http://localhost:8789/mcp
```

---

## Acknowledgements

This project would not exist without the incredible open-source ecosystem we build on:

### Core Dependencies

- **[AnswerDotAI/ModernBERT](https://github.com/AnswerDotAI/ModernBERT)** — The backbone architecture for kompress-v8. State-of-the-art encoder optimized for long-context processing.
- **[Hugging Face](https://huggingface.co)** — Dataset hosting, model distribution, and the incredible `datasets` library that makes data loading trivial.
- **[Cloudflare](https://cloudflare.com)** — Zero-trust hosting for our MCP servers, proposal site, and telemetry. Workers, D1, Vectorize, R2 — the entire stack.
- **[Bun](https://bun.sh)** — Blazing-fast JavaScript runtime that powers kompress-ultra and our agent tooling.
- **[Nushell](https://nushell.sh)** — Structured shell for our dev harness. Pipe-oriented, type-safe, and genuinely pleasant to write.

### Agent Infrastructure

- **[OpenCode](https://opencode.ai)** — The agent framework that runs our dogfeeding loops. Plugin architecture, MCP integration, and multi-agent orchestration.
- **[Crush](https://github.com/charmbracelet/crush)** — The AI assistant that helped build this. Charmbracelet's engineering philosophy shows.
- **[Tokio](https://tokio.rs)** — Async runtime for all Rust crates. The foundation of our transport and cognition layers.
- **[Axum](https://github.com/tokio-rs/axum)** — HTTP framework for the portail gateway. Ergonomic, fast, and rock-solid.

### Research Community

- **[Milvus](https://milvus.io)** — Vector database for semantic memory and context circulator.
- **[SQLite](https://sqlite.org)** — Embedded database for mempalace and local state. Sometimes the simplest tool is the best tool.
- **[PyTorch](https://pytorch.org)** — Training framework for kompress-v8 fine-tuning.
- **[HuggingFace Transformers](https://github.com/huggingface/transformers)** — Model loading, tokenization, and inference utilities.

### Inspiration

- The **Voting Ensemble Paradox** paper that started it all — understanding why conservative voting fails in learned pruning ensembles.
- The **agent coding community** — Open-source agents, coding assistants, and the broader movement toward autonomous development tools.
- **Peter's cats** — Who supervised from the keyboard during late-night dogfeeding runs.

---

<div align="center">

**Built by:** [Crush](https://github.com/charmbracelet/crush) + [OpenCode](https://opencode.ai) agent loops

*15 min, 3 rounds, fully autonomous*

[![Star on GitHub](https://img.shields.io/github/stars/peterlodri-sec/ultrameshai?style=social)](https://github.com/peterlodri-sec/ultrameshai)
[![Follow on HuggingFace](https://img.shields.io/badge/Follow-%40PeetPedro-FFD21E?style=social)](https://huggingface.co/PeetPedro)

</div>
