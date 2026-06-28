# UltraMeshAI Architecture

UltraMeshAI is a loop-engineered agentic system designed for distributed execution across cloud VMs and Raspberry Pis.

```
+-------------------------------------------------------------+
| 1. COGNITION LAYER   (LLM Client, State, Prompting)         |
+-------------------------------------------------------------+
                              │
                              ▼
+-------------------------------------------------------------+
| 2. ORCHESTRATION LAYER (Topology Mapping & Routing)         |
+-------------------------------------------------------------+
                              │
                              ▼
+-------------------------------------------------------------+
| 3. TRANSPORT LAYER     (Headscale / WireGuard Mesh VPN)     |
+-------------------------------------------------------------+
                              │
                              ▼
+-------------------------------------------------------------+
| 4. EXECUTION LAYER    (Target Node Daemon, Bun Runtimes)    |
+-------------------------------------------------------------+
```

## Crate Layout

*   `crates/agent-core` — Core LLM client (Rig integration), session management, and tool dispatching.
*   `crates/cognition` — Advanced prompting and model routing logic.
*   `crates/orchestrator` — Topology mapping and routing for multi-agent execution.
*   `crates/loops` — Implementation of specialized agent loops (Ralph, Red Team, Juniors, Deepwork).
*   `crates/transport` — Framed protobuf over UDS for secure system-to-system communication.
*   `crates/node-registry` — Centralized store tracking active nodes, capacities, and VPN health.
*   `crates/mempalace` — In-memory SQLite-based state and session storage.
*   `crates/milvus-brain` — Vector memory integration for long-term agent context.
