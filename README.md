# UltrameshAI

Loop-Engineering Agent Stack substrate for high-performance, decentralized capability-aware agent routing and lifecycle management on cloud VMs and Raspberry Pis.

---

## 🏗️ 4-Layer Architecture

```
+-------------------------------------------------------------+
| 1. COGNITION LAYER   (LLM Client, Session, Prompt Routing)  |
+-------------------------------------------------------------+
                              │
                              ▼
+-------------------------------------------------------------+
| 2. ORCHESTRATION LAYER (Topology Mapping & Lock-free Route)  |
+-------------------------------------------------------------+
                              │
                              ▼
+-------------------------------------------------------------+
| 3. TRANSPORT LAYER     (UDP Multicast, Framed UDS Protobuf) |
+-------------------------------------------------------------+
                              │
                              ▼
+-------------------------------------------------------------+
| 4. EXECUTION LAYER    (Nushell sandbox unit-harness, Bun)   |
+-------------------------------------------------------------+
```

### 1. Cognition Layer (`crates/cognition`, `crates/agent-core`)
Wraps OpenAICompatible models (e.g. DashScope/Qwen) and handles prompting dispatch. Features a unified `LlmClient` capable of running real model completion or offline mock testing.

### 2. Orchestration Layer (`crates/orchestrator`, `crates/node-registry`)
* **Cluster Topology Router:** Parses `topology.toml`, managing capability-aware targets. Uses `arc-swap` for lock-free, ultra-low latency routing requests from Layer 1.
* **Directory Watcher Hot-Reloading:** Uses recommended OS filesystem watchers (via `notify`) to dynamically reload node updates without restarting the binary.
* **Rig Agent Tool:** Exposes `ClusterRouterTool` implementing the `rig::tool::Tool` trait for integration with LLM agent loops.
* **Heartbeat Registry:** Decentralized node advertising over UDP multicast.

### 3. Transport Layer (`crates/transport`)
Handles length-delimited framed protobuf serialization over Unix Domain Sockets (UDS) for pipelined high-throughput message processing.

### 4. Execution Layer (`scripts/`)
Nushell lifecycle harness (`unit-harness.nu`) managing spawning, memory watching (cgroups/polling), and stats reporting of runtime sandboxes.

---

## 🛠️ Getting Started

### 📋 Prerequisites
* Nix package manager with flakes enabled
* Rust (Rust 1.75+ toolchain)
* Nushell (for harness scripts)

### 🚀 Commands

#### 1. Warming Nix Cache & Checking Shells
```bash
# Verify flake evaluates cleanly
nix flake check

# Warm shell caches (required before benchmarking)
nix build .#agent-unit --no-link

# Spawn standard developer shell
nix develop .#agent-unit --command nu -c "version"
```

#### 2. Running the Test Suite
```bash
# Run all workspace test suites (excluding telemetry DB tests)
cargo test --workspace --exclude mempalace

# Test orchestrator specifically with Rig features enabled
cargo test -p loop-engineering-orchestrator --features rig
```

---

## ⚙️ Configuration (`topology.toml`)

Create a local `topology.toml` file in the root or execution directory to define coordinator and target compute resources:

```toml
# Coordinator configurations
coordinator_ip = "100.64.0.1"
coordinator_port = 8080

# Heterogeneous target nodes (Hetzner compute nodes, Raspberry Pis, etc.)
[[nodes]]
ip = "100.64.0.10"
role = "Compute"
capabilities = ["cuda", "llm-inference"]
runtime = "RustNative"

[[nodes]]
ip = "100.64.0.20"
role = "Worker"
capabilities = ["sensors", "arm64", "camera"]
runtime = "Bun"
```
Updates made to this file are automatically picked up by the background file watcher.
