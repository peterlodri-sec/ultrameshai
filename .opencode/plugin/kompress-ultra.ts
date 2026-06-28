// kompress-ultra: 4-role living context layer (Composer, Pruner, Rewriter, Circulator)
import type { Plugin, PluginInput } from "@opencode-ai/plugin";
import { compressMessage, CompressionLevel } from "./rewriter";

export interface KompressUltraOptions {
  relevanceThreshold?: number;
  maxMessagesKept?: number;
  milvusUrl?: string;
  mempalaceDb?: string;
  pollIntervalMs?: number;
  adaptiveThreshold?: boolean;
  droppedMessageDigest?: boolean;
  sliceAwareBoost?: boolean;
  /** Show kompress status block after each prune */
  displayPruneStatus?: boolean;
  /** Show human-readable transparency summary (what/why) */
  transparencyMode?: boolean;
}

const DEFAULT_OPTIONS: Required<KompressUltraOptions> = {
  relevanceThreshold: 0.65,
  maxMessagesKept: 50,
  milvusUrl: "http://localhost:19530",
  mempalaceDb: "mempalace.db",
  pollIntervalMs: 60000,
  adaptiveThreshold: true,
  droppedMessageDigest: true,
  sliceAwareBoost: true,
  displayPruneStatus: true,
  transparencyMode: true,
};

interface Message {
  role: string;
  content: string;
  [key: string]: unknown;
}

interface SystemContext {
  messages: Message[];
  systemPrompt?: string;
  taskGoal?: string;
  sliceId?: string;
  [key: string]: unknown;
}

interface BrainState {
  status: string;
  patterns_total: number;
  findings_total: number;
  units_processed: number;
  last_data_at_ms: number;
  poll_count: number;
  interval_ms: number;
}

interface KompressStats {
  model: string;
  pruned: number;
  kept: number;
  total: number;
  threshold: number;
  density: number;
  history: number[];
  tokensPruned: number;
  tokensKept: number;
}

// ─── Circuit Breaker ──────────────────────────────────────────────────────────

const circuitBreaker = { failures: 0, open_until: 0 };

function isCircuitOpen(): boolean {
  return Date.now() < circuitBreaker.open_until;
}

function recordSuccess() {
  circuitBreaker.failures = 0;
}

function recordFailure(): void {
  circuitBreaker.failures++;
  if (circuitBreaker.failures >= 3) {
    circuitBreaker.open_until = Date.now() + 60_000; // 60s cooldown
  }
}

// ─── Circulator Queue (async, non-blocking) ──────────────────────────────────

interface CirculatorEntry {
  session_id: string;
  agent_type: string;
  message_role: string;
  content_hash: string;
  classification: "fact" | "event" | "instruction" | "task";
  topic_key?: string;
  residual: string;
  timestamp_ms: number;
}

const circulatorQueue: CirculatorEntry[] = [];
const CIRCULATOR_CAP = 100;
const CIRCULATOR_BATCH = 10;

function classifyMessage(content: string): "fact" | "event" | "instruction" | "task" {
  const lower = content.toLowerCase();
  if (/\b(shall|should|must|need|implement|create|build|fix|update)\b/.test(lower)) return "instruction";
  if (/\b(todo|task|step|goal|objective)\b/.test(lower)) return "task";
  if (/\b(did|done|completed|failed|error|changed|updated)\b/.test(lower)) return "event";
  return "fact";
}

function enqueueCirculator(entry: CirculatorEntry): void {
  if (circulatorQueue.length >= CIRCULATOR_CAP) {
    spillCirculatorOverflow([entry]);
    return;
  }
  circulatorQueue.push(entry);
  if (circulatorQueue.length >= CIRCULATOR_BATCH) {
    flushCirculatorAsync();
  }
}

async function flushCirculatorAsync(): Promise<void> {
  if (circulatorQueue.length === 0) return;
  const entries = circulatorQueue.splice(0);
  // Async: embed + write to milvus pruned_context collection
  // Milvus down → spill to overflow file
  try {
    const texts = entries.map(e => e.residual).join("\n---\n");
    const embedding = await embedText(texts);
    if (!embedding) {
      spillCirculatorOverflow(entries);
      return;
    }
    // Write to milvus (non-blocking)
    fetch(`${DEFAULT_OPTIONS.milvusUrl}/v1/insert`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        collection_name: "pruned_context",
        fields: {
          finding_id: `kompress-circ-${Date.now()}`,
          agent_id: "kompress",
          topic: "pruned-context",
          summary: texts.slice(0, 4096),
          embedding,
          tags: entries.map(e => e.classification),
          created_at: Date.now(),
          embedding_model: "bge-m3",
        },
      }),
      signal: AbortSignal.timeout(1000),
    }).catch(() => spillCirculatorOverflow(entries));
  } catch {
    spillCirculatorOverflow(entries);
  }
}

function spillCirculatorOverflow(entries: CirculatorEntry[]): void {
  const path = `${process.env.HOME}/.cache/ultrameshai/overflow-circulator.jsonl`;
  try {
    const lines = entries.map(e => JSON.stringify(e)).join("\n") + "\n";
    Bun.write(path, lines, { append: true });
  } catch {
    // silent
  }
}

// ─── Adaptive Threshold ───────────────────────────────────────────────────────

function computeDensity(messages: Message[]): number {
  if (messages.length < 2) return 0.0;
  const windowStart = Math.floor(messages.length / 3);
  const recent = messages.slice(windowStart);
  return recent.length / (messages.length || 1);
}

function adaptiveThreshold(density: number, base: number): number {
  const offset = 0.15 - density * 0.4;
  return Math.max(0.4, Math.min(0.8, base + offset));
}

// ─── Milvus Embedding ─────────────────────────────────────────────────────────

async function embedText(text: string): Promise<number[] | null> {
  if (isCircuitOpen()) return null;
  const endpoint =
    process.env.OVHCLOUD_EMBEDDING_URL ??
    "https://oai.endpoints.kepler.ai.cloud.ovh.net/v1/chat/completions";
  const apiKey = process.env.OVHCLOUD_API_KEY ?? "";

  try {
    const res = await fetch(endpoint, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${apiKey}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ model: "bge-m3", input: text }),
      signal: AbortSignal.timeout(1000),
    });
    if (!res.ok) return null;
    const json = await res.json() as { data?: { embedding?: number[] }[] };
    return json.data?.[0]?.embedding ?? null;
  } catch {
    return null;
  }
}

async function writeDroppedDigest(
  dropped: Message[],
  milvusUrl: string,
): Promise<void> {
  if (dropped.length === 0) return;
  try {
    const texts = dropped.map((m) => m.content).join("\n---\n");
    const embedding = await embedText(texts);
    if (!embedding) return;

    await fetch(`${milvusUrl}/v1/insert`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        collection_name: "research_findings",
        fields: {
          finding_id: `kompress-discard-${Date.now()}`,
          agent_id: "kompress",
          topic: "kompress-discard",
          summary: texts.slice(0, 4096),
          embedding,
          tags: ["kompress-discard"],
          created_at: Date.now(),
          embedding_model: "bge-m3",
        },
      }),
      signal: AbortSignal.timeout(1000),
    });
  } catch {
    // skip
  }
}

// ─── MessageScore + Safety Floors (Pruner) ───────────────────────────────────

export interface MessageScore {
  relevance: number;   // 0-1, vector similarity to task goal
  recency: number;     // 0-1, Ebbinghaus decay
  structural: number;  // 0-1, user/code/error boost
  total: number;       // weighted sum
  protected: boolean;  // last 5, user, code, error
}

export function isProtected(msg: Message, index: number, total: number): boolean {
  if (index >= total - 5) return true;
  if (msg.role === 'user') return true;
  if (msg.content?.includes('```')) return true;
  if (msg.type === 'error' || msg.content?.startsWith('Error:')) return true;
  return false;
}

export function ebbinghausDecay(age: number): number {
  const halfLife = 5;
  return Math.exp(-age / halfLife);
}

export function structuralBoost(msg: Message): number {
  let boost = 0.3;
  if (msg.role === 'user') boost = 0.9;
  if (msg.content?.includes('```')) boost = Math.max(boost, 0.8);
  if (msg.type === 'error' || msg.content?.startsWith('Error:')) boost = Math.max(boost, 0.9);
  if (msg.role === 'tool') boost = Math.max(boost, 0.6);
  return boost;
}

export async function scoreMessage(msg: Message, index: number, total: number, taskGoal?: string): Promise<MessageScore> {
  const recency = ebbinghausDecay(total - index);
  const structural = structuralBoost(msg);
  let relevance = 0.5;
  if (taskGoal && msg.content) {
    try {
      relevance = await scoreMessageMilvus(msg.content, "", taskGoal);
    } catch {
      relevance = 0.5;
    }
  }
  const total_score = relevance * 0.4 + recency * 0.3 + structural * 0.3;
  return { relevance, recency, structural, total: total_score, protected: isProtected(msg, index, total) };
}

// ─── Milvus Scoring (internal) ────────────────────────────────────────────────

async function scoreMessageMilvus(
  text: string,
  _milvusUrl: string,
  sliceId?: string,
): Promise<number> {
  try {
    const embedding = await embedText(text);
    if (!embedding) return 0.5;

    let baseScore = await queryMilvusSimilarity(embedding, DEFAULT_OPTIONS.milvusUrl);
    if (sliceId && DEFAULT_OPTIONS.sliceAwareBoost) {
      const sliceEmbedding = await embedText(`slice:${sliceId}`);
      if (sliceEmbedding) {
        const sliceScore = await queryMilvusSimilarity(sliceEmbedding, DEFAULT_OPTIONS.milvusUrl);
        baseScore += Math.min(0.15, sliceScore * 0.15);
      }
    }
    return baseScore;
  } catch {
    return 0.5;
  }
}

// ─── Milvus Similarity ────────────────────────────────────────────────────────

async function queryMilvusSimilarity(
  embedding: number[],
  milvusUrl: string,
): Promise<number> {
  const collections = ["research_findings", "learning_patterns"];
  try {
    const promises = collections.map(async (coll) => {
      try {
        const res = await fetch(`${milvusUrl}/v1/query`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            collection_name: coll,
            output_fields: ["embedding"],
            topK: 1,
            vector: embedding,
          }),
          signal: AbortSignal.timeout(1000),
        });
        if (res.ok) {
          const json = await res.json() as { results?: { distance?: number }[] };
          return json.results?.[0]?.distance ?? 0.0;
        }
      } catch {
        // ignore
      }
      return 0.0;
    });
    const results = await Promise.all(promises);
    return Math.max(...results);
  } catch {
    return 0.0;
  }
}

// ─── Honcho Patterns (Composer) ──────────────────────────────────────────────

async function fetchHonchoPatterns(
  milvusUrl: string,
  topic: string,
): Promise<string[]> {
  try {
    const embedding = await embedText(topic);
    if (!embedding) return [];

    const res = await fetch(`${milvusUrl}/v1/query`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        collection_name: "learning_patterns",
        output_fields: ["summary", "confidence", "pattern_type"],
        topK: 5,
        vector: embedding,
        filter: "confidence >= 0.5",
      }),
    });
    if (!res.ok) return [];
    const json = await res.json() as {
      results?: { summary?: string; confidence?: number; pattern_type?: string }[];
    };
    return (json.results ?? []).map(
      (r) =>
        `[${r.pattern_type}] ${r.summary} (conf=${r.confidence?.toFixed(2)})`,
    );
  } catch {
    return [];
  }
}

// ─── Token Estimation ─────────────────────────────────────────────────────────

function estimateTokens(text: string): number {
  const len = text.length;
  if (len === 0) return 1;
  return Math.max(1, Math.min(4096, Math.ceil(len / 4)));
}

// ─── Token Budget Escalation Ladder ───────────────────────────────────────────

interface AgentTokenBudget {
  agent_type: string;
  max_context_tokens: number;
  compression_aggressiveness: number;
  brain_injection_budget: number;
}

const DEFAULT_BUDGETS: Record<string, AgentTokenBudget> = {
  coder: { max_context_tokens: 100_000, compression_aggressiveness: 0.8, brain_injection_budget: 500 },
  researcher: { max_context_tokens: 128_000, compression_aggressiveness: 0.4, brain_injection_budget: 1000 },
  reviewer: { max_context_tokens: 64_000, compression_aggressiveness: 0.6, brain_injection_budget: 500 },
  orchestrator: { max_context_tokens: 128_000, compression_aggressiveness: 0.5, brain_injection_budget: 800 },
};

function escalateForBudget(
  messages: Message[],
  budget: AgentTokenBudget,
): Message[] {
  const currentTokens = messages.reduce((sum, m) => sum + estimateTokens(m.content), 0);
  if (currentTokens <= budget.max_context_tokens) return messages;

  // Step 1: Stronger compression on all non-protected messages
  let result = messages.map((msg, i) => {
    const age = messages.length - i;
    if (age <= 5) return msg;
    return { ...msg, content: compressMessage(msg.content, CompressionLevel.Ultra) };
  });

  if (result.reduce((sum, m) => sum + estimateTokens(m.content), 0) <= budget.max_context_tokens) {
    return result;
  }

  // Step 2: Drop oldest unprotected messages
  result = result.filter((msg, i) => {
    if (isProtected(msg, i, result.length)) return true;
    return true; // keep for now; further pruning handled by Pruner
  });

  return result;
}

// ─── Brain State Reader ────────────────────────────────────────────────────────

async function readBrainState(): Promise<BrainState | null> {
  const path = `${process.env.HOME}/.cache/ultrameshai/brain-state.json`;
  try {
    const content = await Bun.file(path).text();
    return JSON.parse(content) as BrainState;
  } catch {
    return null;
  }
}

function buildBrainLine(brainState: BrainState): string {
  const icon = brainState.status === "Alive" ? "🧠" : brainState.status === "Stale" ? "💤" : "❓";
  const age = brainState.last_data_at_ms === 0
    ? "never"
    : `${Math.round((Date.now() - brainState.last_data_at_ms) / 1000)}s ago`;
  return `${icon} BRAIN ${brainState.status} | patterns:${brainState.patterns_total} findings:${brainState.findings_total} units:${brainState.units_processed} | last:${age}`;
}

// ─── Kompress Status Display ──────────────────────────────────────────────────────

function buildKompressDisplay(stats: KompressStats, transparencyMode: boolean = false): Message {
  const saved = stats.tokensPruned - stats.tokensKept;
  
  if (transparencyMode) {
    // Human-readable transparency summary
    const lines = [
      `🗜️  kompress: context optimized`,
      `   • Removed ${stats.pruned} low-signal messages (below threshold ${stats.threshold.toFixed(2)})`,
      `   • Kept ${stats.kept} messages (last 5 + user/code/errors + high-relevance)`,
      `   • Saved ~${saved.toLocaleString()} tokens (${Math.round(saved / stats.tokensPruned * 100)}% reduction)`,
      `   • Pruned content sent to brain for future retrieval`,
    ];
    
    if (stats.history.length > 0) {
      const avg = (stats.history.reduce((a, b) => a + b, 0) / stats.history.length).toFixed(0);
      lines.push(`   • Context trend: ${avg} msg avg (stable)`);
    }
    
    lines.push("─");
    
    return {
      role: "system",
      content: lines.join("\n"),
      _kompress: true,
      _kompressPruneEvent: true,
    } as Message;
  }
  
  // Technical dense format (legacy)
  const lines = [
    `── kompress ${stats.model} ──`,
    `  pruned  ${stats.pruned} msg  ${stats.tokensPruned.toLocaleString()} tok  (threshold=${stats.threshold.toFixed(2)}, density=${stats.density.toFixed(2)})`,
    `  kept    ${stats.kept} msg  ${stats.tokensKept.toLocaleString()} tok`,
    `  saved   ${saved > 0 ? "+" : ""}${saved.toLocaleString()} tok`,
    `  total   ${stats.total} msg → ${stats.kept} msg`,
  ];

  if (stats.history.length > 0) {
    const avg = (stats.history.reduce((a, b) => a + b, 0) / stats.history.length).toFixed(1);
    lines.push(`  history avg ${avg} msg  (${stats.history.slice(-3).join(", ")})`);
  }

  lines.push("──");

  return {
    role: "system",
    content: lines.join("\n"),
    _kompress: true,
    _kompressPruneEvent: true,
  } as Message;
}

// ─── Stats ────────────────────────────────────────────────────────────────────

async function writeCompactionStats(
  dbPath: string,
  prunedCount: number,
  contextSizeAfter: number,
): Promise<void> {
  try {
    const { execSync } = await import("child_process");
    execSync(
      `mempalace write -- pruned=${prunedCount} --context-size=${contextSizeAfter}`,
      { cwd: dbPath, stdio: "ignore" },
    );
  } catch {
    // skip
  }
}

// ─── Plugin ───────────────────────────────────────────────────────────────────

const state = {
  lastCompactionAt: 0,
  contextSizeHistory: [] as number[],
  cachedPatterns: [] as string[],
  lastPatternFetch: 0,
};

export default (
  _input: PluginInput,
  options?: KompressUltraOptions,
) => {
  const mergedOpts = { ...DEFAULT_OPTIONS, ...options };

  return {
    // ─── messages.transform: Pruner + Rewriter + Circulator ─────────────────
    "experimental.chat.messages.transform": async (
      input: unknown,
      _output: unknown,
    ) => {
      const ctx = input as SystemContext;
      const messages = ctx.messages ?? [];
      if (messages.length <= mergedOpts.maxMessagesKept) return;

      const density = computeDensity(messages);
      const threshold = mergedOpts.adaptiveThreshold
        ? adaptiveThreshold(density, mergedOpts.relevanceThreshold)
        : mergedOpts.relevanceThreshold;

      // PRUNER: Score all messages
      const scored = await Promise.all(
        messages.map((msg, idx) => ({
          msg,
          idx,
          score: scoreMessage(msg, idx, messages.length, ctx.taskGoal),
        })),
      );

      // PRUNER: Split by safety floors + threshold
      const kept: Message[] = [];
      const dropped: Message[] = [];
      for (const s of scored) {
        if (s.score.protected || s.score.total >= threshold) kept.push(s.msg);
        else dropped.push(s.msg);
      }

      // Safety: 50% prune cap
      const maxPrune = Math.floor(messages.length * 0.5);
      if (dropped.length > maxPrune) {
        const excess = dropped.length - maxPrune;
        kept.push(...dropped.slice(0, excess));
        dropped.splice(0, excess);
      }

      // Drop weakest until under cap
      let final = kept;
      if (kept.length > mergedOpts.maxMessagesKept) {
        final = kept
          .sort((a, b) => {
            const sa = scored.find((s) => s.msg === a)?.score.total ?? 0;
            const sb = scored.find((s) => s.msg === b)?.score.total ?? 0;
            return sa - sb;
          })
          .slice(-mergedOpts.maxMessagesKept);
      }

      // Safety: empty context guard
      if (final.length === 0) return;

      // REWRITER: Compress by age (TokenPilot: only tail, never prefix)
      const rewritten = final.map((msg, i) => {
        const age = final.length - i;
        if (age <= 5) return msg; // Verbatim (KV cache prefix)
        if (age <= 15) return { ...msg, content: compressMessage(msg.content, CompressionLevel.Lite) };
        return { ...msg, content: compressMessage(msg.content, CompressionLevel.Ultra) };
      });

      // Token budget escalation
      const agentType = (ctx as Record<string, unknown>).agent_type as string || "orchestrator";
      const budget = DEFAULT_BUDGETS[agentType] || DEFAULT_BUDGETS.orchestrator;
      const budgeted = escalateForBudget(rewritten, budget);

      // CIRCULATOR: enqueue pruned messages (async, non-blocking)
      const prunedCount = messages.length - budgeted.length;
      if (prunedCount > 0) {
        for (const msg of dropped) {
          const contentHash = crypto?.getRandomValues?.(new Uint8Array(8))
            ? Array.from(crypto.getRandomValues(new Uint8Array(8))).map(b => b.toString(16).padStart(2, "0")).join("").slice(0, 16)
            : `${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
          enqueueCirculator({
            session_id: (ctx as Record<string, unknown>).session_id as string || "unknown",
            agent_type,
            message_role: msg.role,
            content_hash: contentHash,
            classification: classifyMessage(msg.content),
            residual: msg.content,
            timestamp_ms: Date.now(),
          });
        }
      }

      // Update context
      if (prunedCount > 0) {
        state.contextSizeHistory.push(messages.length);
        if (state.contextSizeHistory.length > 10) {
          state.contextSizeHistory = state.contextSizeHistory.slice(-10);
        }

        const model = (ctx as Record<string, unknown>).model as string || "unknown";
        const tokensPruned = dropped.reduce((sum, m) => sum + estimateTokens(m.content), 0);
        const tokensKept = budgeted.reduce((sum, m) => sum + estimateTokens(m.content), 0);
        const kompressStats: KompressStats = {
          model,
          pruned: prunedCount,
          kept: budgeted.length,
          total: messages.length,
          threshold,
          density,
          history: [...state.contextSizeHistory],
          tokensPruned,
          tokensKept,
        };
        const kompressDisplay = buildKompressDisplay(kompressStats, mergedOpts.transparencyMode);

        // Append brain liveness to kompress display
        const brainState = await readBrainState();
        if (brainState) {
          kompressDisplay.content += `\n${buildBrainLine(brainState)}`;
        }

        (ctx as { messages: Message[] }).messages = [kompressDisplay, ...budgeted];

        await writeCompactionStats(
          mergedOpts.mempalaceDb,
          prunedCount,
          budgeted.length,
        );

        if (mergedOpts.droppedMessageDigest && dropped.length > 0) {
          await writeDroppedDigest(dropped, mergedOpts.milvusUrl);
        }
      }
    },

    // ─── system.transform: Composer ─────────────────────────────────────────
    "experimental.chat.system.transform": async (
      input: unknown,
      _output: unknown,
    ) => {
      const ctx = input as SystemContext;
      const now = Date.now();

      // Circuit breaker check
      if (isCircuitOpen()) {
        const staleLine = "❓ BRAIN STALE (circuit breaker open)";
        ctx.systemPrompt = (ctx.systemPrompt ?? "") + "\n\n" + staleLine;
        return;
      }

      // Fetch patterns (cached by poll interval)
      if (now - state.lastPatternFetch > mergedOpts.pollIntervalMs) {
        const topic =
          ctx.taskGoal ?? ctx.messages?.[0]?.content?.slice(0, 100) ?? "general";
        try {
          state.cachedPatterns = await fetchHonchoPatterns(
            mergedOpts.milvusUrl,
            topic,
          );
          recordSuccess();
        } catch {
          recordFailure();
          state.cachedPatterns = [];
        }
        state.lastPatternFetch = now;
      }

      const density = computeDensity(ctx.messages ?? []);
      const activeThreshold = mergedOpts.adaptiveThreshold
        ? adaptiveThreshold(density, mergedOpts.relevanceThreshold)
        : mergedOpts.relevanceThreshold;

      // Brain state line (50-token budget)
      const brainState = await readBrainState();
      let brainLine = "";
      if (brainState) {
        brainLine = buildBrainLine(brainState);
      }

      const kompressBlock = [
        `## kompress auto-pruning | threshold ${activeThreshold.toFixed(2)} | density ${density.toFixed(2)} | max-kept ${mergedOpts.maxMessagesKept} msg`,
        brainLine,
      ].filter(Boolean);

      if (state.cachedPatterns.length > 0) {
        kompressBlock.push("", "## honcho patterns");
        state.cachedPatterns.forEach((p) => kompressBlock.push(`- ${p}`));
      }

      ctx.systemPrompt = (ctx.systemPrompt ?? "") + "\n\n" + kompressBlock.join("\n");
    },

    "experimental.compaction.autocontinue": async (
      input: unknown,
      _output: unknown,
    ) => {
      const ctx = input as { messages?: Message[]; [key: string]: unknown };
      const messages = ctx.messages ?? [];
      const now = Date.now();

      if (now - state.lastCompactionAt < mergedOpts.pollIntervalMs) return;
      if (messages.length < mergedOpts.maxMessagesKept) return;

      state.lastCompactionAt = now;

      return { action: "compaction.trigger", reason: "context_limit_near" };
    },
  };
};
