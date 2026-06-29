const PII_PATTERNS: RegExp[] = [
  /\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b/g,
  /\b(\+?1[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b/g,
  /\b(?:\d{1,3}\.){3}\d{1,3}\b/g,
  /sk-[A-Za-z0-9]{20,}/g,
  /hf_[A-Za-z0-9]{10,}/g,
  /ghp_[A-Za-z0-9]{36}/g,
  /xoxb-[A-Za-z0-9-]+/g,
];

export function scrubPII(text: string): string {
  let result = text;
  for (const pattern of PII_PATTERNS) {
    result = result.replace(pattern, "[REDACTED]");
  }
  return result;
}

export function normalizeForDedup(text: string): string {
  return text.trim().toLowerCase().replace(/\s+/g, " ");
}

export function contentHash(text: string): string {
  const normalized = normalizeForDedup(text);
  let h = 0;
  for (let i = 0; i < normalized.length; i++) {
    h = ((h << 5) - h + normalized.charCodeAt(i)) | 0;
  }
  return `dh-${Math.abs(h).toString(36)}`;
}

export function isQualityAnswer(answer: string, minLen = 50): boolean {
  if (answer.length < minLen) return false;
  if (/^\s*$/.test(answer)) return false;
  if (/^(I don't know|I'm not sure|I cannot|I can't)/.test(answer.trim())) return false;
  return true;
}

export function isEnglish(text: string): boolean {
  const sample = text.slice(0, 200);
  const asciiRatio = sample.split("").filter((c) => c.charCodeAt(0) < 128).length / sample.length;
  return asciiRatio > 0.8;
}
