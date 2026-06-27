// kompress-ultra: DCP plugin for ultrameshai
// /caveman ultra inside — drop articles/filler/hedging, abbreviate prose words,
// code symbols/API names/errors exact, pattern: [thing] [action] [reason].
// resume normal: security warnings, irreversible actions, stop caveman / normal mode.
// backends: milvus-brain (vector similarity), mempalace (stats), honcho (patterns).
import type { Plugin, PluginInput } from "@opencode-ai/plugin";

export interface KompressUltraOptions {
  relevanceThreshold?: number;
  maxMessagesKept?: number;
  milvusUrl?: string;
  mempalaceDb?: string;
  pollIntervalMs?: number;
  adaptiveThreshold?: boolean;
  droppedMessageDigest?: boolean;
  sliceAwareBoost?: boolean;
  /** Show DCP status block after each prune */
  displayPruneStatus?: boolean;
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

// ─── milvus ─────────────────────────────────────────────────────────────────

async function embedText(text: string): Promise<number[] | null> {
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
    });
  } catch {
    // skip
  }
}

async function scoreMessage(
  text: string,
  milvusUrl: string,
  sliceId?: string,
): Promise<number> {
  try {
    const embedding = await embedText(text);
    if (!embedding) return 0.0;

    let baseScore = await queryMilvusSimilarity(embedding, milvusUrl);
    if (sliceId && opts.sliceAwareBoost) {
      const sliceEmbedding = await embedText(`slice:${sliceId}`);
      if (sliceEmbedding) {
        const sliceScore = await queryMilvusSimilarity(sliceEmbedding, milvusUrl);
        baseScore += Math.min(0.15, sliceScore * 0.15);
      }
    }
    return baseScore;
  } catch {
    return 0.0;
  }
}

async function queryMilvusSimilarity(
  embedding: number[],
  milvusUrl: string,
): Promise<number> {
  for (const coll of ["research_findings", "learning_patterns"]) {
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
      });
      if (res.ok) {
        const json = await res.json() as { results?: { distance?: number }[] };
        const d = json.results?.[0]?.distance ?? 0.0;
        if (d > 0) return d;
      }
    } catch {
      // continue
    }
  }
  return 0.0;
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

// ─── Honcho ──────────────────────────────────────────────────────────────────

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

/** Rough token estimate: ~4 chars per token for English, clamp 1-4096. */
function estimateTokens(text: string): number {
  const len = text.length;
  if (len === 0) return 1;
  return Math.max(1, Math.min(4096, Math.ceil(len / 4)));
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

// ─── DCP Status Display ───────────────────────────────────────────────────────

interface DcpStats {
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

interface BrainState {
  status: string;
  patterns_total: number;
  findings_total: number;
  units_processed: number;
  last_data_at_ms: number;
  poll_count: number;
  interval_ms: number;
}

function buildDcpDisplay(stats: DcpStats): Message {
  const saved = stats.tokensPruned - stats.tokensKept;
  const lines = [
    `── DCP ${stats.model} ──`,
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
    _dcp: true,
    _dcpPruneEvent: true,
  } as Message;
}

// ─── Plugin ───────────────────────────────────────────────────────────────────

const opts = { ...DEFAULT_OPTIONS };

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
  const { ...mergedOpts } = { ...DEFAULT_OPTIONS, ...options };

  return {
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

      // Score + split
      const scored = await Promise.all(
        messages.map((msg) => ({
          msg,
          score: scoreMessage(msg.content, mergedOpts.milvusUrl, ctx.sliceId),
        })),
      );
      const scoredResolved = await Promise.all(scored);

      const kept: Message[] = [];
      const dropped: Message[] = [];
      for (const s of scoredResolved) {
        if (s.score >= threshold) kept.push(s.msg);
        else dropped.push(s.msg);
      }

      // Drop weakest until under cap
      const final =
        kept.length > mergedOpts.maxMessagesKept
          ? kept
              .sort((a, b) => {
                const sa = scoredResolved.find((s) => s.msg === a)?.score ?? 0;
                const sb = scoredResolved.find((s) => s.msg === b)?.score ?? 0;
                return sa - sb;
              })
              .slice(-mergedOpts.maxMessagesKept)
          : kept;

      const prunedCount = messages.length - final.length;
      if (prunedCount > 0) {
        // Update history
        state.contextSizeHistory.push(messages.length);
        if (state.contextSizeHistory.length > 10) {
          state.contextSizeHistory = state.contextSizeHistory.slice(-10);
        }

        // Build DCP display block
        const model = (ctx as Record<string, unknown>).model as string || "unknown";
        const tokensPruned = dropped.reduce((sum, m) => sum + estimateTokens(m.content), 0);
        const tokensKept = final.reduce((sum, m) => sum + estimateTokens(m.content), 0);
        const dcpStats: DcpStats = {
          model,
          pruned: prunedCount,
          kept: final.length,
          total: messages.length,
          threshold,
          density,
          history: [...state.contextSizeHistory],
          tokensPruned,
          tokensKept,
        };
        const dcpDisplay = buildDcpDisplay(dcpStats);

        // Append brain liveness to DCP display
        const brainState = await readBrainState();
        if (brainState) {
          const icon = brainState.status === "Alive" ? "🧠" : brainState.status === "Stale" ? "💤" : "❓";
          const age = brainState.last_data_at_ms === 0 ? "never" : `${Math.round((Date.now() - brainState.last_data_at_ms) / 1000)}s ago`;
          dcpDisplay.content += `\n\n${icon} BRAIN ${brainState.status} | patterns:${brainState.patterns_total} findings:${brainState.findings_total} units:${brainState.units_processed} last:${age}`;
        }

        // Inject display message + update context
        (ctx as { messages: Message[] }).messages = [dcpDisplay, ...final];

        await writeCompactionStats(
          mergedOpts.mempalaceDb,
          prunedCount,
          final.length,
        );

        if (mergedOpts.droppedMessageDigest && dropped.length > 0) {
          await writeDroppedDigest(dropped, mergedOpts.milvusUrl);
        }
      }
    },

    "experimental.chat.system.transform": async (
      input: unknown,
      _output: unknown,
    ) => {
      const ctx = input as SystemContext;
      const now = Date.now();

      if (now - state.lastPatternFetch > mergedOpts.pollIntervalMs) {
        const topic =
          ctx.taskGoal ?? ctx.messages?.[0]?.content?.slice(0, 100) ?? "general";
        state.cachedPatterns = await fetchHonchoPatterns(
          mergedOpts.milvusUrl,
          topic,
        );
        state.lastPatternFetch = now;
      }

      const density = computeDensity(ctx.messages ?? []);
      const activeThreshold = mergedOpts.adaptiveThreshold
        ? adaptiveThreshold(density, mergedOpts.relevanceThreshold)
        : mergedOpts.relevanceThreshold;

      const dcpBlock = [
        "## DCP auto-pruning",
        `- threshold ${activeThreshold.toFixed(2)} (density ${density.toFixed(2)})`,
        `- max-kept ${mergedOpts.maxMessagesKept} msg`,
        `- adapt-threshold ${mergedOpts.adaptiveThreshold ? "on" : "off"}`,
        `- slice-boost ${mergedOpts.sliceAwareBoost ? "on" : "off"}`,
        `- dropped-digest ${mergedOpts.droppedMessageDigest ? "on" : "off"}`,
        `- milvus ${mergedOpts.milvusUrl}`,
      ];

      if (state.cachedPatterns.length > 0) {
        dcpBlock.push("", "## honcho patterns");
        state.cachedPatterns.forEach((p) => dcpBlock.push(`- ${p}`));
      }

      // Append brain liveness status
      const brainState = await readBrainState();
      if (brainState) {
        const icon = brainState.status === "Alive" ? "🧠" : brainState.status === "Stale" ? "💤" : "❓";
        const age = brainState.last_data_at_ms === 0 ? "never" : `${Math.round((Date.now() - brainState.last_data_at_ms) / 1000)}s ago`;
        dcpBlock.push("", `${icon} BRAIN ${brainState.status} | patterns:${brainState.patterns_total} findings:${brainState.findings_total} units:${brainState.units_processed} last:${age}`);
      }

      ctx.systemPrompt = (ctx.systemPrompt ?? "") + "\n\n" + dcpBlock.join("\n");
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