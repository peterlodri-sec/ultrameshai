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
    const { KompressUltra } = await import("kompress-ultra");
    const k = new KompressUltra();
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
