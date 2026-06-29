import { describe, test, expect, afterEach } from "bun:test";
import { pickTopic, shouldReflect, DEFAULT_TOPICS } from "../src/topic";
import { DogfeedDB } from "../src/db";
import { mkdtempSync, rmSync } from "fs";
import { join } from "path";

let dbPath: string;
let conn: DogfeedDB;

afterEach(() => {
  conn?.close();
  if (dbPath) rmSync(dbPath, { recursive: true, force: true });
});

function setup(): DogfeedDB {
  dbPath = mkdtempSync(join(import.meta.dir, "../tmp-topic-"));
  const dbFile = join(dbPath, "test.db");
  conn = new DogfeedDB(dbFile);
  return conn;
}

describe("pickTopic", () => {
  test("returns from provided list when non-empty", () => {
    const db = setup();
    const topic = pickTopic(["ml", "os", "networking"], db, ["test"], "", "", 512);
    expect(["ml", "os", "networking"]).toContain(topic);
  });

  test("returns default topic when list empty", () => {
    const db = setup();
    const topic = pickTopic([], db, ["test"], "", "", 512);
    expect(DEFAULT_TOPICS).toContain(topic);
  });

  test("picks under-represented topic", () => {
    const db = setup();
    for (let i = 0; i < 10; i++) {
      db.insertRecord({
        topic: "ml", question: `Q${i}`, answer: `A${i}`, model: "test",
        tokens_in: 0, tokens_out: 0, hash: `dh-pick-${i}`, pushed: false,
      });
    }
    const topic = pickTopic(["ml", "os"], db, ["test"], "", "", 512);
    expect(topic).toBe("os");
  });
});

describe("shouldReflect", () => {
  test("returns false when ralphEvery is 0", () => {
    expect(shouldReflect(0, 0)).toBe(false);
    expect(shouldReflect(50, 0)).toBe(false);
  });

  test("returns false when totalRecords is 0", () => {
    expect(shouldReflect(0, 50)).toBe(false);
  });

  test("returns true at interval boundary", () => {
    expect(shouldReflect(50, 50)).toBe(true);
    expect(shouldReflect(100, 50)).toBe(true);
  });

  test("returns false between intervals", () => {
    expect(shouldReflect(49, 50)).toBe(false);
    expect(shouldReflect(51, 50)).toBe(false);
  });
});

describe("DEFAULT_TOPICS", () => {
  test("has at least 5 topics", () => {
    expect(DEFAULT_TOPICS.length).toBeGreaterThanOrEqual(5);
  });
  test("all are strings", () => {
    for (const t of DEFAULT_TOPICS) {
      expect(typeof t).toBe("string");
      expect(t.length).toBeGreaterThan(0);
    }
  });
});
