import { DogfeedDB } from "./db.js";
import type { DogfeedEvent, LoopStats } from "./types.js";

export function logEvent(
  conn: DogfeedDB,
  level: DogfeedEvent["level"],
  message: string,
): void {
  conn.logEvent(level, message);
  const prefix = level === "ERROR" ? "!!" : level === "WARN" ? "! " : "  ";
  console.log(`[dogfeed] ${prefix}${message}`);
}

export function logStats(conn: DogfeedDB, statsPath: string): LoopStats {
  const stats = conn.stats();
  const output = {
    ...stats,
    exported_at: new Date().toISOString(),
  };
  try {
    Bun.write(statsPath, JSON.stringify(output, null, 2));
  } catch {
    // stats export is best-effort
  }
  return stats;
}

export function formatStats(stats: LoopStats): string {
  const uptimeH = Math.floor(stats.uptime_sec / 3600);
  const uptimeM = Math.floor((stats.uptime_sec % 3600) / 60);
  return [
    `Records: ${stats.records_generated} generated, ${stats.records_pushed} pushed`,
    `Tokens: ${stats.tokens_used.toLocaleString()}`,
    `API calls: ${stats.api_calls}`,
    `Errors: ${stats.errors}`,
    `Topics: ${stats.topics_seen.length} (${stats.topics_seen.slice(0, 5).join(", ")})`,
    `Models: ${stats.models_used.length} (${stats.models_used.slice(0, 3).join(", ")})`,
    `Uptime: ${uptimeH}h ${uptimeM}m`,
    `Started: ${stats.started_at}`,
  ].join("\n");
}
