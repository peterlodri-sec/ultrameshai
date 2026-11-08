export type CompressionLevel = "lite" | "ultra";

export interface CompressResult {
  input: string;
  output: string;
  tokensSaved: number;
}

export async function compressBatch(
  texts: string[],
  level: CompressionLevel = "lite",
): Promise<(string | undefined)[]> {
  try {
    let mod: {
      compressMessage?: (content: string, level: number) => string;
      CompressionLevel?: { Lite: number; Ultra: number };
      KompressUltra?: new () => {
        compress: (opts: {
          text: string;
          level: CompressionLevel;
          preserveCodeBlocks: boolean;
        }) => Promise<{ compressed: string }>;
      };
    };
    try {
      mod = await import("kompress-ultra");
    } catch {
      mod = await import("../../kompress-ultra/src/index.js");
    }

    if (typeof mod.compressMessage === "function" && mod.CompressionLevel) {
      const targetLevel =
        level === "ultra" ? mod.CompressionLevel.Ultra : mod.CompressionLevel.Lite;
      return texts.map((text) => {
        try {
          return mod.compressMessage!(text, targetLevel);
        } catch {
          return undefined;
        }
      });
    }

    if (mod.KompressUltra) {
      const k = new mod.KompressUltra();
      const results: (string | undefined)[] = [];
      for (const text of texts) {
        try {
          const result = await k.compress({
            text,
            level,
            preserveCodeBlocks: true,
          });
          results.push(result.compressed);
        } catch {
          results.push(undefined);
        }
      }
      return results;
    }

    return texts.map(() => undefined);
  } catch {
    return texts.map(() => undefined);
  }
}

export async function compressText(
  text: string,
  level: CompressionLevel = "lite",
): Promise<string | undefined> {
  const results = await compressBatch([text], level);
  return results[0];
}
