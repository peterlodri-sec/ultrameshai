# Implementation Plan

## Step 1 — Routing Matrix
Create `docs/routing-matrix.md`:
- Group loops by domain (SWE-bench, testing, devops, quality, memory, infra, research, utility)
- For each: when to use, which model tier, example task

## Step 2 — Support Policy
Create `docs/support-policy.md`:
- Define the 3 tiers (validated, guided, experimental)
- List which loops are in each tier (be honest!)
- Define what it takes to promote a loop (ECL + tests + production use)

## Step 3 — Workflow Catalog
Create `docs/workflow-catalog.md`:
- List 3-5 validated workflows
- Each with: name, loops involved, success criteria, ECL reference

## Step 4 — Capability Matrix
Create `docs/capability-matrix.json`:
- JSON object with all 48 loops
- Each has: `tier`, `model`, `category`, `tested`

## Step 5 — CLI Tool
Create `scripts/ultrameshai.sh`:
- `ecl-lint` — wraps `nu scripts/lint-ecl.nu`
- `config-validate` — checks models.toml parses, all loops mapped
- `doctor` — cargo test, ECL count, stale active changes

## Step 6 — Update AGENTS.md
- Add routing quick-reference
- Link to full routing matrix
- Document support tiers

## Step 7 — Verify
- Run `scripts/ultrameshai.sh doctor`
- All docs render correctly
- ECL lint and archive
