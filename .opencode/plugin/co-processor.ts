import { compressMessage, CompressionLevel } from "./rewriter";

export interface CoProcessorOptions {
  model?: string;
  base_url?: string;
  timeout_ms?: number;
}

const COMPRESS_PROMPT =
  "Compress to caveman-ultra. [thing] [action] [reason]. Preserve code blocks, errors, API names, file paths.";
const SYNTHESIS_PROMPT =
  "Synthesize into 50-token brain state. Dense, technical, no fluff.";

export class CoProcessor {
  private model: string;
  private baseUrl: string;
  private timeoutMs: number;
  private available: boolean;

  constructor(opts: CoProcessorOptions = {}) {
    this.model = opts.model ?? "qwen2.5:1.5b";
    this.baseUrl = opts.base_url ?? "http://localhost:11434";
    this.timeoutMs = opts.timeout_ms ?? 15_000;
    this.available = false;
  }

  async init(): Promise<boolean> {
    try {
      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), Math.min(this.timeoutMs, 3000));
      const res = await fetch(`${this.baseUrl}/api/tags`, { signal: controller.signal });
      clearTimeout(timer);
      this.available = res.ok;
      return this.available;
    } catch {
      this.available = false;
      return false;
    }
  }

  async compress(content: string): Promise<string> {
    if (!this.available) return this.compressFallback(content);

    const start = Date.now();
    try {
      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), this.timeoutMs);
      const res = await fetch(`${this.baseUrl}/api/generate`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        signal: controller.signal,
        body: JSON.stringify({
          model: this.model,
          prompt: `${COMPRESS_PROMPT}\n\n${content}`,
          stream: false,
          options: { temperature: 0.1, num_predict: 512 },
        }),
      });
      clearTimeout(timer);
      const latency = Date.now() - start;

      if (latency > 150) {
        return this.compressFallback(content);
      }

      if (!res.ok) {
        this.available = false;
        return this.compressFallback(content);
      }

      const json = (await res.json()) as { response?: string };
      return json.response?.trim() ?? this.compressFallback(content);
    } catch {
      this.available = false;
      return this.compressFallback(content);
    }
  }

  async synthesize(
    findings: Array<{ content: string; score: number }>,
  ): Promise<string> {
    if (!this.available) return this.synthesizeFallback(findings);

    const top = findings
      .sort((a, b) => b.score - a.score)
      .slice(0, 10)
      .map((f) => `[score=${f.score.toFixed(2)}] ${f.content}`)
      .join("\n");

    try {
      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), this.timeoutMs);
      const res = await fetch(`${this.baseUrl}/api/generate`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        signal: controller.signal,
        body: JSON.stringify({
          model: this.model,
          prompt: `${SYNTHESIS_PROMPT}\n\n${top}`,
          stream: false,
          options: { temperature: 0.1, num_predict: 256 },
        }),
      });
      clearTimeout(timer);

      if (!res.ok) {
        this.available = false;
        return this.synthesizeFallback(findings);
      }

      const json = (await res.json()) as { response?: string };
      return json.response?.trim() ?? this.synthesizeFallback(findings);
    } catch {
      this.available = false;
      return this.synthesizeFallback(findings);
    }
  }

  async score_batch(
    messages: Array<{ role: string; content: string }>,
  ): Promise<number[]> {
    if (!this.available) return this.scoreFallback(messages);

    const batch = messages
      .slice(-20)
      .map((m, i) => `[${i}] ${m.role}: ${m.content.slice(0, 200)}`)
      .join("\n");

    try {
      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), this.timeoutMs);
      const res = await fetch(`${this.baseUrl}/api/generate`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        signal: controller.signal,
        body: JSON.stringify({
          model: this.model,
          prompt: `Score each message relevance 0.0-1.0. Output only comma-separated scores.\n\n${batch}`,
          stream: false,
          options: { temperature: 0.0, num_predict: 256 },
        }),
      });
      clearTimeout(timer);

      if (!res.ok) {
        this.available = false;
        return this.scoreFallback(messages);
      }

      const json = (await res.json()) as { response?: string };
      const raw = json.response?.trim() ?? "";
      const scores = raw
        .split(/[\s,]+/)
        .map((s) => parseFloat(s))
        .filter((n) => !isNaN(n) && n >= 0 && n <= 1);

      if (scores.length === 0) return this.scoreFallback(messages);
      return scores;
    } catch {
      this.available = false;
      return this.scoreFallback(messages);
    }
  }

  // ─── Fallbacks ────────────────────────────────────────────────────────────

  private compressFallback(content: string): string {
    return compressMessage(content, CompressionLevel.Ultra);
  }

  private synthesizeFallback(
    findings: Array<{ content: string; score: number }>,
  ): string {
    const top = findings
      .sort((a, b) => b.score - a.score)
      .slice(0, 5)
      .map((f) => compressMessage(f.content, CompressionLevel.Ultra))
      .join(" | ");
    return `brain-state: ${top}`;
  }

  private scoreFallback(
    messages: Array<{ role: string; content: string }>,
  ): number[] {
    return messages.map((m) => {
      let s = 0.3;
      if (m.role === "user") s = Math.max(s, 0.9);
      if (m.role === "tool") s = Math.max(s, 0.6);
      if (m.content.includes("```")) s = Math.max(s, 0.8);
      if (m.content.startsWith("Error:")) s = Math.max(s, 0.9);
      return s;
    });
  }
}
