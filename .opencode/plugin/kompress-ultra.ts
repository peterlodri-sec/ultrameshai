// kompress-ultra: DCP plugin for ultrameshai
// /caveman ultra inside — drop articles/filler/hedging, abbreviate prose words,
// code symbols/API names/errors exact, pattern: [thing] [action] [reason].
// resume normal: security warnings, irreversible actions, stop caveman / normal mode.
// backends: milvus-brain (vector similarity), mempalace (stats), honcho (patterns).
import type { Plugin, PluginInput } from "@opencode-ai/plugin";

export interface KompressUltraOptions {
  /** Minimum cosine similarity to keep a message (0.0-1.0) — adaptive if adaptiveThreshold=true */
  relevanceThreshold?: number;
  /** Hard cap on messages to keep in context */
  maxMessagesKept?: number;
  /** milvus server URL */
  milvusUrl?: string;
  /** mempalace SQLite db path */
  mempalaceDb?: string;
  /** honcho pattern poll interval (ms) */
  pollIntervalMs?: number;
  /** Enable adaptive threshold (sparse→0.8, dense→0.4) */
  adaptiveThreshold?: boolean;
  /** Enable dropped-message digest to milvus */
  droppedMessageDigest?: boolean;
  /** Enable slice-aware score boost */
  sliceAwareBoost?: boolean;
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

/** Compute density: messages per recent time window. High density → lower threshold. */
function computeDensity(messages: Message[]): number {
  if (messages.length < 2) return 0.0;
  // Simple density: count messages in last 3rd of conversation
  const windowStart = Math.floor(messages.length / 3);
  const recent = messages.slice(windowStart);
  return recent.length / (messages.length || 1); // 0..1
}

/** Adaptive threshold: sparse → 0.8, dense → 0.4 */
function adaptiveRelevanceThreshold(density: number, base: number): number {
  // density 0 → sparse (use base+0.15), density 1 → dense (use base-0.25)
  const offset = 0.15 - density * 0.4;
  return Math.max(0.4, Math.min(0.8, base + offset));
}

// ─── milvus write (dropped-message digest) ────────────────────────────────────

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

/** Write dropped messages as research_findings to milvus (tag: kompress-discard) */
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
    // Silently skip digest write failures
  }
}

// ─── Scoring ─────────────────────────────────────────────────────────────────

async function scoreMessage(
  text: string,
  milvusUrl: string,
  sliceId?: string,
): Promise<number> {
  try {
    const embedding = await embedText(text);
    if (!embedding) return 0.0;

    // Boost: if sliceId provided, embed slice goal and add bonus
    let baseScore = await queryMilvusSimilarity(embedding, milvusUrl);
    if (sliceId && opts.sliceAwareBoost) {
      const sliceEmbedding = await embedText(`slice:${sliceId}`);
      if (sliceEmbedding) {
        const sliceScore = await queryMilvusSimilarity(sliceEmbedding, milvusUrl);
        baseScore += Math.min(0.15, sliceScore * 0.15); // cap boost at +0.15
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
      // continue to next collection
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
    // Silently skip if mempalace CLI not available
  }
}

// ─── Honcho patterns ──────────────────────────────────────────────────────────

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
  // Merge with defaults at every invocation (hot reload support)
  const { ...mergedOpts } = { ...DEFAULT_OPTIONS, ...options };

  return {
    /** Prune low-relevance messages with adaptive threshold + slice boost + dropped digest */
    "experimental.chat.messages.transform": async (
      input: unknown,
      _output: unknown,
    ) => {
      const ctx = input as SystemContext;
      const messages = ctx.messages ?? [];
      if (messages.length <= mergedOpts.maxMessagesKept) return;

      // 1. Adaptive threshold
      const density = computeDensity(messages);
      const threshold = mergedOpts.adaptiveThreshold
        ? adaptiveRelevanceThreshold(density, mergedOpts.relevanceThreshold)
        : mergedOpts.relevanceThreshold;

      // 2. Score all messages
      const scored = await Promise.all(
        messages.map((msg) => ({
          msg,
          score: scoreMessage(msg.content, mergedOpts.milvusUrl, ctx.sliceId),
        })),
      );
      const scoredResolved = await Promise.all(scored);

      // 3. Split kept vs dropped
      const kept: Message[] = [];
      const dropped: Message[] = [];
      for (const s of scoredResolved) {
        if (s.score >= threshold) kept.push(s.msg);
        else dropped.push(s.msg);
      }

      // 4. If over cap, drop lowest-scoring first
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
        (ctx as { messages: Message[] }).messages = final;
        await writeCompactionStats(
          mergedOpts.mempalaceDb,
          prunedCount,
          final.length,
        );

        // 5. Dropped-message digest
        if (mergedOpts.droppedMessageDigest && dropped.length > 0) {
          await writeDroppedDigest(dropped, mergedOpts.milvusUrl);
        }
      }
    },

    /** Inject kompress directives + honcho patterns + adaptive threshold info */
    "experimental.chat.system.transform": async (
      input: unknown,
      _output: unknown,
    ) => {
      const ctx = input as SystemContext;
      const now = Date.now();

      if (now - state.lastPatternFetch > mergedOpts.pollIntervalMs) {
        const topic = ctx.taskGoal ?? ctx.messages?.[0]?.content?.slice(0, 100) ?? "general";
        state.cachedPatterns = await fetchHonchoPatterns(mergedOpts.milvusUrl, topic);
        state.lastPatternFetch = now;
      }

      const density = computeDensity(ctx.messages ?? []);
      const activeThreshold = mergedOpts.adaptiveThreshold
        ? adaptiveRelevanceThreshold(density, mergedOpts.relevanceThreshold)
        : mergedOpts.relevanceThreshold;

      const directives = [
        "## Kompress DCP Directives",
        `- adaptive threshold: ${activeThreshold.toFixed(2)} (density=${density.toFixed(2)})`,
        `- max context messages: ${mergedOpts.maxMessagesKept}`,
        `- prune low-similarity messages before context limit`,
        `- slice-aware boost: ${mergedOpts.sliceAwareBoost ? "on" : "off"} (+0.15 max)`,
        `- dropped-message digest: ${mergedOpts.droppedMessageDigest ? "on" : "off"}`,
      ];

      if (state.cachedPatterns.length > 0) {
        directives.push("", "## Relevant Honcho Patterns");
        state.cachedPatterns.forEach((p) => directives.push(`- ${p}`));
      }

      ctx.systemPrompt = (ctx.systemPrompt ?? "") + "\n\n" + directives.join("\n");
    },

    /** Trigger compaction before context limit hit */
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
      state.contextSizeHistory.push(messages.length);

      return { action: "compaction.trigger", reason: "context_limit_near" };
    },
  };
};