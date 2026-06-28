var __require = /* @__PURE__ */ ((x) => typeof require !== "undefined" ? require : typeof Proxy !== "undefined" ? new Proxy(x, {
  get: (a, b) => (typeof require !== "undefined" ? require : a)[b]
}) : x)(function(x) {
  if (typeof require !== "undefined")
    return require.apply(this, arguments);
  throw Error('Dynamic require of "' + x + '" is not supported');
});

// plugin/rewriter.ts
function compressMessage(content, level) {
  if (level === 0 /* Verbatim */)
    return content;
  const codeBlocks = [];
  let result = content.replace(/```[\s\S]*?```/g, (match) => {
    codeBlocks.push(match);
    return `__CODE_BLOCK_${codeBlocks.length - 1}__`;
  });
  const errors = [];
  result = result.replace(/Error:[^\n]*/g, (match) => {
    errors.push(match);
    return `__ERROR_${errors.length - 1}__`;
  });
  if (level === 1 /* Lite */) {
    result = result.replace(/\b(the|a|an|this|that|these|those)\b/gi, " ");
    result = result.replace(/\b(just|really|basically|actually|simply)\b/gi, " ");
    result = result.replace(/\b(I would be happy to|Sure!|Great!|Excellent!)\b/gi, "");
    result = result.replace(/\s{2,}/g, " ").trim();
  } else if (level === 2 /* Ultra */) {
    const sentences = result.split(/[.!?]+/).filter((s) => s.trim());
    const compressed = sentences.map((s) => {
      s = s.trim();
      if (/\b(sure|great|excellent|happy|glad|welcome)\b/i.test(s))
        return "";
      s = s.replace(/\b(I (have|will|am|would|can|did|was|were)|it (is|was|has))\b/gi, "").trim();
      s = s.replace(/\b(the|a|an|this|that|these|those)\b/gi, "").trim();
      s = s.replace(/\b(just|really|basically|actually|simply|now|successfully|already|still|even)\b/gi, "").trim();
      s = s.replace(/\b(is|are|was|were|been|being|has|had|do|does|did|and|or|but|for|with|from|in|on|at|to|of)\b/gi, " ").trim();
      s = s.replace(/\b(it|me|my|we|our|they|them|their)\b/gi, " ").trim();
      s = s.replace(/\b(system|implementation|support|tests|things|stuff|work)\b/gi, " ").trim();
      return s.replace(/\s{2,}/g, " ").trim();
    }).filter(Boolean).join(". ");
    result = compressed;
  }
  result = result.replace(/__CODE_BLOCK_(\d+)__/g, (_, i) => codeBlocks[parseInt(i)]);
  result = result.replace(/__ERROR_(\d+)__/g, (_, i) => errors[parseInt(i)]);
  return result.replace(/\s{2,}/g, " ").trim();
}

// plugin/kompress-ultra.ts
var DEFAULT_OPTIONS = {
  relevanceThreshold: 0.65,
  maxMessagesKept: 35,
  milvusUrl: "http://localhost:19530",
  mempalaceDb: "mempalace.db",
  pollIntervalMs: 60000,
  adaptiveThreshold: true,
  droppedMessageDigest: true,
  sliceAwareBoost: true,
  displayPruneStatus: true,
  transparencyMode: true
};
var circuitBreaker = { failures: 0, open_until: 0 };
function isCircuitOpen() {
  return Date.now() < circuitBreaker.open_until;
}
function recordSuccess() {
  circuitBreaker.failures = 0;
}
function recordFailure() {
  circuitBreaker.failures++;
  if (circuitBreaker.failures >= 3) {
    circuitBreaker.open_until = Date.now() + 60000;
  }
}
var circulatorQueue = [];
var CIRCULATOR_CAP = 100;
var CIRCULATOR_BATCH = 10;
function classifyMessage(content) {
  const lower = content.toLowerCase();
  if (/\b(shall|should|must|need|implement|create|build|fix|update)\b/.test(lower))
    return "instruction";
  if (/\b(todo|task|step|goal|objective)\b/.test(lower))
    return "task";
  if (/\b(did|done|completed|failed|error|changed|updated)\b/.test(lower))
    return "event";
  return "fact";
}
function enqueueCirculator(entry) {
  if (circulatorQueue.length >= CIRCULATOR_CAP) {
    spillCirculatorOverflow([entry]);
    return;
  }
  circulatorQueue.push(entry);
  if (circulatorQueue.length >= CIRCULATOR_BATCH) {
    flushCirculatorAsync();
  }
}
async function flushCirculatorAsync() {
  if (circulatorQueue.length === 0)
    return;
  const entries = circulatorQueue.splice(0);
  try {
    const texts = entries.map((e) => e.residual).join(`
---
`);
    const embedding = await embedText(texts);
    if (!embedding) {
      spillCirculatorOverflow(entries);
      return;
    }
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
          tags: entries.map((e) => e.classification),
          created_at: Date.now(),
          embedding_model: "bge-m3"
        }
      }),
      signal: AbortSignal.timeout(1000)
    }).catch(() => spillCirculatorOverflow(entries));
  } catch {
    spillCirculatorOverflow(entries);
  }
}
function spillCirculatorOverflow(entries) {
  const path = `${process.env.HOME}/.cache/ultrameshai/overflow-circulator.jsonl`;
  try {
    const lines = entries.map((e) => JSON.stringify(e)).join(`
`) + `
`;
    Bun.write(path, lines, { append: true });
  } catch {}
}
function computeDensity(messages) {
  if (messages.length < 2)
    return 0;
  const windowStart = Math.floor(messages.length / 3);
  const recent = messages.slice(windowStart);
  return recent.length / (messages.length || 1);
}
function adaptiveThreshold(density, base) {
  const offset = 0.15 - density * 0.4;
  return Math.max(0.4, Math.min(0.8, base + offset));
}
async function embedText(text) {
  if (isCircuitOpen())
    return null;
  const endpoint = process.env.OVHCLOUD_EMBEDDING_URL ?? "https://oai.endpoints.kepler.ai.cloud.ovh.net/v1/chat/completions";
  const apiKey = process.env.OVHCLOUD_API_KEY ?? "";
  try {
    const res = await fetch(endpoint, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${apiKey}`,
        "Content-Type": "application/json"
      },
      body: JSON.stringify({ model: "bge-m3", input: text }),
      signal: AbortSignal.timeout(1000)
    });
    if (!res.ok)
      return null;
    const json = await res.json();
    return json.data?.[0]?.embedding ?? null;
  } catch {
    return null;
  }
}
async function writeDroppedDigest(dropped, milvusUrl) {
  if (dropped.length === 0)
    return;
  try {
    const texts = dropped.map((m) => m.content).join(`
---
`);
    const embedding = await embedText(texts);
    if (!embedding)
      return;
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
          embedding_model: "bge-m3"
        }
      }),
      signal: AbortSignal.timeout(1000)
    });
  } catch {}
}
function isProtected(msg, index, total) {
  if (index >= total - 5)
    return true;
  if (msg.role === "user")
    return true;
  if (msg.content?.includes("```"))
    return true;
  if (msg.type === "error" || msg.content?.startsWith("Error:"))
    return true;
  return false;
}
function ebbinghausDecay(age) {
  const halfLife = 5;
  return Math.exp(-age / halfLife);
}
function structuralBoost(msg) {
  let boost = 0.3;
  if (msg.role === "user")
    boost = 0.9;
  if (msg.content?.includes("```"))
    boost = Math.max(boost, 0.8);
  if (msg.type === "error" || msg.content?.startsWith("Error:"))
    boost = Math.max(boost, 0.9);
  if (msg.role === "tool")
    boost = Math.max(boost, 0.6);
  return boost;
}
async function scoreMessage(msg, index, total, taskGoal) {
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
async function scoreMessageMilvus(text, _milvusUrl, sliceId) {
  try {
    const embedding = await embedText(text);
    if (!embedding)
      return 0.5;
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
async function queryMilvusSimilarity(embedding, milvusUrl) {
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
            vector: embedding
          }),
          signal: AbortSignal.timeout(1000)
        });
        if (res.ok) {
          const json = await res.json();
          return json.results?.[0]?.distance ?? 0;
        }
      } catch {}
      return 0;
    });
    const results = await Promise.all(promises);
    return Math.max(...results);
  } catch {
    return 0;
  }
}
async function fetchHonchoPatterns(milvusUrl, topic) {
  try {
    const embedding = await embedText(topic);
    if (!embedding)
      return [];
    const res = await fetch(`${milvusUrl}/v1/query`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        collection_name: "learning_patterns",
        output_fields: ["summary", "confidence", "pattern_type"],
        topK: 5,
        vector: embedding,
        filter: "confidence >= 0.5"
      })
    });
    if (!res.ok)
      return [];
    const json = await res.json();
    return (json.results ?? []).map((r) => `[${r.pattern_type}] ${r.summary} (conf=${r.confidence?.toFixed(2)})`);
  } catch {
    return [];
  }
}
function estimateTokens(text) {
  const len = text.length;
  if (len === 0)
    return 1;
  return Math.max(1, Math.min(4096, Math.ceil(len / 4)));
}
var DEFAULT_BUDGETS = {
  coder: { agent_type: "coder", max_context_tokens: 1e5, compression_aggressiveness: 0.8, brain_injection_budget: 500 },
  researcher: { agent_type: "researcher", max_context_tokens: 128000, compression_aggressiveness: 0.4, brain_injection_budget: 1000 },
  reviewer: { agent_type: "reviewer", max_context_tokens: 64000, compression_aggressiveness: 0.6, brain_injection_budget: 500 },
  orchestrator: { agent_type: "orchestrator", max_context_tokens: 128000, compression_aggressiveness: 0.5, brain_injection_budget: 800 }
};
function escalateForBudget(messages, budget) {
  const currentTokens = messages.reduce((sum, m) => sum + estimateTokens(m.content), 0);
  if (currentTokens <= budget.max_context_tokens)
    return messages;
  let result = messages.map((msg, i) => {
    const age = messages.length - i;
    if (age <= 5)
      return msg;
    return { ...msg, content: compressMessage(msg.content, 2 /* Ultra */) };
  });
  if (result.reduce((sum, m) => sum + estimateTokens(m.content), 0) <= budget.max_context_tokens) {
    return result;
  }
  result = result.filter((msg, i) => {
    if (isProtected(msg, i, result.length))
      return true;
    return true;
  });
  return result;
}
async function readBrainState() {
  const path = `${process.env.HOME}/.cache/ultrameshai/brain-state.json`;
  try {
    const content = await Bun.file(path).text();
    return JSON.parse(content);
  } catch {
    return null;
  }
}
function buildBrainLine(brainState) {
  const icon = brainState.status === "Alive" ? "\uD83E\uDDE0" : brainState.status === "Stale" ? "\uD83D\uDCA4" : "❓";
  const age = brainState.last_data_at_ms === 0 ? "never" : `${Math.round((Date.now() - brainState.last_data_at_ms) / 1000)}s ago`;
  return `${icon} BRAIN ${brainState.status} | patterns:${brainState.patterns_total} findings:${brainState.findings_total} units:${brainState.units_processed} | last:${age}`;
}
function buildKompressDisplay(stats, transparencyMode = false) {
  const saved = stats.tokensPruned - stats.tokensKept;
  if (transparencyMode) {
    const lines2 = [
      `\uD83D\uDDDC️  kompress: context optimized`,
      `   • Removed ${stats.pruned} low-signal messages (below threshold ${stats.threshold.toFixed(2)})`,
      `   • Kept ${stats.kept} messages (last 5 + user/code/errors + high-relevance)`,
      `   • Saved ~${saved.toLocaleString()} tokens (${Math.round(saved / stats.tokensPruned * 100)}% reduction)`,
      `   • Pruned content sent to brain for future retrieval`
    ];
    if (stats.history.length > 0) {
      const avg = (stats.history.reduce((a, b) => a + b, 0) / stats.history.length).toFixed(0);
      lines2.push(`   • Context trend: ${avg} msg avg (stable)`);
    }
    lines2.push("─");
    return {
      role: "system",
      content: lines2.join(`
`),
      _kompress: true,
      _kompressPruneEvent: true
    };
  }
  const lines = [
    `── kompress ${stats.model} ──`,
    `  pruned  ${stats.pruned} msg  ${stats.tokensPruned.toLocaleString()} tok  (threshold=${stats.threshold.toFixed(2)}, density=${stats.density.toFixed(2)})`,
    `  kept    ${stats.kept} msg  ${stats.tokensKept.toLocaleString()} tok`,
    `  saved   ${saved > 0 ? "+" : ""}${saved.toLocaleString()} tok`,
    `  total   ${stats.total} msg → ${stats.kept} msg`
  ];
  if (stats.history.length > 0) {
    const avg = (stats.history.reduce((a, b) => a + b, 0) / stats.history.length).toFixed(1);
    lines.push(`  history avg ${avg} msg  (${stats.history.slice(-3).join(", ")})`);
  }
  lines.push("──");
  return {
    role: "system",
    content: lines.join(`
`),
    _kompress: true,
    _kompressPruneEvent: true
  };
}
async function writeCompactionStats(dbPath, prunedCount, contextSizeAfter) {
  try {
    const { execSync } = await import("child_process");
    execSync(`mempalace write -- pruned=${prunedCount} --context-size=${contextSizeAfter}`, { cwd: dbPath, stdio: "ignore" });
  } catch {}
}
var state = {
  lastCompactionAt: 0,
  contextSizeHistory: [],
  cachedPatterns: [],
  lastPatternFetch: 0,
  lastStatusBannerAt: 0,
  bannerInterval: null
};
var kompress_ultra_default = (_input, options) => {
  const mergedOpts = { ...DEFAULT_OPTIONS, ...options };
  const BANNER_INTERVAL_MS = 300000;
  if (state.bannerInterval)
    clearInterval(state.bannerInterval);
  state.lastStatusBannerAt = Date.now();
  state.bannerInterval = setInterval(async () => {
    const now = Date.now();
    if (now - state.lastStatusBannerAt < BANNER_INTERVAL_MS)
      return;
    state.lastStatusBannerAt = now;
    const brainState = await readBrainState();
    const brainLine = brainState ? buildBrainLine(brainState) : "❓ BRAIN UNKNOWN";
    const historyLen = state.contextSizeHistory.length;
    const avgSize = historyLen > 0 ? Math.round(state.contextSizeHistory.reduce((a, b) => a + b, 0) / historyLen) : 0;
    const lines = [
      "─── kompress status ───",
      brainLine,
      `patterns cached: ${state.cachedPatterns.length}`,
      `context history: ${historyLen} snapshots, avg ${avgSize} msg`,
      `circuit breaker: ${isCircuitOpen() ? "OPEN" : "closed"}`,
      "───────────────────────"
    ];
    console.log(lines.join(`
`));
  }, BANNER_INTERVAL_MS);
  return {
    "experimental.chat.messages.transform": async (input, _output) => {
      const ctx = input;
      const messages = ctx.messages ?? [];
      if (messages.length <= mergedOpts.maxMessagesKept)
        return;
      const density = computeDensity(messages);
      const threshold = mergedOpts.adaptiveThreshold ? adaptiveThreshold(density, mergedOpts.relevanceThreshold) : mergedOpts.relevanceThreshold;
      const scored = await Promise.all(messages.map(async (msg, idx) => ({
        msg,
        idx,
        score: await scoreMessage(msg, idx, messages.length, ctx.taskGoal)
      })));
      const kept = [];
      const dropped = [];
      for (const s of scored) {
        if (s.score.protected || s.score.total >= threshold)
          kept.push(s.msg);
        else
          dropped.push(s.msg);
      }
      const maxPrune = Math.floor(messages.length * 0.5);
      if (dropped.length > maxPrune) {
        const excess = dropped.length - maxPrune;
        kept.push(...dropped.slice(0, excess));
        dropped.splice(0, excess);
      }
      let final = kept;
      if (kept.length > mergedOpts.maxMessagesKept) {
        final = kept.sort((a, b) => {
          const sa = scored.find((s) => s.msg === a)?.score.total ?? 0;
          const sb = scored.find((s) => s.msg === b)?.score.total ?? 0;
          return sa - sb;
        }).slice(-mergedOpts.maxMessagesKept);
      }
      if (final.length === 0)
        return;
      const rewritten = final.map((msg, i) => {
        const age = final.length - i;
        if (age <= 5)
          return msg;
        if (age <= 15)
          return { ...msg, content: compressMessage(msg.content, 1 /* Lite */) };
        return { ...msg, content: compressMessage(msg.content, 2 /* Ultra */) };
      });
      const agentType = ctx.agent_type || "orchestrator";
      const budget = DEFAULT_BUDGETS[agentType] || DEFAULT_BUDGETS.orchestrator;
      const budgeted = escalateForBudget(rewritten, budget);
      const prunedCount = messages.length - budgeted.length;
      if (prunedCount > 0) {
        for (const msg of dropped) {
          const contentHash = crypto?.getRandomValues?.(new Uint8Array(8)) ? Array.from(crypto.getRandomValues(new Uint8Array(8))).map((b) => b.toString(16).padStart(2, "0")).join("").slice(0, 16) : `${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
          enqueueCirculator({
            session_id: ctx.session_id || "unknown",
            agent_type: agentType,
            message_role: msg.role,
            content_hash: contentHash,
            classification: classifyMessage(msg.content),
            residual: msg.content,
            timestamp_ms: Date.now()
          });
        }
      }
      if (prunedCount > 0) {
        state.contextSizeHistory.push(messages.length);
        if (state.contextSizeHistory.length > 10) {
          state.contextSizeHistory = state.contextSizeHistory.slice(-10);
        }
        const model = ctx.model || "unknown";
        const tokensPruned = dropped.reduce((sum, m) => sum + estimateTokens(m.content), 0);
        const tokensKept = budgeted.reduce((sum, m) => sum + estimateTokens(m.content), 0);
        const kompressStats = {
          model,
          pruned: prunedCount,
          kept: budgeted.length,
          total: messages.length,
          threshold,
          density,
          history: [...state.contextSizeHistory],
          tokensPruned,
          tokensKept
        };
        const kompressDisplay = buildKompressDisplay(kompressStats, mergedOpts.transparencyMode);
        const brainState = await readBrainState();
        if (brainState) {
          kompressDisplay.content += `
${buildBrainLine(brainState)}`;
        }
        ctx.messages = [kompressDisplay, ...budgeted];
        await writeCompactionStats(mergedOpts.mempalaceDb, prunedCount, budgeted.length);
        if (mergedOpts.droppedMessageDigest && dropped.length > 0) {
          await writeDroppedDigest(dropped, mergedOpts.milvusUrl);
        }
      }
    },
    "experimental.chat.system.transform": async (input, _output) => {
      const ctx = input;
      const now = Date.now();
      if (isCircuitOpen()) {
        const staleLine = "❓ BRAIN STALE (circuit breaker open)";
        ctx.systemPrompt = (ctx.systemPrompt ?? "") + `

` + staleLine;
        return;
      }
      if (now - state.lastPatternFetch > mergedOpts.pollIntervalMs) {
        const topic = ctx.taskGoal ?? ctx.messages?.[0]?.content?.slice(0, 100) ?? "general";
        try {
          state.cachedPatterns = await fetchHonchoPatterns(mergedOpts.milvusUrl, topic);
          recordSuccess();
        } catch {
          recordFailure();
          state.cachedPatterns = [];
        }
        state.lastPatternFetch = now;
      }
      const density = computeDensity(ctx.messages ?? []);
      const activeThreshold = mergedOpts.adaptiveThreshold ? adaptiveThreshold(density, mergedOpts.relevanceThreshold) : mergedOpts.relevanceThreshold;
      const brainState = await readBrainState();
      let brainLine = "";
      if (brainState) {
        brainLine = buildBrainLine(brainState);
      }
      const kompressBlock = [
        `## kompress auto-pruning | threshold ${activeThreshold.toFixed(2)} | density ${density.toFixed(2)} | max-kept ${mergedOpts.maxMessagesKept} msg`,
        brainLine
      ].filter(Boolean);
      if (state.cachedPatterns.length > 0) {
        kompressBlock.push("", "## honcho patterns");
        state.cachedPatterns.forEach((p) => kompressBlock.push(`- ${p}`));
      }
      ctx.systemPrompt = (ctx.systemPrompt ?? "") + `

` + kompressBlock.join(`
`);
    },
    "experimental.compaction.autocontinue": async (input, _output) => {
      const ctx = input;
      const messages = ctx.messages ?? [];
      const now = Date.now();
      if (now - state.lastCompactionAt < mergedOpts.pollIntervalMs)
        return;
      if (messages.length < mergedOpts.maxMessagesKept)
        return;
      state.lastCompactionAt = now;
      return { action: "compaction.trigger", reason: "context_limit_near" };
    }
  };
};
export {
  structuralBoost,
  scoreMessage,
  isProtected,
  ebbinghausDecay,
  kompress_ultra_default as default
};
