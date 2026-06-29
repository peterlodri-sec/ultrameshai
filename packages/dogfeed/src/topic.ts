import type { DogfeedDB } from "./db.js";
import { ask, type LLMResponse } from "./llm.js";

const RALPH_SYSTEM = `You are Ralph, a topic-reflection engine for a data generation loop. Given recent questions the loop has generated, suggest ONE under-covered topic that would produce high-quality training data. Reply with ONLY the topic name (2-4 words), no explanation.`;

const DEFAULT_TOPICS = [
  "distributed systems",
  "machine learning fundamentals",
  "networking and protocols",
  "database internals",
  "operating systems",
  "programming language theory",
  "cryptographic primitives",
  "web architecture",
  "container orchestration",
  "concurrent programming",
];

export function pickTopic(
  topics: string[],
  conn: DogfeedDB,
  models: string[],
  openrouterKey: string,
  hfToken: string,
  maxTokens: number,
): string {
  if (topics.length > 0) {
    return weightedPick(topics, conn);
  }
  return DEFAULT_TOPICS[Math.floor(Math.random() * DEFAULT_TOPICS.length)];
}

function weightedPick(topics: string[], conn: DogfeedDB): string {
  const seen = new Map<string, number>();
  for (const r of conn.recentRecords(100)) {
    seen.set(r.topic, (seen.get(r.topic) ?? 0) + 1);
  }
  let best = topics[0];
  let bestScore = Infinity;
  for (const t of topics) {
    const count = seen.get(t) ?? 0;
    const score = count + Math.random() * 0.5;
    if (score < bestScore) {
      bestScore = score;
      best = t;
    }
  }
  return best;
}

export async function runReflection(
  conn: DogfeedDB,
  models: string[],
  openrouterKey: string,
  hfToken: string,
  maxTokens: number,
): Promise<string | null> {
  const recent = conn.recentRecords(10);
  if (recent.length < 5) return null;

  const questionList = recent.map((r) => `- ${r.topic}: ${r.question.slice(0, 80)}`).join("\n");
  const prompt = `Recent questions in the loop:\n${questionList}\n\nWhat under-covered topic should the loop explore next?`;

  for (const model of models) {
    const resp = await ask(prompt, model, openrouterKey, hfToken, maxTokens);
    if (resp?.content) {
      const topic = resp.content.replace(/["']/g, "").trim();
      if (topic.length > 0 && topic.length < 100) {
        return topic;
      }
    }
  }
  return null;
}

export function shouldReflect(totalRecords: number, ralphEvery: number): boolean {
  if (ralphEvery <= 0) return false;
  return totalRecords > 0 && totalRecords % ralphEvery === 0;
}

export { DEFAULT_TOPICS };
