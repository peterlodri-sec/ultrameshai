import { DogfeedDB } from "./db.js";
import type { Record } from "./types.js";

const HF_API = "https://huggingface.co/api";

export function recordsToJSONL(records: Record[]): string {
  return records
    .map((r) =>
      JSON.stringify({
        id: `dogfeed-${r.created_at?.replace(/[^0-9]/g, "").slice(0, 14) ?? Date.now()}-${r.id}`,
        topic: r.topic,
        question: r.question,
        answer: r.answer,
        compressed_answer: r.compressed_answer ?? null,
        model: r.model,
        tokens_in: r.tokens_in,
        tokens_out: r.tokens_out,
        role: r.compressed_answer ? "pruner" : "generator",
        source: "dogfeed-loop",
        topic_category: r.topic.toLowerCase().replace(/\s+/g, "-"),
        created_at: r.created_at ?? new Date().toISOString(),
      }),
    )
    .join("\n");
}

export async function publishBatch(
  conn: DogfeedDB,
  hfRepo: string,
  hfToken: string,
  opts: { webhookUrl?: string } = {},
): Promise<number> {
  const records = conn.unpushedRecords();
  if (records.length === 0) return 0;

  const jsonl = recordsToJSONL(records);
  const filename = `loop-${new Date().toISOString().replace(/[:.]/g, "-").slice(0, 19)}.jsonl`;
  const path = `data/${filename}`;

  const existingContent = await readHFFile(hfRepo, path, hfToken);
  const content = existingContent ? existingContent + "\n" + jsonl : jsonl;

  const writtenFiles: string[] = [];

  // Push the timestamped batch file
  const batchWrite = existingContent
    ? null
    : await writeHFFile(hfRepo, path, content, hfToken, "add");
  if (existingContent) {
    const overwrite = await writeHFFile(hfRepo, path, content, hfToken, "add");
    if (overwrite.oid) writtenFiles.push(path);
  } else if (batchWrite?.oid) {
    writtenFiles.push(path);
  }

  // Mirror the latest batch to data/latest.jsonl (always fresh)
  const latestWrite = await writeHFFile(
    hfRepo,
    "data/latest.jsonl",
    jsonl,
    hfToken,
    "add",
    "refresh latest batch mirror",
  );
  if (latestWrite.oid) writtenFiles.push("data/latest.jsonl");

  if (writtenFiles.length > 0) {
    // Mark local rows pushed FIRST, then snapshot stats — otherwise the
    // uploaded stats.json under-reports records_pushed by the current
    // batch. See review P2 on PR #3.
    const ids = records.map((r) => r.id!).filter(Boolean) as number[];
    conn.markPushed(ids);
    conn.logEvent("INFO", `pushed ${records.length} records to ${hfRepo}/${path}`);

    // Refresh stats.json AFTER markPushed so the HF mirror matches DB state
    const stats = conn.stats();
    const statsWrite = await writeHFFile(
      hfRepo,
      "data/stats.json",
      JSON.stringify(stats, null, 2),
      hfToken,
      "add",
      "refresh aggregate stats",
    );
    if (statsWrite.oid) writtenFiles.push("data/stats.json");

    // Fire-and-forget webhook to proposal.vaked.dev (if configured)
    await notifyProposal(opts.webhookUrl, {
      repo: hfRepo,
      files: writtenFiles,
      commitOid: batchWrite?.oid ?? latestWrite.oid ?? statsWrite.oid ?? "",
      recordCount: records.length,
      timestamp: new Date().toISOString(),
    });

    return records.length;
  }

  return 0;
}

async function readHFFile(
  repo: string,
  path: string,
  token: string,
): Promise<string | null> {
  try {
    const res = await fetch(`${HF_API}/datasets/${repo}/raw/main/${path}`, {
      headers: { Authorization: `Bearer ${token}` },
      signal: AbortSignal.timeout(15_000),
    });
    if (res.ok) return await res.text();
    return null;
  } catch {
    return null;
  }
}

/**
 * POST a commit to the HF dataset tree using the documented
 * `create_commit` API: header is required, operations are
 * `{key, value}` (or `{key, value, op?}`) where the supported ops
 * are `add` / `delete` / `copy`. We use `add` for both new and
 * overwrite (the API overwrites existing files with the same path).
 *
 * The legacy `$addToEnd` / `upsertFile` shape in earlier commits
 * returned non-OK and silently dropped records — see review P2 on
 * PR #3.
 */
async function writeHFFile(
  repo: string,
  path: string,
  content: string,
  token: string,
  op: "add" | "delete",
  summary = "dogfeed publish",
): Promise<{ oid: string | null }> {
  // HF create_commit expects NDJSON (application/x-ndjson): a `header` line,
  // then one line per file op (`file` for add, `deletedFile` for delete). The
  // old {summary, operations:[{key,value}]} JSON body was silently rejected —
  // records were fetched but nothing published ("pushed 0"). Verified fix.
  const header = JSON.stringify({ key: "header", value: { summary } });
  const line =
    op === "delete"
      ? JSON.stringify({ key: "deletedFile", value: { path } })
      : JSON.stringify({
          key: "file",
          value: {
            path,
            content: btoa(unescape(encodeURIComponent(content))),
            encoding: "base64",
          },
        });
  const res = await fetch(`${HF_API}/datasets/${repo}/commit/main`, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/x-ndjson",
    },
    body: header + "\n" + line + "\n",
    signal: AbortSignal.timeout(30_000),
  });
  if (!res.ok) {
    const text = await res.text();
    console.error(`[dogfeed] HF commit failed for ${path}: ${res.status} ${text.slice(0, 200)}`);
    return { oid: null };
  }
  const json = (await res.json()) as { commitOid?: string; commit?: { oid?: string } };
  return { oid: json.commitOid ?? json.commit?.oid ?? null };
}

/**
 * Fire-and-forget webhook to proposal.vaked.dev so the live-ticker
 * on the proposal site can re-render. Configured via the
 * `DOGFEED_WEBHOOK_URL` env var or the `dogfeed.webhookUrl` option in
 * the systemd module. Failure is non-fatal — the dataset is already
 * pushed; the ticker just won't refresh until the next push.
 */
async function notifyProposal(
  webhookUrl: string | undefined,
  payload: {
    repo: string;
    files: string[];
    commitOid: string;
    recordCount: number;
    timestamp: string;
  },
): Promise<void> {
  if (!webhookUrl) return;
  try {
    const res = await fetch(webhookUrl, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-Dogfeed-Source": "ultrameshai",
        "X-Dogfeed-Event": "batch.pushed",
      },
      body: JSON.stringify(payload),
      signal: AbortSignal.timeout(5_000),
    });
    if (!res.ok) {
      console.warn(`[dogfeed] proposal webhook returned ${res.status}`);
    }
  } catch (err) {
    console.warn(
      `[dogfeed] proposal webhook failed (non-fatal): ${err instanceof Error ? err.message : err}`,
    );
  }
}

export async function pushAll(
  conn: DogfeedDB,
  hfRepo: string,
  hfToken: string,
  batchSize: number,
  opts: { webhookUrl?: string } = {},
): Promise<number> {
  let total = 0;
  while (conn.unpushedRecords().length > 0) {
    const pushed = await publishBatch(conn, hfRepo, hfToken, opts);
    if (pushed === 0) break;
    total += pushed;
    if (total >= batchSize) break;
  }
  return total;
}
