import { describe, test, expect, afterEach } from "bun:test";
import { DogfeedDB } from "../src/db";
import { mkdtempSync, rmSync } from "fs";
import { join } from "path";
import { logStats, formatStats } from "../src/telemetry";
import type { LoopStats } from "../src/types";

let dbPath: string;
let conn: DogfeedDB;

afterEach(() => {
  conn?.close();
  if (dbPath) rmSync(dbPath, { recursive: true, force: true });
});

function setup(): DogfeedDB {
  dbPath = mkdtempSync(join(import.meta.dir, "../tmp-telem-"));
  const dbFile = join(dbPath, "test.db");
  conn = new DogfeedDB(dbFile);
  return conn;
}

describe("logStats", () => {
  test("returns stats object", () => {
    const db = setup();
    const stats = logStats(db, join(dbPath, "stats.json"));
    expect(stats.records_generated).toBe(0);
    expect(stats.records_pushed).toBe(0);
  });
});

describe("formatStats", () => {
  test("formats readable output", () => {
    const stats: LoopStats = {
      records_generated: 100,
      records_pushed: 50,
      tokens_used: 12345,
      api_calls: 100,
      errors: 2,
      topics_seen: ["ml", "os", "networking"],
      models_used: ["qwen", "llama"],
      uptime_sec: 3661,
      started_at: "2026-06-29T12:00:00Z",
    };
    const output = formatStats(stats);
    expect(output).toContain("100 generated");
    expect(output).toContain("50 pushed");
    expect(output).toContain("12,345");
    expect(output).toContain("1h 1m");
    expect(output).toContain("ml, os, networking");
  });
});
