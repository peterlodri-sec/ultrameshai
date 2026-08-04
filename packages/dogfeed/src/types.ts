export type LLMProvider = "openrouter" | "hf" | "local";

export interface DogfeedConfig {
  openrouterKey?: string;
  hfToken?: string;
  models: string[];
  maxTokens: number;
  intervalSec: number;
  topics: string[];
  ralphEvery: number;
  dailyCallLimit: number;
  dailyTokenLimit: number;
  dbPath: string;
  hfRepo: string;
  pushEvery: number;
  compress: boolean;
  compressionLevel: "lite" | "ultra";
  statsPath: string;
  telemetryEnabled: boolean;
}

export const DEFAULT_CONFIG: DogfeedConfig = {
  models: ["local/qwen2.5:3b", "openai/gpt-oss-20b:free", "nvidia/nemotron-3-nano-30b-a3b:free", "google/gemma-4-26b-a4b-it:free"],
  maxTokens: 1024,
  intervalSec: 30,
  topics: [],
  ralphEvery: 50,
  dailyCallLimit: 0,
  dailyTokenLimit: 0,
  dbPath: "./dogfeed.db",
  hfRepo: "",
  pushEvery: 50,
  compress: false,
  compressionLevel: "lite",
  statsPath: "./stats.json",
  telemetryEnabled: true,
};

export interface Record {
  id?: number;
  topic: string;
  question: string;
  answer: string;
  model: string;
  tokens_in: number;
  tokens_out: number;
  compressed_answer?: string;
  hash: string;
  pushed: boolean;
  created_at: string;
}

export interface DogfeedEvent {
  level: "INFO" | "WARN" | "ERROR";
  message: string;
}

export interface LoopStats {
  records_generated: number;
  records_pushed: number;
  tokens_used: number;
  api_calls: number;
  errors: number;
  topics_seen: string[];
  models_used: string[];
  uptime_sec: number;
  started_at: string;
}

export interface LLMPayload {
  model: string;
  messages: { role: string; content: string }[];
  max_tokens: number;
  temperature: number;
}
