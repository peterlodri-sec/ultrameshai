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
        answer: r.compressed_answer ?? r.answer,
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
): Promise<number> {
  const records = conn.unpushedRecords();
  if (records.length === 0) return 0;

  const jsonl = recordsToJSONL(records);
  const filename = `loop-${new Date().toISOString().replace(/[:.]/g, "-").slice(0, 19)}.jsonl`;
  const path = `data/${filename}`;

  const existingContent = await readHFFile(hfRepo, path, hfToken);
  const content = existingContent ? existingContent + "\n" + jsonl : jsonl;

  const sha = existingContent ? undefined : await createHFFile(hfRepo, path, content, hfToken);
  if (sha) {
    const ids = records.map((r) => r.id!).filter(Boolean) as number[];
    conn.markPushed(ids);
    conn.logEvent("INFO", `pushed ${records.length} records to ${hfRepo}/${path}`);
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

async function createHFFile(
  repo: string,
  path: string,
  content: string,
  token: string,
): Promise<string | null> {
  const encoded = btoa(unescape(encodeURIComponent(content)));
  const res = await fetch(`${HF_API}/datasets/${repo}/commit/main`, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      operations: [
        {
          $addToEnd: {
            path,
            content: { $base64: encoded },
          },
        },
      ],
    }),
    signal: AbortSignal.timeout(30_000),
  });
  if (!res.ok) return null;
  const json = (await res.json()) as { commit?: { oid?: string } };
  return json.commit?.oid ?? null;
}

export async function pushAll(
  conn: DogfeedDB,
  hfRepo: string,
  hfToken: string,
  batchSize: number,
): Promise<number> {
  let total = 0;
  while (conn.unpushedRecords().length > 0) {
    const pushed = await publishBatch(conn, hfRepo, hfToken);
    if (pushed === 0) break;
    total += pushed;
    if (total >= batchSize) break;
  }
  return total;
}
