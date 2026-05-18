import type { Sandbox, BackendStatus } from "@/api/types";

const CONTROLLER_URL = process.env.CONTROLLER_URL ?? "http://localhost:8080";
const DEFAULT_CLUSTER = process.env.DEFAULT_CLUSTER ?? "localhost:9090";

type StoredSandbox = {
  sandbox: Sandbox;
  token: string;
  secret_token?: string;
  status_url: string;
  url: string;
};

const globalStore = globalThis as unknown as {
  __sandboxStore?: Map<string, StoredSandbox>;
};
if (!globalStore.__sandboxStore) {
  globalStore.__sandboxStore = new Map();
}
const store = globalStore.__sandboxStore;

type ConnectResponse = {
  backend_id: string;
  spawned: boolean;
  status: string;
  token: string;
  url: string;
  subdomain?: string;
  secret_token?: string;
  status_url: string;
  drone?: string;
};

type CreateRequest = {
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

function trustToPool(trust: string | undefined): string {
  if (trust === "trusted") return "trusted";
  return "untrusted";
}

export async function createSandbox(body: CreateRequest): Promise<{
  sandbox: Sandbox;
  connection: { token: string; url: string; secret_token?: string; status_url: string };
}> {
  const now = new Date().toISOString();
  const image = body.image ?? "sandbox-runtime:node24-latest";
  const trust = body.trust ?? "untrusted";
  const cluster = DEFAULT_CLUSTER;

  const connectBody = {
    key: {
      name: body.key ?? `sb-${Date.now()}`,
      namespace: "",
      tag: "",
    },
    spawn_config: {
      cluster,
      pool: trustToPool(trust),
      executable: {
        image,
        env: body.env ?? {},
      },
      lifetime_limit_seconds: body.timeout_ms
        ? Math.ceil(body.timeout_ms / 1000)
        : 3600,
      max_idle_seconds: body.max_idle_ms
        ? Math.ceil(body.max_idle_ms / 1000)
        : 300,
    },
  };

  const res = await fetch(`${CONTROLLER_URL}/ctrl/connect`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(connectBody),
  });

  if (!res.ok) {
    const text = await res.text().catch(() => "");
    throw new Error(`Controller ${res.status}: ${text}`);
  }

  const cr: ConnectResponse = await res.json();
  const expiresAt = new Date(
    Date.now() + (connectBody.spawn_config.lifetime_limit_seconds * 1000),
  ).toISOString();

  const sandbox: Sandbox = {
    id: cr.backend_id,
    backend_id: cr.backend_id,
    tenant: process.env.AUTH_TENANT ?? "tenant-default",
    status: cr.status as BackendStatus,
    runtime: body.runtime ?? "node24",
    trust,
    arch: "amd64",
    image,
    resources: {
      vcpus: body.resources?.vcpus ?? 1,
      memory_mb: body.resources?.memory_mb ?? 512,
    },
    ports: [],
    tags: body.tags ?? {},
    persistent: false,
    created_at: now,
    updated_at: now,
    expires_at: expiresAt,
    drone_name: cr.drone,
    cluster,
  };

  const entry: StoredSandbox = {
    sandbox,
    token: cr.token,
    secret_token: cr.secret_token,
    status_url: cr.status_url,
    url: cr.url,
  };
  store.set(cr.backend_id, entry);

  return {
    sandbox,
    connection: {
      token: cr.token,
      url: cr.url,
      secret_token: cr.secret_token,
      status_url: cr.status_url,
    },
  };
}

async function refreshStatus(entry: StoredSandbox): Promise<Sandbox> {
  try {
    const res = await fetch(
      `${CONTROLLER_URL}/pub/b/${encodeURIComponent(entry.sandbox.backend_id)}/status`,
    );
    if (res.ok) {
      const data = await res.json();
      if (data.status) {
        entry.sandbox.status = data.status as BackendStatus;
        entry.sandbox.updated_at = new Date(data.time ?? Date.now()).toISOString();
        if (data.status === "ready" && !entry.sandbox.ready_at) {
          entry.sandbox.ready_at = entry.sandbox.updated_at;
        }
        if (data.status === "terminated") {
          entry.sandbox.terminated_at = entry.sandbox.updated_at;
        }
      }
    }
  } catch {
    // Status endpoint unavailable — return stale data
  }
  return entry.sandbox;
}

export async function listSandboxes(opts?: {
  status?: string;
  limit?: number;
}): Promise<{ items: Sandbox[]; next_cursor?: string }> {
  const entries = Array.from(store.values());

  const sandboxes = await Promise.all(entries.map(refreshStatus));

  const filtered = opts?.status
    ? sandboxes.filter((s) => s.status === opts.status)
    : sandboxes;

  filtered.sort(
    (a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime(),
  );

  const limit = opts?.limit ?? 100;
  return { items: filtered.slice(0, limit) };
}

export async function getSandbox(
  id: string,
): Promise<{ sandbox: Sandbox; connection?: { token: string; url: string } } | null> {
  const entry = store.get(id);
  if (!entry) return null;

  const sandbox = await refreshStatus(entry);
  return {
    sandbox,
    connection: { token: entry.token, url: entry.url },
  };
}
