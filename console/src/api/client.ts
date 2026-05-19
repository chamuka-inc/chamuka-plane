import type { ListSandboxesResponse, Sandbox, ApiError } from "./types";

export type CreateSandboxRequest = {
  runtime?: string;
  image?: string;
  trust?: "trusted" | "untrusted" | "paranoid";
  resources?: { vcpus?: number; memory_mb?: number };
  timeout_ms?: number;
  env?: Record<string, string>;
  tags?: Record<string, string>;
  key?: string;
  max_idle_ms?: number;
  persistent?: boolean;
};

export type CreateSandboxResponse = {
  sandbox: Sandbox;
  connection: {
    token: string;
    url: string;
    secret_token?: string;
    status_url: string;
  };
};

export class BffError extends Error {
  constructor(
    public code: string,
    message: string,
    public status: number,
  ) {
    super(message);
    this.name = "BffError";
  }
}

const headers = { "Content-Type": "application/json" };

async function request<T>(url: string, init?: RequestInit): Promise<T> {
  const res = await fetch(url, { headers, ...init });
  if (!res.ok) {
    let code = "UNKNOWN";
    let msg = res.statusText;
    try {
      const body = (await res.json()) as ApiError;
      code = body.error.code;
      msg = body.error.message;
    } catch {
      // Non-JSON error response (proxy 502, etc.)
    }
    throw new BffError(code, `${code}: ${msg}`, res.status);
  }
  return res.json() as Promise<T>;
}

export function createBffClient() {
  return {
    listSandboxes(params?: {
      status?: string;
      limit?: number;
      cursor?: string;
    }): Promise<ListSandboxesResponse> {
      const sp = new URLSearchParams();
      sp.set("limit", String(params?.limit ?? 100));
      if (params?.status) sp.set("status", params.status);
      if (params?.cursor) sp.set("cursor", params.cursor);
      return request(`/api/sandboxes?${sp}`);
    },

    async getSandbox(id: string): Promise<Sandbox> {
      const result = await request<{ sandbox: Sandbox }>(`/api/sandboxes/${encodeURIComponent(id)}`);
      return result.sandbox;
    },

    createSandbox(body: CreateSandboxRequest): Promise<CreateSandboxResponse> {
      return request("/api/sandboxes", {
        method: "POST",
        body: JSON.stringify(body),
      });
    },

    async terminateSandbox(id: string, force?: boolean): Promise<void> {
      const qs = force ? "?force=true" : "";
      const res = await fetch(`/api/sandboxes/${encodeURIComponent(id)}${qs}`, {
        method: "DELETE",
        headers,
      });
      if (!res.ok && res.status !== 204) {
        let code = "UNKNOWN";
        let msg = res.statusText;
        try {
          const body = (await res.json()) as ApiError;
          code = body.error.code;
          msg = body.error.message;
        } catch {
          // Non-JSON error response
        }
        throw new BffError(code, `${code}: ${msg}`, res.status);
      }
    },
  };
}

export type BffClient = ReturnType<typeof createBffClient>;
