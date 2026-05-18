const g = globalThis as unknown as {
  __connectionCache?: Map<string, string>;
};
if (!g.__connectionCache) {
  g.__connectionCache = new Map();
}

const cache = g.__connectionCache;

export function setConnectionUrl(sandboxId: string, url: string): void {
  cache.set(sandboxId, url);
}

export function getConnectionUrl(sandboxId: string): string | undefined {
  return cache.get(sandboxId);
}
