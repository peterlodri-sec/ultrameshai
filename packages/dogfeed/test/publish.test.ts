import { describe, test, expect } from "bun:test";
import { recordsToJSONL } from "../src/publish";
import type { Record } from "../src/types";

const SAMPLE_RECORDS: Record[] = [
  {
    id: 1,
    topic: "distributed systems",
    question: "What is CAP theorem?",
    answer: "The CAP theorem states that a distributed system can provide at most two of three guarantees.",
    model: "qwen/qwen-2.5-7b-instruct:free",
    tokens_in: 10,
    tokens_out: 50,
    hash: "dh-1",
    pushed: false,
    created_at: "2026-06-29T12:00:00Z",
  },
  {
    id: 2,
    topic: "ml",
    question: "What is RL?",
    answer: "Reinforcement learning trains agents via reward signals.",
    model: "qwen/qwen-2.5-7b-instruct:free",
    tokens_in: 8,
    tokens_out: 30,
    compressed_answer: "RL: trains agents via reward.",
    hash: "dh-2",
    pushed: false,
    created_at: "2026-06-29T12:01:00Z",
  },
];

describe("recordsToJSONL", () => {
  test("produces valid JSONL", () => {
    const jsonl = recordsToJSONL(SAMPLE_RECORDS);
    const lines = jsonl.split("\n");
    expect(lines.length).toBe(2);
    for (const line of lines) {
      const parsed = JSON.parse(line);
      expect(parsed.id).toMatch(/^dogfeed-/);
      expect(parsed.source).toBe("dogfeed-loop");
      expect(parsed.topic_category).toBeDefined();
    }
  });

  test("uses compressed_answer when available", () => {
    const jsonl = recordsToJSONL(SAMPLE_RECORDS);
    const lines = jsonl.split("\n");
    const second = JSON.parse(lines[1]);
    expect(second.answer).toBe("RL: trains agents via reward.");
    expect(second.role).toBe("pruner");
  });

  test("uses raw answer when no compression", () => {
    const jsonl = recordsToJSONL(SAMPLE_RECORDS);
    const lines = jsonl.split("\n");
    const first = JSON.parse(lines[0]);
    expect(first.answer).toContain("CAP theorem");
    expect(first.role).toBe("generator");
  });

  test("handles empty array", () => {
    expect(recordsToJSONL([])).toBe("");
  });

  test("topic_category normalizes spaces", () => {
    const jsonl = recordsToJSONL(SAMPLE_RECORDS);
    const first = JSON.parse(jsonl.split("\n")[0]);
    expect(first.topic_category).toBe("distributed-systems");
  });
});
