import { SandboxError } from "./errors.js";
import { normalizeBaseUrl, sleep } from "./utils.js";

export type HttpMethod = "GET" | "POST" | "PUT" | "DELETE";

export type RequestOptions = {
  method: HttpMethod;
  path: string;
  body?: unknown;
  headers?: Record<string, string>;
  query?: Record<string, string>;
  signal?: AbortSignal;
};

export type HttpClient = {
  request<T>(options: RequestOptions): Promise<T>;
  requestRaw(options: RequestOptions): Promise<Response>;
};

const RETRYABLE_STATUS_CODES = new Set([429, 500, 502, 503, 504]);
const MAX_RETRIES = 3;
const BASE_DELAY_MS = 1000;

export function createHttpClient(baseUrl: string, token?: string, tenant?: string): HttpClient {
  const normalizedBase = normalizeBaseUrl(baseUrl);

  async function doFetch(options: RequestOptions): Promise<Response> {
    const pathWithoutLeadingSlash = options.path.startsWith("/")
      ? options.path.slice(1)
      : options.path;
    const url = new URL(`${normalizedBase}/${pathWithoutLeadingSlash}`);
    if (options.query) {
      for (const [k, v] of Object.entries(options.query)) {
        url.searchParams.set(k, v);
      }
    }

    const headers: Record<string, string> = { ...options.headers };
    if (token) {
      headers["Authorization"] = `Bearer ${token}`;
    }
    if (tenant) {
      headers["X-Auth-Tenant"] = tenant;
    }
    if (options.body !== undefined) {
      headers["Content-Type"] = "application/json";
    }

    const init: RequestInit = {
      method: options.method,
      headers,
    };
    if (options.body !== undefined) {
      init.body = JSON.stringify(options.body);
    }
    if (options.signal) {
      init.signal = options.signal;
    }
    return fetch(url.toString(), init);
  }

  async function requestWithRetry(options: RequestOptions): Promise<Response> {
    let lastError: Error | undefined;

    for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
      try {
        const response = await doFetch(options);

        if (response.ok || !RETRYABLE_STATUS_CODES.has(response.status)) {
          return response;
        }

        await response.body?.cancel();

        if (attempt < MAX_RETRIES) {
          const delay = BASE_DELAY_MS * Math.pow(2, attempt);
          await sleep(delay);
          continue;
        }

        return response;
      } catch (err) {
        lastError = err instanceof Error ? err : new Error(String(err));
        if (attempt < MAX_RETRIES) {
          const delay = BASE_DELAY_MS * Math.pow(2, attempt);
          await sleep(delay);
          continue;
        }
      }
    }

    throw lastError ?? new SandboxError("request failed after retries");
  }

  return {
    async request<T>(options: RequestOptions): Promise<T> {
      const response = await requestWithRetry(options);
      if (!response.ok) {
        const text = await response.text().catch(() => "");
        throw new SandboxError(
          `HTTP ${response.status}: ${text}`,
          response.status,
        );
      }
      return (await response.json()) as T;
    },

    async requestRaw(options: RequestOptions): Promise<Response> {
      const response = await requestWithRetry(options);
      if (!response.ok) {
        const text = await response.text().catch(() => "");
        throw new SandboxError(
          `HTTP ${response.status}: ${text}`,
          response.status,
        );
      }
      return response;
    },
  };
}
