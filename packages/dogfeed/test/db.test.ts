import { describe, test, expect, afterEach } from "bun:test";
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
  dbPath = mkdtempSync(join(import.meta.dir, "../tmp-"));
  const dbFile = join(dbPath, "test.db");
  conn = new DogfeedDB(dbFile);
  return conn;
}

describe("DogfeedDB", () => {
  test("insert and retrieve record", () => {
    const db = setup();
    const id = db.insertRecord({
      topic: "ml",
      question: "What is RL?",
      answer: "Reinforcement learning is a type of machine learning.",
      model: "qwen/qwen-2.5-7b-instruct:free",
      tokens_in: 10,
      tokens_out: 20,
      hash: "dh-abc",
      pushed: false,
    });
    expect(id).toBeGreaterThan(0);
    expect(db.totalRecords()).toBe(1);
  });

  test("detects duplicates", () => {
    const db = setup();
    db.insertRecord({
      topic: "ml",
      question: "Q1",
      answer: "A1",
      model: "test",
      tokens_in: 0,
      tokens_out: 0,
      hash: "dh-dup",
      pushed: false,
    });
    expect(db.isDuplicate("dh-dup")).toBe(true);
    expect(db.isDuplicate("dh-other")).toBe(false);
  });

  test("unpushed and mark pushed", () => {
    const db = setup();
    const id1 = db.insertRecord({
      topic: "ml", question: "Q1", answer: "A1", model: "test",
      tokens_in: 0, tokens_out: 0, hash: "dh-1", pushed: false,
    });
    const id2 = db.insertRecord({
      topic: "ml", question: "Q2", answer: "A2", model: "test",
      tokens_in: 0, tokens_out: 0, hash: "dh-2", pushed: false,
    });
    expect(db.unpushedRecords().length).toBe(2);
    db.markPushed([id1]);
    expect(db.unpushedRecords().length).toBe(1);
  });

  test("recent records", () => {
    const db = setup();
    for (let i = 0; i < 5; i++) {
      db.insertRecord({
        topic: "ml", question: `Q${i}`, answer: `A${i}`, model: "test",
        tokens_in: 0, tokens_out: 0, hash: `dh-${i}`, pushed: false,
      });
    }
    expect(db.recentRecords(3).length).toBe(3);
    expect(db.recentRecords(3)[0].question).toBe("Q4");
  });

  test("topics seen", () => {
    const db = setup();
    db.insertRecord({ topic: "ml", question: "Q", answer: "A", model: "test", tokens_in: 0, tokens_out: 0, hash: "dh-1", pushed: false });
    db.insertRecord({ topic: "os", question: "Q", answer: "A", model: "test", tokens_in: 0, tokens_out: 0, hash: "dh-2", pushed: false });
    db.insertRecord({ topic: "ml", question: "Q", answer: "A", model: "test", tokens_in: 0, tokens_out: 0, hash: "dh-3", pushed: false });
    expect(db.topicsSeen()).toEqual(["ml", "os"]);
  });

  test("config get/set", () => {
    const db = setup();
    expect(db.configGet("key")).toBeNull();
    db.configSet("key", "value");
    expect(db.configGet("key")).toBe("value");
    db.configSet("key", "updated");
    expect(db.configGet("key")).toBe("updated");
  });

  test("log event", () => {
    const db = setup();
    db.logEvent("INFO", "test event");
    db.logEvent("ERROR", "test error");
    const stats = db.stats();
    expect(stats.errors).toBe(1);
  });

  test("today calls and tokens", () => {
    const db = setup();
    expect(db.todayCalls()).toBe(0);
    expect(db.todayTokens()).toBe(0);
    db.insertRecord({ topic: "ml", question: "Q", answer: "A", model: "test", tokens_in: 100, tokens_out: 200, hash: "dh-t", pushed: false });
    expect(db.todayCalls()).toBe(1);
    expect(db.todayTokens()).toBe(300);
  });

  test("stats", () => {
    const db = setup();
    db.insertRecord({ topic: "ml", question: "Q", answer: "A", model: "test", tokens_in: 10, tokens_out: 20, hash: "dh-s", pushed: false });
    const stats = db.stats();
    expect(stats.records_generated).toBe(1);
    expect(stats.records_pushed).toBe(0);
    expect(stats.tokens_used).toBe(30);
  });

  test("empty markPushed is no-op", () => {
    const db = setup();
    db.markPushed([]);
  });
});
