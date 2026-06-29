import { DogfeedDB, DEFAULT_CONFIG, isQualityAnswer, contentHash, recordsToJSONL, compressBatch } from "../src/index";
import { mkdtempSync, rmSync } from "fs";
import { join } from "path";

const dbPath = mkdtempSync(join(import.meta.dir, "../tmp-compress-"));
const conn = new DogfeedDB(join(dbPath, "dogfeed.db"));

console.log("=== dogfeed with-compression example ===\n");

const sampleQA = [
  {
    topic: "distributed systems",
    question: "How does Raft consensus work?",
    answer: "Raft is a consensus algorithm designed to be more understandable than Paxos. It works by electing a leader among a set of servers. The leader handles all client requests and replicates the log to followers. If the leader fails, a new election occurs. Raft uses random election timeouts to avoid split votes. Each server has a monotonically increasing term number. Log entries are committed once a majority of servers have replicated them.",
  },
  {
    topic: "machine learning",
    question: "What is gradient descent?",
    answer: "Gradient descent is an optimization algorithm used to minimize a function by iteratively moving in the direction of steepest descent. In machine learning, it is used to update model parameters to minimize the loss function. The learning rate determines the step size. Variants include batch, stochastic, and mini-batch gradient descent. Adam optimizer combines momentum and adaptive learning rates for faster convergence.",
  },
];

const config = { ...DEFAULT_CONFIG, compress: true, compressionLevel: "lite" };

// Store with optional compression
for (const qa of sampleQA) {
  const hash = contentHash(qa.question + qa.answer);
  if (!conn.isDuplicate(hash)) {
    let compressed: string | undefined;
    try {
      const results = await compressBatch([qa.answer], config.compressionLevel);
      compressed = results[0];
    } catch {
      console.log(`  (kompress-ultra not available, skipping compression)`);
    }

    conn.insertRecord({
      topic: qa.topic, question: qa.question, answer: qa.answer, model: "sim",
      tokens_in: 10, tokens_out: 50, compressed_answer: compressed, hash, pushed: false,
    });

    const ratio = compressed
      ? `${Math.round((1 - compressed.length / qa.answer.length) * 100)}% reduction`
      : "no compression";
    console.log(`✓ ${qa.topic} — ${ratio}`);
  }
}

console.log(`\nRecords: ${conn.totalRecords()}`);

// Show JSONL output
const records = conn.recentRecords(conn.totalRecords());
const jsonl = recordsToJSONL(records);
console.log(`\nJSONL (${jsonl.split("\n").length} records):`);
for (const line of jsonl.split("\n")) {
  const parsed = JSON.parse(line);
  console.log(`  ${parsed.topic}: role=${parsed.role} answer_len=${parsed.answer.length}`);
}

conn.close();
rmSync(dbPath, { recursive: true, force: true });

console.log("\n=== done ===");
