# CI Autonomous Agents — Design Proposal

> **Principle:** Never block. Never fail a merge. Open PRs for fixes. The loop starts again.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  GitHub Repository (any repo in peterlodri-sec)        │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐             │
│  │ Chore    │  │ Security │  │ PR       │             │
│  │ Agent    │  │ Agent    │  │ Manager  │             │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘             │
│       │              │              │                   │
│       ▼              ▼              ▼                   │
│  ┌──────────────────────────────────────────┐          │
│  │  GitHub Actions (scheduled + event)      │          │
│  │  - Never runs on PR head commits         │          │
│  │  - Only on schedule or push to main      │          │
│  │  - Opens NEW PRs, never modifies existing│          │
│  └──────────────────────────────────────────┘          │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

## Three Autonomous Agents

### 1. Chore Agent
**Trigger:** `schedule: cron(0 6 * * 1)` (weekly Monday 06:00 UTC)
**What it does:**
- Runs `cargo outdated`, `npm outdated`, `bun outdated`
- Checks for lockfile drift (`cargo update --dry-run`, `bun install --frozen-lockfile` dry run)
- Updates `dogfeed.json` metadata (branch status, crate versions)
- Updates `docs/STATUS.md` if stale (>7 days since last edit)

**Output:** Opens a PR titled `chore: weekly dependency + metadata refresh (YYYY-MM-DD)`
**Safety:**
- Never commits directly to main
- PR runs full CI before becoming mergeable
- Auto-labels: `chore`, `autonomous`, `safe-to-merge`
- If CI passes: auto-approves (but does NOT auto-merge — human decides)

### 2. Security Agent
**Trigger:** `schedule: cron(0 3 * * *)` (daily 03:00 UTC) + `workflow_run` on Dependabot PRs
**What it does:**
- Runs `cargo audit` on all Rust crates
- Runs `npm audit` on kompress-ultra
- Checks for known CVEs in dependencies via GitHub Advisory API
- Scans for hardcoded secrets (`trufflehog`, `gitleaks`)
- Validates `dogfeed.json` constraints (memory caps, protobuf limits)

**Output:** 
- If critical CVE found → opens PR with `security` + `urgent` labels, assigns @peterlodri
- If informational → opens PR with `security` + `chore` labels
- If clean → no PR, just logs to Actions summary

**Safety:**
- Never auto-merges security PRs
- Always includes CVE ID, severity, and fix version in PR body
- Never exposes secrets in logs (uses `::add-mask::`)

### 3. PR Manager
**Trigger:** `pull_request: [opened, synchronize, review_requested]`
**What it does:**
- Validates PR metadata (title format, body completeness)
- Checks if `dogfeed.json` needs updating based on changed files
- Runs `mesh.nu status` equivalent (git status across submodules)
- Adds appropriate labels based on changed files:
  - `*.rs` → `rust`
  - `*.ts` → `typescript`
  - `proto/` → `ipc`
  - `docs/` → `documentation`
  - `Cargo.toml` → `dependencies`
- Posts a summary comment with:
  - Files changed count
  - Estimated review time
  - Related PRs (if any)
  - dogfeed.json impact assessment

**Output:** Labels + summary comment on the PR
**Safety:**
- Read-only on the PR itself (never pushes to PR branches)
- Only adds labels and comments
- Respects existing labels (won't overwrite)

## Implementation

### File Structure
```
.github/
  workflows/
    chore-agent.yml      # Weekly dependency refresh
    security-agent.yml   # Daily security scan
    pr-manager.yml       # PR labeling + summary
  actions/
    dogfeed-sync/
      action.yml         # Reusable action: update dogfeed.json
    pr-summary/
      action.yml         # Reusable action: generate PR summary
```

### Key Design Decisions

1. **Never modify existing PRs** — Agents only open new PRs or add labels/comments
2. **Never auto-merge** — Human decides merge timing
3. **Always pass CI first** — Agent PRs run full CI before becoming mergeable
4. **Idempotent** — Running twice produces same result (no duplicate PRs)
5. **Graceful failure** — If agent can't run, it logs warning and exits 0 (never fails the workflow)

### Deployment
- Each repo gets its own `.github/workflows/` copy
- Shared actions live in `ultrameshai/.github/actions/` and are referenced via `uses: peterlodri-sec/ultrameshai/.github/actions/dogfeed-sync@main`
- Secrets: `GITHUB_TOKEN` (default), `TRUFFLEHOG_TOKEN` (optional, for deeper secret scanning)

## Metrics (tracked in dogfeed.json)

```json
{
  "ci_agents": {
    "chore_agent": { "last_run": "2026-06-29", "prs_opened": 3, "prs_merged": 2 },
    "security_agent": { "last_run": "2026-06-29", "critical_found": 0, "info_found": 2 },
    "pr_manager": { "prs_labeled": 15, "summaries_posted": 15 }
  }
}
```

## Next Steps

1. Create `.github/workflows/chore-agent.yml` in ultrameshai
2. Create `.github/workflows/security-agent.yml` in ultrameshai
3. Create `.github/workflows/pr-manager.yml` in ultrameshai
4. Create reusable actions in `.github/actions/`
5. Test on ultrameshai first, then roll out to other repos
6. Update `dogfeed.json` with CI agent config
