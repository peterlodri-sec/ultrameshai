# Proposal: Integrating `kompress-ultra` Context Management into Headroom
**Author:** Peter Lodri & Antigravity  
**Reference Paper:** [Asymmetric Loss Modulation Resolves the Voting Ensemble Paradox in Learned Context-Pruning Ensembles](https://kompress.vaked.dev/paper/main.pdf) (Lodri et al., 2026)  
**Target Audience:** Headroom Core Maintainers & Contributors  
**Status:** Draft / RFC  

---

## 📋 EXECUTIVE SUMMARY

In long-running agent loops (such as SWE-bench tasks, multi-turn reasoning chains, and autonomous coding cycles), **context window bloat** is the primary driver of latency, cost, and cognitive degradation ("lost-in-the-middle" effects). 

`kompress-ultra` is an intelligent, learned context-pruning engine based on the `kompress-v8` model architecture—a **149M-parameter dual-head ModernBERT** fine-tuned using asymmetric loss modulation. 

This proposal outlines the integration of `kompress-ultra` into **Headroom** (referencing Headroom PR #1419) to provide near-lossless context compression, achieving a **0.993 exact-keep rate** on critical tokens compared to traditional prompt-compression baselines (e.g., `LLMLingua-2`'s 0.867 and `TextRank`'s 0.599).

---

## 1. THE THEORETICAL FOUNDATION

### 1.1 The Representational Impoverishment Problem
As established in the reference paper, context pruning is not semantically neutral. Traditional token-eviction algorithms suffer from **representational impoverishment** on the *critical-syntactic* token class $T_{\text{crit}}$—including identifiers, file paths, exit codes, and delimiters:

> *"A downstream agent cannot reason about a signal name, file path, or exit code that has been evicted from its context; token eviction is therefore an act of representational impoverishment."*

### 1.2 The Voting Ensemble Paradox
A common design pattern for safety in context pruning is to use a multi-checkpoint voting ensemble. However, the paper proves the **Voting Ensemble Paradox**: under unanimity-to-keep (AND) voting ($k=1$ drop-if-any) over checkpoints trained on asymmetric data floors, the ensemble's eviction indicator collapses to the pointwise maximum of the individual voter indicators:

$$I_{\text{ens}}(x) = \bigvee_{i=1}^N I_i(x) = \max_{i \in [N]} I_i(x) = I_{i^*_k}(x)$$

Where $V_{i^*_k}$ represents the weakest voter on a given token stratum $S_k$. This results in a **stratum-wise Pareto collapse**:

$$\text{Recall}_{S_k}(\hat{V}) = \text{Recall}_{S_k}(V_{i^*_k})$$

Empirically, rather than reducing errors, the ensemble acts as a *"Frankenstein of per-stratum worst cases,"* regressing the heretic exact-keep rate to **0.931** (worse than any single model alone, such as `v4` at **0.967**).

---

## 2. THE THREE CORRECTIVE MECHANISMS

To resolve the ensemble paradox and ensure the preservation of critical reasoning tokens, `kompress-ultra` implements three complementary mechanisms defined in the paper:

```
[Mechanism A: Asymmetric Loss]      [Mechanism B: Regex Override]      [Mechanism C: Self-Labeling]
      Training-time 3.0x                  Inference-time surgical            Oracle loop internalizing
   weighted Cross-Entropy penalty            force-keep filter                the regex safety floor
```

### Mechanism A: Asymmetric Loss Modulation
During training, we apply a $\lambda = 3.0$ weighted cross-entropy penalty specifically on the false eviction of critical-syntactic tokens ($T_{\text{crit}}$):

$$\mathcal{L}_i = \mathcal{L}_{\text{base}}(\theta_i) + \lambda \cdot \frac{1}{|T_{\text{crit}}|} \sum_{x \in T_{\text{crit}}} I^{\text{fe}}_i(x)$$

This concentrates gradients on each voter's weakest strata, shrinking their rejection sets and lifting the overall Pareto frontier.

### Mechanism B: Post-Inference Regex Override
A lightweight ($\sim 0.1 \text{ ms}$), training-free inference-time filter that surgically force-keeps Must-Keep patterns (CamelCase, hex addresses, dotted paths, flags):

$$I^{(B)}_i(x) = I_i(x) \land \neg \text{Match}_{\text{MUST\_KEEP\_RE}}(\text{decode}(x))$$

### Mechanism C: Self-Labeling Loop
Uses the combination of $A + B$ as an oracle to relabel the training dataset, allowing subsequent generations of the model to internalize the regex safety floor directly into the model weights, making the inference-time override redundant over time.

---

## 3. THE MODEL ARCHITECTURE: DUAL-HEAD ModernBERT

The production model, `kompress-v8`, utilizes a **149M-parameter ModernBERT** backbone paired with a custom dual-head architecture:
1. **Token-Classifier Head ($h_{\text{tok}}$):** Computes per-token eviction logits.
2. **Span-CNN Head ($h_{\text{span}}$):** Computes span-level coherence to prevent token fragmentation.

The two heads are coupled via an **Asymmetric Modulation Gate** which inhibits the eviction of tokens within high-coherence spans:

$$\tilde{I}_i(x) = \sigma\left(\text{logits}_{\text{tok}}(x) - \gamma g(\text{logits}_{\text{span}}(x))\right)$$

---

## 4. EMPIRICAL BENCHMARKS

`kompress-v8` was evaluated against standard prompt-compression baselines on adversarial, must-keep-dense prompts (the *Heretic* benchmark):

| Method | Exact Keep % ($T_{\text{crit}}$) | Keep Rate (Tokens Kept) | Avg. Latency (ms) |
| :--- | :---: | :---: | :---: |
| **`kompress-v8` (Ours, Production)** | **0.993** | 0.936 | **97.0 ms** |
| **`kompress-v8` (Ours, `v4` SSL)** | **0.967** | 0.823 | — |
| **Random Eviction (Floor)** | 0.910 | 0.835 | 0.0 ms |
| **`LLMLingua-2`** | 0.867 | 1.550 | 238.9 ms |
| **`TextRank` (Extractive)** | 0.599 | 0.543 | 23.1 ms |

### Key Observations:
* **LLMLingua-2 Limitation:** Fails to preserve critical identifiers, dropping them in favor of overall token-budget reduction, resulting in a low **0.867** exact-keep rate.
* **TextRank Limitation:** Extractive sentence-level summarization is unsuitable for agent context preservation, dropping high-density token strata entirely.
* **`kompress-v8` Sweet Spot:** Achieves near-perfect must-keep survival (**0.993**) with highly efficient local CPU execution times ($\sim 97 \text{ ms}$).

---

## 5. PROPOSED INTEGRATION PATH FOR HEADROOM

Integrating `kompress-ultra` into Headroom will allow developers to run long-running agent loops at a fraction of the cost and latency while preserving execution safety.

1. **Context Compression Middleware:**
   Add a `ContextCompressor` pipeline step in Headroom that intercepts outgoing payloads and routes them through the local ONNX export of `kompress-v8`.
2. **Deterministic Safety Net (PR #1419):**
   Integrate the regex-driven Must-Keep override as a configurable post-inference filter.
3. **Pluggable Memory Backends:**
   Support local SQLite-based memory tracking (matching the `LoopKit` state kernel) to easily transition pruned context into passive vector storage.
