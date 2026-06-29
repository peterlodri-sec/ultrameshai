# UltrameshAI

<p align="center">
  <img src="docs/media/social_preview.jpg" alt="UltrameshAI Social Preview" width="100%">
</p>

Loop-Engineering Agent Stack substrate for high-performance, decentralized capability-aware agent routing and lifecycle management on cloud VMs and Raspberry Pis.

---

### 👶 Explain Like I'm 5 (ELI5)

Imagine you have a team of AI friends (agents) living on different computers (like big cloud servers and tiny Raspberry Pis). **UltrameshAI** is the **nervous system** that connects them:
1. **It knows who is best at what:** If one agent is great at writing code and another is great at searching the web, it routes your question to the right agent (**Cognition & Orchestration**).
2. **It talks super fast:** It lets them send messages to each other instantly (**Transport**).
3. **It keeps them in line:** It watches over them so they don't hog too much memory or crash your computer (**Execution**).

---

### 🧠 What is `kompress-ultra`?

When you talk to an AI for a long time, the chat history gets huge. This makes the AI slow, expensive to run, and it starts forgetting things. **`kompress-ultra`** is the **smart memory-manager** that solves this:

* **WHAT:** A smart filter that shrinks the chat history so the AI only reads what is absolutely necessary.
* **HOW:** 
  * **It squeezes sentences:** It removes filler words, turning *"I would be happy to help you write the code for authentication"* into *"write authentication code"* (**Rewriter**).
  * **It throws away the clutter:** It deletes low-value messages but always keeps a critical-syntactic safety floor ($T_{\text{crit}}$) (such as code blocks, errors, path names, and the last 5 messages) so nothing important is lost (**Pruner**).
  * **It files old memories away:** Anything it deletes is saved into a long-term database (Milvus/MemPalace) so the AI can search and recall it later (**Circulator**).
#### Mathematical Safety Floor & Paradox Resolution
To resolve the **Voting Ensemble Paradox** where conservative voting collapses stratum-wise recall:

Under AND-voting, the per-stratum ensemble indicator equals the maximum over voter indicators. That max picks out the **weakest voter on that stratum** — the one most likely to have evicted a critical token — so the ensemble's recall drops to the level of its worst member:

$$I_{\text{ens}}(x) \;=\; \bigvee_{i=1}^{N} I_i(x) \;=\; I_{i^{\*}_k}(x)$$

> **Notation:** \(i^{\*}_k = \arg\min_{i \in [N]} \text{recall}_i\) — the **weakest** voter on stratum \(k\) is the one whose indicator survives the OR. Under AND-voting, the ensemble's stratum-wise recall equals the worst voter's recall on that stratum.

We apply an asymmetric loss penalty (\(\lambda = 3.0\)) on the false eviction of critical-syntactic tokens (\(T_{\text{crit}}\)):

$$\mathcal{L}_i = \mathcal{L}_{\text{base}}(\theta_i) + \lambda \cdot \frac{1}{|T_{\text{crit}}|} \sum_{x \in T_{\text{crit}}} I^{\text{fe}}_i(x)$$

* **WHY:** To keep the agents running **fast, cheap, and smart**—especially on resource-constrained hardware like a Raspberry Pi!
* **EVALUATION:** Achieves up to **78.5% token savings** while maintaining a **0.993 exact-keep rate** on critical reasoning tokens.

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
