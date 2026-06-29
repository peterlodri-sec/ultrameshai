import { DogfeedDB, DEFAULT_CONFIG, pickTopic, shouldReflect, contentHash, isQualityAnswer, recordsToJSONL } from "../src/index";
import { mkdtempSync, rmSync } from "fs";
import { join } from "path";

const dbPath = mkdtempSync(join(import.meta.dir, "../tmp-example-"));
const conn = new DogfeedDB(join(dbPath, "dogfeed.db"));

console.log("=== dogfeed basic-loop example ===\n");

// Simulate a mini-loop
const topics = ["distributed systems", "machine learning", "networking"];
const config = { ...DEFAULT_CONFIG, topics };

for (let i = 0; i < 3; i++) {
  const topic = pickTopic(topics, conn, config.models, "", "", config.maxTokens);
  const question = `What is the most important concept in ${topic}?`;
  const answer = `${topic} is a vast field. The most important concept depends on context, but understanding fundamental principles is key to mastering the subject.`;

  if (isQualityAnswer(answer)) {
    const hash = contentHash(question + answer);
    if (!conn.isDuplicate(hash)) {
      conn.insertRecord({
        topic, question, answer, model: "simulated",
        tokens_in: 10, tokens_out: 30, hash, pushed: false,
      });
      console.log(`[${i + 1}] topic="${topic}" — stored`);
    }
  }
}

console.log(`\nTotal records: ${conn.totalRecords()}`);
console.log(`Topics seen: ${conn.topicsSeen().join(", ")}`);

const stats = conn.stats();
console.log(`\nStats:\n  Generated: ${stats.records_generated}\n  Tokens: ${stats.tokens_used}`);

// Export as JSONL
const records = conn.recentRecords(conn.totalRecords());
const jsonl = recordsToJSONL(records);
console.log(`\nJSONL output (${jsonl.split("\n").length} records):`);
console.log(jsonl.split("\n")[0].slice(0, 120) + "...");

conn.close();
rmSync(dbPath, { recursive: true, force: true });

console.log("\n=== done ===");
