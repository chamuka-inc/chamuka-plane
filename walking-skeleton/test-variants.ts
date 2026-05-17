/**
 * Walking Skeleton — Variant Tests
 *
 * Requires: docker compose up -d (walking-skeleton/docker-compose.yml)
 */

import { CONTROLLER_URL, createAndWait, postSandboxEndpoint, terminate, TENANT } from "./helpers";

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

async function testNetworkPolicy(): Promise<void> {
  console.log("==> Test: network policy update");
  const { id } = await createAndWait();
  console.log(`    Sandbox created: ${id}`);

  try {
    const policy: NetworkPolicy = {
      mode: "rules",
      rules: [
        { domains: ["*.example.com"], action: "allow" },
        { cidrs: ["10.0.0.0/8"], ports: [443], action: "deny" },
      ],
    };

    const resp = await postSandboxEndpoint(id, "network-policy", policy);
    const returned = (await resp.json()) as NetworkPolicy;

    if (returned.mode !== "rules") {
      throw new Error(`Expected mode "rules", got "${returned.mode}"`);
    }
    if (!returned.rules || returned.rules.length !== 2) {
      throw new Error(`Expected 2 rules, got ${returned.rules?.length ?? 0}`);
    }

    console.log("    Network policy applied and echoed back correctly");
  } finally {
    await terminate(id);
  }
  console.log("==> PASSED: network policy update\n");
}

async function testSnapshot(): Promise<void> {
  console.log("==> Test: snapshot creation");
  const { id } = await createAndWait();
  console.log(`    Sandbox created: ${id}`);

  try {
    const resp = await postSandboxEndpoint(id, "snapshot", {
      name: "walking-skeleton-snap",
      ttl_days: 3,
      tags: { env: "test", variant: "walking-skeleton" },
    });

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

    const expiresAt = new Date(snapshot.expires_at);
    const daysDiff = (expiresAt.getTime() - Date.now()) / (1000 * 60 * 60 * 24);
    if (daysDiff < 2.5 || daysDiff > 3.5) {
      throw new Error(`Expected ~3 days TTL, got ${daysDiff.toFixed(1)} days`);
    }

    console.log(`    Snapshot created: ${snapshot.id}`);
    console.log(`    expires_at: ${snapshot.expires_at}`);
  } finally {
    await terminate(id);
  }
  console.log("==> PASSED: snapshot creation\n");
}

async function testParanoidTierRouting(): Promise<void> {
  console.log("==> Test: paranoid tier routing (expect 503)");

  const resp = await fetch(`${CONTROLLER_URL}/pub/v1/sandboxes`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-Auth-Tenant": TENANT,
    },
    body: JSON.stringify({
      runtime: "custom",
      image: "sandbox-runtime:node24-latest",
      trust: "paranoid",
      resources: { vcpus: 1, memory_mb: 512 },
    }),
  });

  if (resp.status !== 503) {
    throw new Error(`Expected 503 (no paranoid drone), got ${resp.status}`);
  }

  const body = (await resp.json()) as { error: { code: string } };
  if (body.error.code !== "NO_DRONE_AVAILABLE") {
    throw new Error(`Expected NO_DRONE_AVAILABLE, got ${body.error.code}`);
  }

  console.log("    Controller correctly rejects paranoid request (no drone in pool)");
  console.log("==> PASSED: paranoid tier routing\n");
}

async function main() {
  console.log("Walking Skeleton — Variant Tests\n");

  const results = await Promise.allSettled([
    testNetworkPolicy(),
    testSnapshot(),
    testParanoidTierRouting(),
  ]);
  const failures = results.filter((r) => r.status === "rejected") as PromiseRejectedResult[];

  if (failures.length > 0) {
    for (const f of failures) {
      console.error("FAILED:", f.reason?.message ?? f.reason);
    }
    process.exit(1);
  }

  console.log("==> All variant tests PASSED!");
}

main().catch((err) => {
  console.error("FAILED:", err.message);
  process.exit(1);
});
