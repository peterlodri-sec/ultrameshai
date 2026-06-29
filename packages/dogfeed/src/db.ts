import { Database } from "bun:sqlite";
import type { Record, DogfeedEvent, LoopStats } from "./types.js";

export class DogfeedDB {
  private db: Database;

  constructor(path: string) {
    this.db = new Database(path);
    this.db.exec("PRAGMA journal_mode = WAL");
    this.db.exec("PRAGMA synchronous = NORMAL");
    this.migrate();
  }

  private migrate(): void {
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS records (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        topic TEXT NOT NULL,
        question TEXT NOT NULL,
        answer TEXT NOT NULL,
        model TEXT NOT NULL,
        tokens_in INTEGER DEFAULT 0,
        tokens_out INTEGER DEFAULT 0,
        compressed_answer TEXT,
        hash TEXT NOT NULL UNIQUE,
        pushed INTEGER DEFAULT 0,
        created_at TEXT DEFAULT (datetime('now'))
      );
      CREATE INDEX IF NOT EXISTS idx_records_topic ON records(topic);
      CREATE INDEX IF NOT EXISTS idx_records_pushed ON records(pushed);

      CREATE TABLE IF NOT EXISTS provider_calls (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        model TEXT NOT NULL,
        kind TEXT NOT NULL,
        tokens_in INTEGER DEFAULT 0,
        tokens_out INTEGER DEFAULT 0,
        ok INTEGER DEFAULT 1,
        created_at TEXT DEFAULT (datetime('now'))
      );
      CREATE INDEX IF NOT EXISTS idx_provider_calls_created ON provider_calls(created_at);

      CREATE TABLE IF NOT EXISTS config (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
      );

      CREATE TABLE IF NOT EXISTS events (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        level TEXT NOT NULL,
        message TEXT NOT NULL,
        created_at TEXT DEFAULT (datetime('now'))
      );
    `);
  }

  insertRecord(r: Omit<Record, "id" | "created_at">): number {
    const stmt = this.db.prepare(`
      INSERT INTO records (topic, question, answer, model, tokens_in, tokens_out, compressed_answer, hash, pushed)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0)
      ON CONFLICT(hash) DO NOTHING
    `);
    const result = stmt.run(r.topic, r.question, r.answer, r.model, r.tokens_in, r.tokens_out, r.compressed_answer ?? null, r.hash);
    return Number(result.lastInsertRowid);
  }

  isDuplicate(hash: string): boolean {
    const stmt = this.db.prepare("SELECT 1 FROM records WHERE hash = ? LIMIT 1");
    return stmt.get(hash) !== null;
  }

  recordProviderCall(model: string, kind: string, tokensIn: number, tokensOut: number, ok = true): void {
    this.db.prepare(
      "INSERT INTO provider_calls (model, kind, tokens_in, tokens_out, ok) VALUES (?, ?, ?, ?, ?)"
    ).run(model, kind, tokensIn, tokensOut, ok ? 1 : 0);
  }

  totalRecords(): number {
    const stmt = this.db.prepare("SELECT COUNT(*) as n FROM records");
    return (stmt.get() as { n: number }).n;
  }

  unpushedRecords(): Record[] {
    const stmt = this.db.prepare("SELECT * FROM records WHERE pushed = 0 ORDER BY id");
    return stmt.all() as Record[];
  }

  markPushed(ids: number[]): void {
    if (ids.length === 0) return;
    const placeholders = ids.map(() => "?").join(",");
    this.db.prepare(`UPDATE records SET pushed = 1 WHERE id IN (${placeholders})`).run(...ids);
  }

  recentRecords(n: number): Record[] {
    const stmt = this.db.prepare("SELECT * FROM records ORDER BY id DESC LIMIT ?");
    return stmt.all(n) as Record[];
  }

  topicsSeen(): string[] {
    const stmt = this.db.prepare("SELECT DISTINCT topic FROM records ORDER BY topic");
    return (stmt.all() as { topic: string }[]).map((r) => r.topic);
  }

  configGet(key: string): string | null {
    const stmt = this.db.prepare("SELECT value FROM config WHERE key = ?");
    const row = stmt.get(key) as { value: string } | undefined;
    return row?.value ?? null;
  }

  configSet(key: string, value: string): void {
    this.db.prepare("INSERT OR REPLACE INTO config (key, value) VALUES (?, ?)").run(key, value);
  }

  logEvent(level: DogfeedEvent["level"], message: string): void {
    this.db.prepare("INSERT INTO events (level, message) VALUES (?, ?)").run(level, message);
  }

  todayCalls(): number {
    // Count actual provider HTTP calls, not stored rows (each iteration
    // makes ≥1 call; failed calls also count so the daily quota is honored).
    const stmt = this.db.prepare(
      "SELECT COUNT(*) as n FROM provider_calls WHERE created_at >= date('now')"
    );
    return (stmt.get() as { n: number }).n;
  }

  todayTokens(): number {
    const stmt = this.db.prepare(
      "SELECT COALESCE(SUM(tokens_in + tokens_out), 0) as n FROM provider_calls WHERE created_at >= date('now')"
    );
    return (stmt.get() as { n: number }).n;
  }

  totalCalls(): number {
    return (this.db.prepare("SELECT COUNT(*) as n FROM provider_calls").get() as { n: number }).n;
  }

  stats(): LoopStats {
    const total = this.totalRecords();
    const pushed = (this.db.prepare("SELECT COUNT(*) as n FROM records WHERE pushed = 1").get() as { n: number }).n;
    const tokens = (this.db.prepare("SELECT COALESCE(SUM(tokens_in + tokens_out), 0) as n FROM records").get() as { n: number }).n;
    const errors = (this.db.prepare("SELECT COUNT(*) as n FROM events WHERE level = 'ERROR'").get() as { n: number }).n;
    const models = (this.db.prepare("SELECT DISTINCT model FROM provider_calls ORDER BY model").all() as { model: string }[]).map((r) => r.model);
    return {
      records_generated: total,
      records_pushed: pushed,
      tokens_used: tokens,
      api_calls: this.totalCalls(),
      errors,
      topics_seen: this.topicsSeen(),
      models_used: models,
      uptime_sec: 0,
      started_at: new Date().toISOString(),
    };
  }

  close(): void {
    this.db.close();
  }
}
