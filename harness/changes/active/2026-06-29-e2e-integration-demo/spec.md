# E2E Integration Demo — Spec

## Goal

Create an integration test harness that exercises the node-registry daemon + transport layer + mempalace as a single running system. This validates the wiring: HTTP API serving, heartbeat reception, node registration, and health reporting.

## Non-Goals

- No LLM or agent loop integration (Phase 1+ scope)
- No multi-node mesh (requires headscale)
- No milvus vector store (standalone)
- No spawn benchmark (requires nix on Linux)

## Architecture

```
                                   ┌───────────────┐
  curl ──POST /heartbeat──►        │               │
  curl ──GET  /health─────►        │ node-registry │
  curl ──GET  /nodes──────►        │  (axum :3000) │
                                   │               │
                                   └───────┬───────┘
                                           │ Arc<Mutex<NodeRegistry>>
                                           ▼
                                   ┌───────────────┐
                                   │  Mempalace     │
                                   │  (SQLite DB)   │
                                   └───────────────┘
```

## Interfaces

### Inputs
- `POST /heartbeat` with JSON body + HMAC-SHA256 signature header
- `GET /health` returns JSON health status
- `GET /nodes` returns JSON node list

### Outputs
- Process exit code 0 on clean shutdown
- Integration test report (JSON summary)

## Dependencies

- `crates/node-registry` (HTTP API, crypto, types, registry)
- `curl` (test client)
- Port 3000 free on localhost
- `HEARTBEAT_SECRET` env var for signed payloads
