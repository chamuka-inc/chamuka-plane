/**
 * Walking Skeleton — Variant Tests
 *
 * Exercises sandbox lifecycle extensions:
 * 1. Create a sandbox, wait for ready
 * 2. Apply a network policy (POST /sandboxes/:id/network-policy)
 * 3. Create a snapshot (POST /sandboxes/:id/snapshot)
 * 4. Terminate the sandbox
 *
 * Requires: docker compose up -d (walking-skeleton/docker-compose.yml)
 */

const CONTROLLER_URL = process.env.CONTROLLER_URL ?? "http://localhost:9090";
const TENANT = "walking-skeleton";

interface CreateSandboxResponse {
  sandbox: { id: string; status: string };
  connection: { token: string; url: string; status_url: string };
}

interface SnapshotResponse {
  snapshot: {
    id: string;
    sandbox_id: string;
    tenant: string;
    image_ref: string;
    base_image: string;
    arch: string;
    runtime: string;
    name: string | null;
    tags: Record<string, string>;
    created_at: string;
    expires_at: string;
  };
}

interface NetworkPolicy {
  mode: "allow-all" | "deny-all" | "rules";
  rules?: Array<{
    domains?: string[];
    cidrs?: string[];
    ports?: number[];
    action?: "allow" | "deny";
  }>;
}

async function waitForReady(statusUrl: string, timeoutMs = 30000): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const resp = await fetch(statusUrl);
      if (resp.ok) {
        const body = (await resp.json()) as { status: string };
        if (body.status === "ready") return;
      }
    } catch {
    }
    await new Promise((r) => setTimeout(r, 500));
  }
  throw new Error(`Sandbox did not become ready within ${timeoutMs}ms`);
}

function rebaseUrl(remoteUrl: string): string {
  const parsed = new URL(remoteUrl);
  const base = new URL(CONTROLLER_URL);
  parsed.hostname = base.hostname;
  parsed.port = base.port;
  parsed.protocol = base.protocol;
  return parsed.toString();
}

async function createAndWait(): Promise<{ id: string }> {
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
    throw new Error(`Create failed: ${resp.status} ${await resp.text()}`);
  }

  const { sandbox, connection } = (await resp.json()) as CreateSandboxResponse;
  console.log(`    Sandbox created: ${sandbox.id}`);

  await waitForReady(rebaseUrl(connection.status_url));
  console.log(`    Sandbox ready`);

  return { id: sandbox.id };
}

async function terminate(id: string): Promise<void> {
  const resp = await fetch(`${CONTROLLER_URL}/pub/v1/sandboxes/${id}`, {
    method: "DELETE",
    headers: { "X-Auth-Tenant": TENANT },
  });
  if (!resp.ok && resp.status !== 404) {
    console.warn(`    Delete returned ${resp.status} (non-fatal)`);
  }
}

async function testNetworkPolicy(): Promise<void> {
  console.log("==> Test: network policy update");

  const { id } = await createAndWait();

  const policy: NetworkPolicy = {
    mode: "rules",
    rules: [
      { domains: ["*.example.com"], action: "allow" },
      { cidrs: ["10.0.0.0/8"], ports: [443], action: "deny" },
    ],
  };

  const resp = await fetch(
    `${CONTROLLER_URL}/pub/v1/sandboxes/${id}/network-policy`,
    {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-Auth-Tenant": TENANT,
      },
      body: JSON.stringify(policy),
    },
  );

  if (!resp.ok) {
    throw new Error(`Network policy update failed: ${resp.status} ${await resp.text()}`);
  }

  const returned = (await resp.json()) as NetworkPolicy;
  if (returned.mode !== "rules") {
    throw new Error(`Expected mode "rules", got "${returned.mode}"`);
  }
  if (!returned.rules || returned.rules.length !== 2) {
    throw new Error(`Expected 2 rules, got ${returned.rules?.length ?? 0}`);
  }

  console.log("    Network policy applied and echoed back correctly");

  await terminate(id);
  console.log("==> PASSED: network policy update\n");
}

async function testSnapshot(): Promise<void> {
  console.log("==> Test: snapshot creation");

  const { id } = await createAndWait();

  const resp = await fetch(
    `${CONTROLLER_URL}/pub/v1/sandboxes/${id}/snapshot`,
    {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-Auth-Tenant": TENANT,
      },
      body: JSON.stringify({
        name: "walking-skeleton-snap",
        ttl_days: 3,
        tags: { env: "test", variant: "walking-skeleton" },
      }),
    },
  );

  if (!resp.ok) {
    throw new Error(`Snapshot creation failed: ${resp.status} ${await resp.text()}`);
  }

  const { snapshot } = (await resp.json()) as SnapshotResponse;

  if (!snapshot.id.startsWith("snap-")) {
    throw new Error(`Expected snapshot id to start with "snap-", got "${snapshot.id}"`);
  }
  if (snapshot.sandbox_id !== id) {
    throw new Error(`Snapshot sandbox_id "${snapshot.sandbox_id}" !== "${id}"`);
  }
  if (snapshot.name !== "walking-skeleton-snap") {
    throw new Error(`Snapshot name "${snapshot.name}" !== "walking-skeleton-snap"`);
  }
  if (snapshot.tenant !== TENANT) {
    throw new Error(`Snapshot tenant "${snapshot.tenant}" !== "${TENANT}"`);
  }
  if (snapshot.tags.env !== "test" || snapshot.tags.variant !== "walking-skeleton") {
    throw new Error(`Snapshot tags mismatch: ${JSON.stringify(snapshot.tags)}`);
  }

  const expiresAt = new Date(snapshot.expires_at);
  const now = new Date();
  const daysDiff = (expiresAt.getTime() - now.getTime()) / (1000 * 60 * 60 * 24);
  if (daysDiff < 2.5 || daysDiff > 3.5) {
    throw new Error(`Expected ~3 days TTL, got ${daysDiff.toFixed(1)} days`);
  }

  console.log(`    Snapshot created: ${snapshot.id}`);
  console.log(`    image_ref: ${snapshot.image_ref}`);
  console.log(`    expires_at: ${snapshot.expires_at}`);

  await terminate(id);
  console.log("==> PASSED: snapshot creation\n");
}

async function main() {
  console.log("Walking Skeleton — Variant Tests\n");

  await testNetworkPolicy();
  await testSnapshot();

  console.log("==> All variant tests PASSED!");
}

main().catch((err) => {
  console.error("FAILED:", err.message);
  process.exit(1);
});
