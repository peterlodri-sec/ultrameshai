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
    expect(second.answer).toContain("Reinforcement learning");
    expect(second.compressed_answer).toBe("RL: trains agents via reward.");
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

describe("compressBatch E2E with kompress-ultra", () => {
  test("compresses answer and produces pruner role", async () => {
    const { compressBatch } = await import("../src/compress");
    const raw =
      "I would be happy to explain the CAP theorem. Basically, in a distributed system, you can really only pick two of consistency, availability, and partition tolerance.";
    const [compressed] = await compressBatch([raw], "lite");
    expect(compressed).toBeDefined();
    expect(compressed!.length).toBeLessThan(raw.length);
  });
});

describe("publishBatch & pushAll NDJSON + latest.jsonl mirror", () => {
  test("publishes timestamped batch, mirrors data/latest.jsonl and data/stats.json, and marks pushed", async () => {
    const { DogfeedDB } = await import("../src/db");
    const { publishBatch } = await import("../src/publish");
    const { mkdtempSync, rmSync } = await import("fs");
    const { join } = await import("path");

    const dir = mkdtempSync(join(import.meta.dir, "../tmp-pub-"));
    const db = new DogfeedDB(join(dir, "test.db"));
    db.insertRecord({
      topic: "distributed systems",
      question: "What is Raft?",
      answer: "Raft is a consensus algorithm with leader election and log replication.",
      compressed_answer: "Raft: consensus algorithm with leader election, log replication.",
      model: "local/qwen3.6-fast:latest",
      tokens_in: 12,
      tokens_out: 42,
      hash: "dh-raft-1",
      pushed: false,
    });

    const committedPaths: string[] = [];
    const origFetch = globalThis.fetch;
    globalThis.fetch = (async (url: string | URL | Request, init?: RequestInit) => {
      const u = String(url);
      if (u.includes("/raw/main/")) {
        return new Response("Not Found", { status: 404 });
      }
      if (u.includes("/commit/main")) {
        const body = String(init?.body ?? "");
        const lines = body.trim().split("\n");
        const opLine = JSON.parse(lines[1]);
        committedPaths.push(opLine.value.path);
        return new Response(
          JSON.stringify({ commitOid: "deadbeef1234567890abcdef1234567890abcdef" }),
          { status: 200, headers: { "Content-Type": "application/json" } },
        );
      }
      return new Response("OK", { status: 200 });
    }) as typeof fetch;

    try {
      const pushed = await publishBatch(db, "PeetPedro/ultrawhale-dogfood", "hf_testtoken12345", {
        batchSize: 50,
      });
      expect(pushed).toBe(1);
      expect(db.unpushedRecords().length).toBe(0);
      expect(committedPaths.some((p) => p.startsWith("data/loop-"))).toBe(true);
      expect(committedPaths).toContain("data/latest.jsonl");
      expect(committedPaths).toContain("data/stats.json");
    } finally {
      globalThis.fetch = origFetch;
      db.close();
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
