type CacheEntry = { url: string; expiresAt: number };

const TTL_MS = 5 * 60 * 1000;
const MAX_ENTRIES = 1000;

const g = globalThis as unknown as {
  __connectionCache?: Map<string, CacheEntry>;
};
if (!g.__connectionCache) {
  g.__connectionCache = new Map();
}

const cache = g.__connectionCache;

export function setConnectionUrl(sandboxId: string, url: string): void {
  if (cache.size >= MAX_ENTRIES) {
    const oldest = cache.keys().next().value;
    if (oldest) cache.delete(oldest);
  }
  cache.set(sandboxId, { url, expiresAt: Date.now() + TTL_MS });
}

export function getConnectionUrl(sandboxId: string): string | undefined {
  const entry = cache.get(sandboxId);
  if (!entry) return undefined;
  if (Date.now() > entry.expiresAt) {
    cache.delete(sandboxId);
    return undefined;
  }
  return entry.url;
}
