import { DogfeedDB } from "./db.js";
import { ask } from "./llm.js";
import { scrubPII, contentHash, isQualityAnswer, isEnglish } from "./scrub.js";
import { pickTopic, runReflection, shouldReflect } from "./topic.js";
import { compressBatch } from "./compress.js";
import { publishBatch } from "./publish.js";
import { logStats, logEvent } from "./telemetry.js";
import type { DogfeedConfig, Record, LoopStats } from "./types.js";

const QUESTION_PROMPT = (topic: string) =>
  `Generate ONE interesting question about "${topic}" that would produce high-quality training data. Reply with ONLY the question, no preamble.`;

const ANSWER_PROMPT = (question: string) =>
  `Answer this question thoroughly and accurately in 2-5 paragraphs. Include technical details, examples, and practical insights:\n\n${question}`;

export interface IterationResult {
  record: Record | null;
  topic: string;
  skipped?: string;
}

export async function iteration(
  config: DogfeedConfig,
  conn: DogfeedDB,
): Promise<IterationResult> {
  if (conn.todayCalls() >= config.dailyCallLimit) {
    return { record: null, topic: "", skipped: "daily call limit" };
  }
  if (conn.todayTokens() >= config.dailyTokenLimit) {
    return { record: null, topic: "", skipped: "daily token limit" };
  }

  // Pass-by-reference so Ralph reflection can steer the next pickTopic.
  let ralphTopic: string | null = null;
  if (shouldReflect(conn.totalRecords(), config.ralphEvery)) {
    const newTopic = await runReflection(
      conn, config.models, config.openrouterKey ?? "", config.hfToken ?? "", config.maxTokens,
    );
    if (newTopic) {
      ralphTopic = newTopic;
      logEvent(conn, "INFO", `ralph: topic steered to "${newTopic}"`);
    }
  }

  const topic = ralphTopic ?? pickTopic(
    config.topics, conn, config.models,
    config.openrouterKey ?? "", config.hfToken ?? "", config.maxTokens,
  );

  const model = config.models[Math.floor(Math.random() * config.models.length)];

  const questionResp = await ask(
    QUESTION_PROMPT(topic), model,
    config.openrouterKey ?? "", config.hfToken ?? "", config.maxTokens,
  );
  if (questionResp) {
    conn.recordProviderCall(model, "question", questionResp.tokens_in, questionResp.tokens_out, true);
  } else {
    conn.recordProviderCall(model, "question", 0, 0, false);
    logEvent(conn, "WARN", `question generation failed for topic "${topic}"`);
    return { record: null, topic, skipped: "question generation failed" };
  }
  if (!questionResp.content) {
    logEvent(conn, "WARN", `question generation returned empty for topic "${topic}"`);
    return { record: null, topic, skipped: "question generation empty" };
  }

  const answerResp = await ask(
    ANSWER_PROMPT(questionResp.content), model,
    config.openrouterKey ?? "", config.hfToken ?? "", config.maxTokens,
  );
  if (answerResp) {
    conn.recordProviderCall(model, "answer", answerResp.tokens_in, answerResp.tokens_out, true);
  } else {
    conn.recordProviderCall(model, "answer", 0, 0, false);
    logEvent(conn, "WARN", `answer generation failed for question`);
    return { record: null, topic, skipped: "answer generation failed" };
  }
  if (!answerResp.content) {
    logEvent(conn, "WARN", `answer generation returned empty`);
    return { record: null, topic, skipped: "answer generation empty" };
  }

  const cleanedAnswer = scrubPII(answerResp.content);

  if (!isQualityAnswer(cleanedAnswer)) {
    return { record: null, topic, skipped: "quality gate" };
  }
  if (!isEnglish(cleanedAnswer)) {
    return { record: null, topic, skipped: "non-english" };
  }

  const hash = contentHash(questionResp.content + cleanedAnswer);
  if (conn.isDuplicate(hash)) {
    return { record: null, topic, skipped: "duplicate" };
  }

  let compressed: string | undefined;
  if (config.compress) {
    const results = await compressBatch([cleanedAnswer], config.compressionLevel);
    compressed = results[0];
  }

  const record: Omit<Record, "id" | "created_at"> = {
    topic,
    question: scrubPII(questionResp.content),
    answer: cleanedAnswer,
    model,
    tokens_in: questionResp.tokens_in + answerResp.tokens_in,
    tokens_out: questionResp.tokens_out + answerResp.tokens_out,
    compressed_answer: compressed,
    hash,
    pushed: false,
  };

  const id = conn.insertRecord(record);
  logEvent(conn, "INFO", `generated record #${id} topic="${topic}" model="${model}"`);

  return { record: { ...record, id, created_at: new Date().toISOString() }, topic };
}

export async function runLoop(config: DogfeedConfig): Promise<void> {
  const conn = new DogfeedDB(config.dbPath);
  let running = true;

  const shutdown = () => {
    running = false;
    logEvent(conn, "INFO", "loop shutting down");
  };

  process.on("SIGINT", shutdown);
  process.on("SIGTERM", shutdown);

  logEvent(conn, "INFO", `loop started interval=${config.intervalSec}s models=${config.models.join(",")}`);
  console.log(`[dogfeed] loop started interval=${config.intervalSec}s`);

  let iter = 0;
  while (running) {
    iter++;
    const result = await iteration(config, conn);

    if (result.record) {
      console.log(`[dogfeed] #${iter} topic="${result.topic}" model="${result.record.model}"`);
    } else if (result.skipped) {
      console.log(`[dogfeed] #${iter} skip: ${result.skipped}`);
    }

    if (
      config.hfRepo &&
      config.hfToken &&
      conn.unpushedRecords().length >= config.pushEvery
    ) {
      const pushed = await publishBatch(conn, config.hfRepo, config.hfToken, {
        webhookUrl: process.env.DOGFEED_WEBHOOK_URL,
      });
      console.log(`[dogfeed] pushed ${pushed} records to ${config.hfRepo}`);
    }

    if (iter % 10 === 0) {
      logStats(conn, config.statsPath);
    }

    await sleep(config.intervalSec * 1000);
  }

  logStats(conn, config.statsPath);
  conn.close();
  console.log("[dogfeed] loop stopped");
}

function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}
