// LiteLLM pre-request middleware hook for kompress
// Intercepts /v1/chat/completions, scores → prunes → rewrites messages before model call
// Lightweight: no milvus calls, no embedding calls. Falls through unchanged on any error.

import { isProtected } from "./kompress-ultra";
import { compressMessage, CompressionLevel } from "./rewriter";

export interface LiteLLMRequest {
  model: string;
  messages: Array<{ role: string; content: string }>;
}

// ─── Lightweight scoring (no milvus) ────────────────────────────────────────

function ebbinghausDecay(age: number): number {
  const halfLife = 5;
  return Math.exp(-age / halfLife);
}

function structuralBoost(msg: { role: string; content: string }): number {
  let boost = 0.3;
  if (msg.role === "user") boost = 0.9;
  if (msg.content.includes("```")) boost = Math.max(boost, 0.8);
  if (msg.content.startsWith("Error:")) boost = Math.max(boost, 0.9);
  if (msg.role === "tool") boost = Math.max(boost, 0.6);
  return boost;
}

function scoreMessage(msg: { role: string; content: string }, index: number, total: number): number {
  const recency = ebbinghausDecay(total - index);
  const structural = structuralBoost(msg);
  // No milvus relevance — use 0.5 baseline
  const relevance = 0.5;
  return relevance * 0.4 + recency * 0.3 + structural * 0.3;
}

// ─── Middleware ──────────────────────────────────────────────────────────────

export function kompressMiddleware(): (
  req: LiteLLMRequest,
  next: (req: LiteLLMRequest) => Promise<any>,
) => Promise<any> {
  return async (req, next) => {
    const messages = req.messages;
    if (messages.length <= 5) return next(req);

    try {
      // ─── Score ────────────────────────────────────────────────────────
      const scored = messages.map((msg, idx) => ({
        msg,
        idx,
        score: scoreMessage(msg, idx, messages.length),
        protected: isProtected(msg, idx, messages.length),
      }));

      // ─── Prune ────────────────────────────────────────────────────────
      const kept = scored.filter(s => s.protected || s.score >= 0.5);

      // Safety: never drop more than 50%
      const maxPrune = Math.floor(messages.length * 0.5);
      const dropped = scored.length - kept.length;
      if (dropped > maxPrune) {
        const excess = dropped - maxPrune;
        const weakest = scored
          .filter(s => !s.protected && s.score < 0.5)
          .slice(0, excess);
        kept.push(...weakest);
      }

      // Sort by score desc, keep top 50
      kept.sort((a, b) => b.score - a.score);
      const trimmed = kept.slice(0, 50);

      if (trimmed.length === 0) return next(req);

      // ─── Rewrite ──────────────────────────────────────────────────────
      const rewritten = trimmed.map((s, i) => {
        const age = trimmed.length - i;
        if (age <= 5) return s.msg;
        if (age <= 15) return { ...s.msg, content: compressMessage(s.msg.content, CompressionLevel.Lite) };
        return { ...s.msg, content: compressMessage(s.msg.content, CompressionLevel.Ultra) };
      });

      return next({ ...req, messages: rewritten });
    } catch {
      // Fallback: pass through unchanged
      return next(req);
    }
  };
}
