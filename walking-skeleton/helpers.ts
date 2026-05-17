export const CONTROLLER_URL = process.env.CONTROLLER_URL ?? "http://localhost:9090";
export const PROXY_URL = process.env.PROXY_URL ?? "http://localhost:8080";
export const TENANT = "walking-skeleton";

export interface CreateSandboxResponse {
  sandbox: { id: string; status: string };
  connection: { token: string; url: string; status_url: string };
}

export function rebaseUrl(remoteUrl: string): string {
  const parsed = new URL(remoteUrl);
  const base = new URL(CONTROLLER_URL);
  parsed.hostname = base.hostname;
  parsed.port = base.port;
  parsed.protocol = base.protocol;
  return parsed.toString();
}

export async function waitForReady(statusUrl: string, timeoutMs = 30000): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    const resp = await fetch(statusUrl).catch(() => null);
    if (resp?.ok) {
      const body = (await resp.json()) as { status: string };
      if (body.status === "ready") return;
    }
    await new Promise((r) => setTimeout(r, 500));
  }
  throw new Error(`Sandbox did not become ready within ${timeoutMs}ms`);
}

export async function createAndWait(): Promise<{ id: string; connection: CreateSandboxResponse["connection"] }> {
  const resp = await fetch(`${CONTROLLER_URL}/pub/v1/sandboxes`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-Auth-Tenant": TENANT,
    },
    body: JSON.stringify({
      runtime: "custom",
      image: "sandbox-runtime:node24-latest",
      resources: { vcpus: 1, memory_mb: 512 },
      ports: [8080],
    }),
  });

  if (!resp.ok) {
    throw new Error(`Create sandbox failed: ${resp.status} ${await resp.text()}`);
  }

  const { sandbox, connection } = (await resp.json()) as CreateSandboxResponse;
  await waitForReady(rebaseUrl(connection.status_url));
  return { id: sandbox.id, connection };
}

export async function terminate(id: string): Promise<void> {
  const resp = await fetch(`${CONTROLLER_URL}/pub/v1/sandboxes/${id}`, {
    method: "DELETE",
    headers: { "X-Auth-Tenant": TENANT },
  });
  if (!resp.ok && resp.status !== 404) {
    console.warn(`  Delete returned ${resp.status} (non-fatal)`);
  }
}

export async function postSandboxEndpoint(sandboxId: string, path: string, body: unknown): Promise<Response> {
  const resp = await fetch(`${CONTROLLER_URL}/pub/v1/sandboxes/${sandboxId}/${path}`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-Auth-Tenant": TENANT,
    },
    body: JSON.stringify(body),
  });
  if (!resp.ok) {
    throw new Error(`POST /${path} failed: ${resp.status} ${await resp.text()}`);
  }
  return resp;
}
