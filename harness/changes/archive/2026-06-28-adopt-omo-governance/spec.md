# Requirements

## Functional

### 1. Routing Matrix (`docs/routing-matrix.md`)
- Map task categories to loop types
- Include: SWE-bench, testing, devops, research, infra, quality
- Each entry: task description → recommended loop → fallback loops

### 2. Support Policy (`docs/support-policy.md`)
- Three tiers: `validated`, `guided`, `experimental`
- `validated` = production-tested, has ECL archive, passes quality gates
- `guided` = works but not production-validated
- `experimental` = stub or untested

### 3. Workflow Catalog (`docs/workflow-catalog.md`)
- List validated end-to-end workflows
- Each workflow: name, description, loops involved, success criteria
- Start with 2-3 validated: "Original 10 E2E", "Models TOML config", "Ralph orchestrator"

### 4. CLI Tool (`bin/ultrameshai` or `scripts/ultrameshai.sh`)
- `ecl-lint` — check ECL completeness
- `config-validate` — validate models.toml syntax + all loops mapped
- `doctor` — check workspace health (cargo test, ECL archive count, stale branches)

### 5. Capability Matrix (`docs/capability-matrix.json`)
- JSON with all 48 loops and their support tier
- Machine-readable for future tooling

## Non-functional
- Docs are thin — link to code, don't duplicate
- Support tiers are honest — no marketing fluff
- CLI is Rust or bash — no new dependencies if possible

## Out of scope
- Web UI for routing (future)
- Interactive CLI wizard (future)
- Automated support tier promotion (future)
