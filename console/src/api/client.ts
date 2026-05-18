import type {
  ListSandboxesResponse,
  Sandbox,
  UsageResponse,
  FleetResponse,
  ApiError,
} from "./types";

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

async function request<T>(url: string): Promise<T> {
  const res = await fetch(url, { headers });
  if (!res.ok) {
    const body = (await res.json()) as ApiError;
    // Message includes code so callers can match on it; detail follows.
    throw new BffError(
      body.error.code,
      `${body.error.code}: ${body.error.message}`,
      res.status,
    );
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

    getSandbox(id: string): Promise<Sandbox> {
      return request(`/api/sandboxes/${id}`);
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
  };
}

export type BffClient = ReturnType<typeof createBffClient>;
