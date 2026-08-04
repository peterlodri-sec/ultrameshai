// HF chat requests go to the OpenAI-compatible router endpoint
// (https://huggingface.co/docs/huggingface_hub/en/guides/inference#openai-compatibility).
// The per-model legacy `/models/<id>/v1/chat/completions` route is not a
// chat-completions path and returns non-OK — see review P2 on PR #3.
const HF_ROUTER_URL = "https://router.huggingface.co/v1/chat/completions";

// Ollama runs an OpenAI-compatible API at localhost:11434.
// https://github.com/ollama/ollama/blob/main/docs/api.md
const OLLAMA_BASE = process.env.OLLAMA_HOST ?? "http://localhost:11434";

export interface LLMResponse {
  content: string;
  model: string;
  tokens_in: number;
  tokens_out: number;
}

export async function askOpenRouter(
  prompt: string,
  model: string,
  key: string,
  maxTokens: number,
): Promise<LLMResponse | null> {
  try {
    const res = await fetch("https://openrouter.ai/api/v1/chat/completions", {
      method: "POST",
      headers: {
        Authorization: `Bearer ${key}`,
        "HTTP-Referer": "https://github.com/peterlodri-sec/ultrameshai",
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        model,
        messages: [{ role: "user", content: prompt }],
        max_tokens: maxTokens,
        temperature: 0.7,
      }),
      signal: AbortSignal.timeout(30_000),
    });
    if (!res.ok) {
      if (res.status === 429) await sleep(10_000);
      return null;
    }
    const json = (await res.json()) as {
      choices?: { message?: { content?: string } }[];
      usage?: { prompt_tokens?: number; completion_tokens?: number };
    };
    const content = json.choices?.[0]?.message?.content?.trim();
    if (!content) return null;
    return {
      content,
      model,
      tokens_in: json.usage?.prompt_tokens ?? 0,
      tokens_out: json.usage?.completion_tokens ?? 0,
    };
  } catch {
    return null;
  }
}

export async function askHF(
  prompt: string,
  modelId: string,
  token: string,
  maxTokens: number,
): Promise<LLMResponse | null> {
  try {
    const res = await fetch(HF_ROUTER_URL, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${token}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        model: modelId,
        messages: [{ role: "user", content: prompt }],
        max_tokens: maxTokens,
        temperature: 0.7,
      }),
      signal: AbortSignal.timeout(60_000),
    });
    if (!res.ok) {
      if (res.status === 429) await sleep(10_000);
      return null;
    }
    const json = (await res.json()) as {
      choices?: { message?: { content?: string } }[];
      usage?: { prompt_tokens?: number; completion_tokens?: number };
    };
    const content = json.choices?.[0]?.message?.content?.trim();
    if (!content) return null;
    return {
      content,
      model: modelId,
      tokens_in: json.usage?.prompt_tokens ?? 0,
      tokens_out: json.usage?.completion_tokens ?? 0,
    };
  } catch {
    return null;
  }
}

export async function askLocal(
  prompt: string,
  model: string,
  maxTokens: number,
): Promise<LLMResponse | null> {
  try {
    const res = await fetch(`${OLLAMA_BASE}/api/chat`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        model,
        messages: [{ role: "user", content: prompt }],
        stream: false,
        options: {
          num_predict: maxTokens,
          temperature: 0.7,
        },
      }),
      signal: AbortSignal.timeout(120_000),
    });
    if (!res.ok) {
      if (res.status === 429) await sleep(10_000);
      return null;
    }
    const json = (await res.json()) as {
      message?: { content?: string };
      eval_count?: number;
      prompt_eval_count?: number;
    };
    const content = json.message?.content?.trim();
    if (!content) return null;
    return {
      content,
      model,
      tokens_in: json.prompt_eval_count ?? 0,
      tokens_out: json.eval_count ?? 0,
    };
  } catch {
    return null;
  }
}

export async function ask(
  prompt: string,
  model: string,
  key: string,
  hfToken: string,
  maxTokens: number,
): Promise<LLMResponse | null> {
  if (model.startsWith("local/")) {
    return askLocal(prompt, model.slice(6), maxTokens);
  }
  if (model.startsWith("hf/")) {
    return askHF(prompt, model.slice(3), hfToken, maxTokens);
  }
  return askOpenRouter(prompt, model, key, maxTokens);
}

function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}
