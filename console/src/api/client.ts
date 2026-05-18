import type {
  ListSandboxesResponse,
  Sandbox,
  UsageResponse,
  FleetResponse,
  ApiError,
} from "./types";

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

    getSandboxWithConnection(id: string): Promise<{ sandbox: Sandbox; connection?: { token: string; url: string } }> {
      return request(`/api/sandboxes/${encodeURIComponent(id)}`);
    },

    getFleetState(): Promise<FleetResponse> {
      return request("/api/fleet");
    },

    getUsage(params?: { period?: string; sandbox_id?: string }): Promise<UsageResponse> {
      const sp = new URLSearchParams();
      if (params?.period) sp.set("period", params.period);
      if (params?.sandbox_id) sp.set("sandbox_id", params.sandbox_id);
      return request(`/api/usage?${sp}`);
    },

    createSandbox(body: CreateSandboxRequest): Promise<CreateSandboxResponse> {
      return request("/api/sandboxes", {
        method: "POST",
        body: JSON.stringify(body),
      });
    },
  };
}

export type BffClient = ReturnType<typeof createBffClient>;
