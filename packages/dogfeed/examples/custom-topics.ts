import { DogfeedDB, DEFAULT_CONFIG, pickTopic, isQualityAnswer, contentHash, scrubPII, recordsToJSONL } from "../src/index";
import { mkdtempSync, rmSync } from "fs";
import { join } from "path";

const dbPath = mkdtempSync(join(import.meta.dir, "../tmp-custom-"));
const conn = new DogfeedDB(join(dbPath, "dogfeed.db"));

console.log("=== dogfeed custom-topics example ===\n");

const customTopics = [
  "Rust ownership model",
  "WebAssembly garbage collection",
  "CRDT conflict resolution",
  "eBPF kernel tracing",
  "Zig comptime metaprogramming",
];

const config = { ...DEFAULT_CONFIG, topics: customTopics };

// Generate Q&A for each custom topic
for (const topic of customTopics) {
  const question = `Explain ${topic} in practical terms with a real-world example.`;
  const answer = `The ${topic} is a powerful concept. In practice, it allows developers to write more efficient and reliable code by leveraging compile-time guarantees and runtime optimizations. For example, a web framework might use it to ensure memory safety without garbage collection overhead.`;

  const cleaned = scrubPII(answer);
  if (isQualityAnswer(cleaned)) {
    const hash = contentHash(question + cleaned);
    if (!conn.isDuplicate(hash)) {
      conn.insertRecord({
        topic, question, answer: cleaned, model: "custom-sim",
        tokens_in: 15, tokens_out: 45, hash, pushed: false,
      });
      console.log(`✓ ${topic}`);
    }
  }
}

console.log(`\nRecords: ${conn.totalRecords()}`);
console.log(`Topics: ${conn.topicsSeen().join(", ")}`);

// Show that topic weighting avoids over-representation
console.log("\n--- Topic distribution (weighted pick) ---");
const counts = new Map<string, number>();
for (let i = 0; i < 30; i++) {
  const t = pickTopic(customTopics, conn, config.models, "", "", config.maxTokens);
  counts.set(t, (counts.get(t) ?? 0) + 1);
}
for (const [t, c] of [...counts.entries()].sort((a, b) => b[1] - a[1])) {
  console.log(`  ${t}: ${c}`);
}

conn.close();
rmSync(dbPath, { recursive: true, force: true });

console.log("\n=== done ===");
