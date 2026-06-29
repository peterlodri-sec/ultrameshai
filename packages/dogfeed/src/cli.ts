#!/usr/bin/env bun
// dogfeed CLI — entrypoint for `bun run` and the NixOS systemd service.
// Loads config from env (DOGFEED_*), defaults, or ~/.config/dogfeed/config.json,
// then starts the indefinite loop. systemd sends SIGTERM for a clean shutdown.

import { runLoop } from "./loop.js";
import { DogfeedDB } from "./db.js";
import { logEvent } from "./telemetry.js";
import { DEFAULT_CONFIG, type DogfeedConfig } from "./types.js";

async function readConfigFile(path: string): Promise<Partial<DogfeedConfig>> {
  try {
    const text = await Bun.file(path).text();
    return JSON.parse(text);
  } catch {
    return {};
  }
}

async function loadConfig(): Promise<DogfeedConfig> {
  const fromFile = await readConfigFile(
    process.env.DOGFEED_CONFIG ?? `${process.env.HOME}/.config/dogfeed/config.json`,
  );

  const cfg: DogfeedConfig = {
    ...DEFAULT_CONFIG,
    ...fromFile,
    openrouterKey: process.env.OPENROUTER_KEY,
    hfToken: process.env.HF_TOKEN,
    hfRepo: process.env.DOGFEED_HF_REPO ?? fromFile.hfRepo ?? DEFAULT_CONFIG.hfRepo,
    dbPath: process.env.DOGFEED_DB ?? fromFile.dbPath ?? DEFAULT_CONFIG.dbPath,
    intervalSec: Number(process.env.DOGFEED_INTERVAL ?? fromFile.intervalSec ?? DEFAULT_CONFIG.intervalSec),
    models: (process.env.DOGFEED_MODELS ?? fromFile.models?.join(",") ?? DEFAULT_CONFIG.models.join(","))
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean),
    topics: (process.env.DOGFEED_TOPICS ?? fromFile.topics?.join(",") ?? "")
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean),
    dailyCallLimit: Number(process.env.DOGFEED_DAILY_CALLS ?? fromFile.dailyCallLimit ?? DEFAULT_CONFIG.dailyCallLimit),
    dailyTokenLimit: Number(process.env.DOGFEED_DAILY_TOKENS ?? fromFile.dailyTokenLimit ?? DEFAULT_CONFIG.dailyTokenLimit),
    pushEvery: Number(process.env.DOGFEED_PUSH_EVERY ?? fromFile.pushEvery ?? DEFAULT_CONFIG.pushEvery),
    ralphEvery: Number(process.env.DOGFEED_RALPH_EVERY ?? fromFile.ralphEvery ?? DEFAULT_CONFIG.ralphEvery),
    statsPath: process.env.DOGFEED_STATS_PATH ?? fromFile.statsPath ?? DEFAULT_CONFIG.statsPath,
    compress: (process.env.DOGFEED_COMPRESS ?? "").trim() === "1" || fromFile.compress === true,
    compressionLevel: (process.env.DOGFEED_COMPRESS_LEVEL as "lite" | "ultra") ?? fromFile.compressionLevel ?? DEFAULT_CONFIG.compressionLevel,
    telemetryEnabled: process.env.DOGFEED_TELEMETRY ? process.env.DOGFEED_TELEMETRY !== "0" : fromFile.telemetryEnabled ?? DEFAULT_CONFIG.telemetryEnabled,
  };
  return cfg;
}

if (import.meta.main) {
  const cfg = await loadConfig();
  // Open the DB once so the very first logEvent lands in the same file
  // the loop uses — keeps the schema migration + startup log in one place.
  const boot = new DogfeedDB(cfg.dbPath);
  logEvent(boot, "INFO", `boot: models=${cfg.models.join(",")} interval=${cfg.intervalSec}s hfRepo=${cfg.hfRepo || "(none)"}`);
  boot.close();
  await runLoop(cfg);
}
