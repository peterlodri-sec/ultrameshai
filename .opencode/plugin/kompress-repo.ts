// Repo-anchored memory: .kompress/ shadow directory for file sidecars
import { readFileSync, writeFileSync, existsSync, mkdirSync, readdirSync } from "fs";
import { join, dirname } from "path";
import { execSync } from "child_process";

const KOMPRESS_DIR = ".kompress";
const FILES_DIR = join(KOMPRESS_DIR, "files");

export interface FileSidecar {
  file_path: string;
  last_mutated_commit: string;
  architectural_intent: string;
  known_quirks: string[];
  dependencies: string[];
  triples: Array<{ s: string; p: string; o: string }>;
}

function ensureDirs(): void {
  if (!existsSync(KOMPRESS_DIR)) mkdirSync(KOMPRESS_DIR, { recursive: true });
  if (!existsSync(FILES_DIR)) mkdirSync(FILES_DIR, { recursive: true });
}

function sidecarPath(filePath: string): string {
  return join(FILES_DIR, `${filePath}.json`);
}

export async function readFileSidecar(filePath: string): Promise<FileSidecar | null> {
  ensureDirs();
  const path = sidecarPath(filePath);
  if (!existsSync(path)) return null;
  return JSON.parse(readFileSync(path, "utf-8")) as FileSidecar;
}

export async function writeFileSidecar(filePath: string, sidecar: FileSidecar): Promise<void> {
  ensureDirs();
  const path = sidecarPath(filePath);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, JSON.stringify(sidecar, null, 2));
}

export async function isSidecarStale(filePath: string): Promise<boolean> {
  const sidecar = await readFileSidecar(filePath);
  if (!sidecar) return true;
  try {
    const currentHash = execSync(`git log -1 --format=%H ${filePath}`, {
      stdio: ["pipe", "pipe", "pipe"],
    }).toString().trim();
    return sidecar.last_mutated_commit !== currentHash;
  } catch {
    return true;
  }
}

export async function invalidateStaleSidecars(): Promise<string[]> {
  ensureDirs();
  const stale: string[] = [];
  try {
    const entries = readdirSync(FILES_DIR, { recursive: true });
    for (const entry of entries) {
      if (!entry.endsWith(".json")) continue;
      const rel = entry.replace(/\.json$/, "");
      if (await isSidecarStale(rel)) stale.push(rel);
    }
  } catch {
    // skip
  }
  return stale;
}
