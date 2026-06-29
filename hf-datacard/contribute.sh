#!/usr/bin/env bash
# contribute.sh — Ultrawhale Dogfood contributor mini-CLI
#
# Prints every command a new contributor needs, in copy-paste order.
# Sections are self-contained — pick what you need, skip the rest.
#
# Usage:
#   ./hf-datacard/contribute.sh            # full tour
#   ./hf-datacard/contribute.sh load       # just the dataset load snippet
#   ./hf-datacard/contribute.sh generate   # just the local generation snippet
#   ./hf-datacard/contribute.sh deploy     # just the deploy snippet
#   ./hf-datacard/contribute.sh prompt     # one-shot LLM prompts for contributors
#   ./hf-datacard/contribute.sh mcp        # MCP config for AI coding agents
#   ./hf-datacard/contribute.sh kickstart  # ultra-oneshot entrypoint for any coding agent
#   ./hf-datacard/contribute.sh --raw      # machine-readable (no banners)
#
# https://huggingface.co/datasets/PeetPedro/ultrawhale-dogfood
# https://github.com/peterlodri-sec/ultrameshai

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RAW=0
SECTION="all"

for arg in "$@"; do
  case "$arg" in
    --raw) RAW=1 ;;
    load|generate|deploy|prompt|mcp|kickstart|all) SECTION="$arg" ;;
    -h|--help)
      sed -n '2,17p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *) echo "unknown: $arg" >&2; exit 2 ;;
  esac
done

hr() { [ "$RAW" -eq 1 ] && return 0 || true; printf '\n\033[1;36m━━━ %s ━━━\033[0m\n' "$1"; }
code() { [ "$RAW" -eq 1 ] && { cat; return 0; } || true; printf '\n```%s\n' "${1:-}"; cat; printf '```\n'; }

section_load() {
  hr "load the dataset"
  code "python"
  cat <<'PY'
from datasets import load_dataset

ds = load_dataset("PeetPedro/ultrawhale-dogfood", split="train")
print(ds)
print(ds[0])

# Filter by topic
code_diffs = ds.filter(lambda x: x["topic_category"] == "distributed-systems")

# Filter by role
pruned = ds.filter(lambda x: x["role"] == "pruner")
PY
}

section_generate() {
  hr "generate locally (dogfeed loop)"
  code "bash"
  cat <<'BASH'
# 1. Clone + enter dev shell
git clone https://github.com/peterlodri-sec/ultrameshai
cd ultrameshai
nix develop .#dogfeed   # or: bun + sqlite + jq on PATH

# 2. Run a 10-iteration loop with free models, 60s interval
bun packages/dogfeed/examples/basic-loop.ts

# 3. Inspect stats
nu scripts/dogfeed.nu stats

# 4. Self-host the systemd service (NixOS)
#    Add to your hosts/<name>/default.nix:
#      imports = [ ../../modules/dogfeed.nix ];
#      peterlodri.dogfeed = {
#        enable = true;
#        openrouterKeyFile = "/run/secrets/dogfeed_openrouter_key";
#        hfTokenFile       = "/run/secrets/dogfeed_hf_token";
#        hfRepo            = "PeetPedro/ultrawhale-dogfood";
#        intervalSec       = 60;
#        models            = [ "qwen/qwen-2.5-7b-instruct:free" ];
#        topics            = [ "distributed systems" "machine learning" ];
#        dailyCallLimit    = 500;
#        dailyTokenLimit   = 200000;
#      };
BASH
}

section_deploy() {
  hr "deploy your own fork (nix + huggingface)"
  code "bash"
  cat <<'BASH'
# 1. Create a new HF dataset (mirror of ultrawhale-dogfood, but yours)
#    https://huggingface.co/new-dataset

# 2. Edit flake inputs / dev-cx53 config:
#      peterlodri.dogfeed.hfRepo = "YOUR_USER/YOUR_DATASET";

# 3. Rebuild dev-cx53
ssh dev-cx53.tail2870dc.ts.net \
  "cd ~/nix-base-deploy && nixos-rebuild switch --flake .#dev-cx53 --use-remote-sudo"

# 4. Verify
ssh dev-cx53.tail2870dc.ts.net \
  "systemctl status dogfeed --no-pager -l | head -20 && journalctl -u dogfeed -n 30"
BASH
}

section_prompt() {
  hr "one-shot prompts for AI coding agents"
  code "markdown"
  cat <<'PROMPT'
# A. Generate a sample matching this dataset
> Generate one row matching the Ultrawhale Dogfood schema:
> {id, topic, question, answer, model, tokens_in, tokens_out, role,
>  source, topic_category, created_at}
>
> Topic: `container orchestration`. Role: `pruner`.
> Question should be ~50 tokens, answer ~200 tokens, then a `compressed_answer`
> version at ~60 tokens. Use a free OpenRouter model. Print the JSON row only.

# B. Filter + analyse the dataset
> Load PeetPedro/ultrawhale-dogfood, group by topic_category, and report
> the mean compression ratio (answer vs compressed_answer length).
> Print a markdown table sorted by row count desc.

# C. Self-host the loop
> Add a new module `modules/dogfeed-local.nix` that runs the dogfeed loop
> under my user (no systemd) with a sqlite DB at ~/.local/share/dogfeed/loop.db
> and pushes to my own HF dataset every 50 records. No sops — read the
> OpenRouter key from ~/.config/dogfeed/openrouter.key with mode 0600.
PROMPT
}

section_mcp() {
  hr "MCP config for AI coding agents (Claude / Cursor / opencode)"
  code "json"
  cat <<'JSON'
{
  "mcpServers": {
    "ultrawhale-dogfood": {
      "type": "stdio",
      "command": "uvx",
      "args": [
        "--from", "ultrawhale-dogfood-mcp",
        "ultrawhale-dogfood-mcp",
        "--repo", "PeetPedro/ultrawhale-dogfood",
        "--hf-token", "${env:HF_TOKEN}"
      ]
    }
  }
}
JSON
  hr "MCP tools exposed"
  cat <<'TEXT'
- dogfeed.search(query, k=10)        # semantic + keyword search over rows
- dogfeed.filter(topic, role)        # exact-match filter, returns JSONL
- dogfeed.sample(n=5, seed=42)       # deterministic sample for prompting
- dogfeed.stats()                    # row counts per topic / role / model
- dogfeed.export(format="parquet")   # local parquet mirror
TEXT
}

section_kickstart() {
  hr "ultra-oneshot-kickstart prompt (works from anywhere)"
  code "markdown"
  cat <<'PROMPT'
# ULTRA-ONESHOT-KICKSTART — Ultrawhale Dogfood entrypoint
# Paste this into ANY coding agent (Claude / opencode / Cursor / aider / Cline)
# Works from zero context. ~280 tokens. Self-bootstrapping.

You are joining **dogfeed-loop** — a self-improving silver-label data
generation loop in the ultrameshai ecosystem. Read the README (linked
in the **sources** below) BEFORE making changes. End every reply with
**🐳 loop-state**: one of `{generating, ready, blocked, done}`.

## Sources (read in order, ~5 min)
1. hf-datacard — https://huggingface.co/datasets/PeetPedro/ultrawhale-dogfood
2. ultrameshai — https://github.com/peterlodri-sec/ultrameshai (this repo)
3. kompress-ultra — https://github.com/peterlodri-sec/ultrameshai/tree/main/packages/kompress-ultra
4. nix-base (dev-cx53 host) — https://github.com/peterlodri-sec/nix-base
5. blog (changelog + rationale) — https://pocoo.vaked.dev

## Loop primitives
- **Topic** (string) — current question category
- **Question** (≤ 50 tok) — generated prompt
- **Answer** (raw, 100–300 tok) — free-model output
- **compressed_answer** (≤ 60 tok) — kompress-ultra Lite pass
- **role**: `generator` (raw only) or `pruner` (also has compressed)
- **model** — OpenRouter FQN, e.g. `qwen/qwen-2.5-7b-instruct:free`

## One-shot commands (copy-paste ready)
- `bun test` — 48 tests must pass
- `nu scripts/dogfeed.nu stats` — live loop stats
- `nu scripts/dogfeed.nu doctor` — env + keys + HF reachability
- `nu scripts/dogfeed.nu push --repo OWNER/REPO --batch 50`
- `nix develop .#dogfeed` — reproducible dev shell
- `ssh dev-cx53.tail2870dc.ts.net "journalctl -u dogfeed -n 30"`
- `gh pr create --base main` for any change

## Hard rules
1. **NEVER** edit `flake.lock` by hand — use `nix flake update`.
2. **NEVER** push without `bun test` green.
3. **NEVER** write a row without PII scrub (`scrub.ts` rules).
4. **ALWAYS** mirror `data/latest.jsonl` on every push.
5. **ALWAYS** end replies with `🐳 loop-state: <state>`.
6. **ALWAYS** reference a real HF commit when claiming "pushed".
7. **NEVER** store API keys in git. sops-nix only.
8. **ALWAYS** ship the smallest viable diff.

## Schema (source of truth: `packages/dogfeed/src/publish.ts`)
```
{id, topic, question, answer, compressed_answer?, model, tokens_in,
 tokens_out, role, source:"dogfeed-loop", topic_category, created_at}
```

## If you are blocked
- Re-read the README (top of `hf-datacard/README.md`).
- Run `nu scripts/dogfeed.nu doctor`.
- Search issues: `gh issue list --label dogfeed --state all`.
- Open an issue with the `blocked` label and the failing command.

**🐳 loop-state: ready**
PROMPT
}

case "$SECTION" in
  all)
    section_load
    section_generate
    section_deploy
    section_prompt
    section_mcp
    section_kickstart
    hr "end"
    [ "$RAW" -eq 1 ] || cat <<'NOTE'

Every section above is self-contained — copy the block you need, skip the rest.
Run with `./hf-datacard/contribute.sh <section>` to print just one.

  Sections: load | generate | deploy | prompt | mcp | kickstart
  Flags:    --raw (no banners), -h (this help)

💘 Generated with Crush
NOTE
    ;;
  load)      section_load ;;
  generate)  section_generate ;;
  deploy)    section_deploy ;;
  prompt)    section_prompt ;;
  mcp)       section_mcp ;;
  kickstart) section_kickstart ;;
esac
