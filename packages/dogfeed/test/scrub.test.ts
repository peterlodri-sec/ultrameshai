import { describe, test, expect } from "bun:test";
import {
  scrubPII,
  normalizeForDedup,
  contentHash,
  isQualityAnswer,
  isEnglish,
} from "../src/scrub";

describe("scrubPII", () => {
  test("redacts emails", () => {
    expect(scrubPII("Contact me at alice@example.com")).toBe("Contact me at [REDACTED]");
  });
  test("redacts phone numbers", () => {
    expect(scrubPII("Call 555-123-4567")).toBe("Call [REDACTED]");
  });
  test("redacts IPs", () => {
    expect(scrubPII("Server at 192.168.1.1")).toBe("Server at [REDACTED]");
  });
  test("redacts API keys", () => {
    expect(scrubPII("Key: sk-abc123def456ghi789jkl0")).toBe("Key: [REDACTED]");
  });
  test("redacts HF tokens", () => {
    expect(scrubPII("Token hf_abcdef1234")).toBe("Token [REDACTED]");
  });
  test("redacts GitHub tokens", () => {
    expect(scrubPII("ghp_1234567890abcdef1234567890abcdef1234")).toBe("[REDACTED]");
  });
  test("passes clean text through", () => {
    const text = "Distributed systems are complex";
    expect(scrubPII(text)).toBe(text);
  });
});

describe("normalizeForDedup", () => {
  test("lowercases and trims", () => {
    expect(normalizeForDedup("  Hello World  ")).toBe("hello world");
  });
  test("collapses whitespace", () => {
    expect(normalizeForDedup("a  b   c")).toBe("a b c");
  });
});

describe("contentHash", () => {
  test("returns consistent hash", () => {
    const h1 = contentHash("test content");
    const h2 = contentHash("test content");
    expect(h1).toBe(h2);
  });
  test("returns different hash for different content", () => {
    const h1 = contentHash("content A");
    const h2 = contentHash("content B");
    expect(h1).not.toBe(h2);
  });
  test("normalizes before hashing", () => {
    const h1 = contentHash("Hello World");
    const h2 = contentHash("  hello  world  ");
    expect(h1).toBe(h2);
  });
  test("prefixed with dh-", () => {
    expect(contentHash("anything")).toMatch(/^dh-/);
  });
});

describe("isQualityAnswer", () => {
  test("rejects short answers", () => {
    expect(isQualityAnswer("Short")).toBe(false);
  });
  test("rejects empty", () => {
    expect(isQualityAnswer("")).toBe(false);
  });
  test("rejects I don't know", () => {
    expect(isQualityAnswer("I don't know the answer to that question at all")).toBe(false);
  });
  test("rejects I cannot", () => {
    expect(isQualityAnswer("I cannot provide information about this topic because of policy")).toBe(false);
  });
  test("accepts substantial answers", () => {
    const answer = "The CAP theorem states that a distributed system can provide at most two of three guarantees: consistency, availability, and partition tolerance.";
    expect(isQualityAnswer(answer)).toBe(true);
  });
  test("custom min length", () => {
    expect(isQualityAnswer("Short but ok", 5)).toBe(true);
  });
});

describe("isEnglish", () => {
  test("detects english", () => {
    expect(isEnglish("The quick brown fox jumps over the lazy dog")).toBe(true);
  });
  test("rejects non-ascii heavy", () => {
    expect(isEnglish("这是一段中文文本用于测试语言检测功能的准确性")).toBe(false);
  });
  test("handles short text", () => {
    expect(isEnglish("Hi")).toBe(true);
  });
});
