/**
 * Walking Skeleton Test
 *
 * End-to-end test exercising:
 * 1. Create a sandbox via the controller API
 * 2. Wait for it to become ready
 * 3. Run a command via agentd's /v1/exec endpoint
 * 4. Verify output
 * 5. Terminate the sandbox
 *
 * Requires: docker compose up -d (walking-skeleton/docker-compose.yml)
 * and the sandbox-runtime:node24 image built and available.
 */

const CONTROLLER_URL = process.env.CONTROLLER_URL ?? "http://localhost:9090";
const PROXY_URL = process.env.PROXY_URL ?? "http://localhost:8080";

interface CreateSandboxResponse {
  sandbox: { id: string; status: string };
  connection: { token: string; url: string; status_url: string };
}

interface ExecSyncResponse {
  cmd_id: string;
  exit_code: number;
  stdout: string;
  stderr: string;
  duration_ms: number;
}

async function waitForReady(statusUrl: string, timeoutMs = 30000): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const resp = await fetch(statusUrl);
      if (resp.ok) {
        const body = await resp.json() as { status: string };
        if (body.status === "ready") return;
      }
    } catch {
      // Not ready yet
    }
    await new Promise((r) => setTimeout(r, 500));
  }
  throw new Error(`Sandbox did not become ready within ${timeoutMs}ms`);
}

async function main() {
  console.log("==> Creating sandbox...");

  const createResp = await fetch(`${CONTROLLER_URL}/pub/v1/sandboxes`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-Auth-Tenant": "walking-skeleton",
    },
    body: JSON.stringify({
      runtime: "custom",
      image: "sandbox-runtime:node24-latest",
      resources: { vcpus: 1, memory_mb: 512 },
      ports: [8080],
    }),
  });

  if (!createResp.ok) {
    throw new Error(`Create failed: ${createResp.status} ${await createResp.text()}`);
  }

  const { sandbox, connection } = (await createResp.json()) as CreateSandboxResponse;
  console.log(`==> Sandbox created: ${sandbox.id}`);
  console.log(`==> Waiting for ready...`);

  const statusUrl = connection.status_url.replace("http://0.0.0.0:9090", CONTROLLER_URL);
  await waitForReady(statusUrl);
  console.log(`==> Sandbox ready!`);

  const proxyPath = new URL(connection.url).pathname;
  const agentdUrl = `${PROXY_URL}${proxyPath}`;
  console.log(`==> Running command: node -e 'console.log("hi")' via ${agentdUrl}`);
  const execResp = await fetch(`${agentdUrl}v1/exec`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      cmd: "node",
      args: ["-e", 'console.log("hi")'],
      timeout_ms: 10000,
    }),
  });

  if (!execResp.ok) {
    throw new Error(`Exec failed: ${execResp.status} ${await execResp.text()}`);
  }

  const result = (await execResp.json()) as ExecSyncResponse;
  console.log(`==> Exit code: ${result.exit_code}`);
  console.log(`==> Stdout: ${JSON.stringify(result.stdout)}`);

  if (result.exit_code !== 0) {
    throw new Error(`Command failed with exit code ${result.exit_code}: ${result.stderr}`);
  }

  if (result.stdout.trim() !== "hi") {
    throw new Error(`Expected stdout "hi", got: ${JSON.stringify(result.stdout)}`);
  }

  console.log(`==> Terminating sandbox...`);
  const deleteResp = await fetch(`${CONTROLLER_URL}/pub/v1/sandboxes/${sandbox.id}`, {
    method: "DELETE",
    headers: { "X-Auth-Tenant": "walking-skeleton" },
  });

  if (!deleteResp.ok && deleteResp.status !== 404) {
    console.warn(`Delete returned ${deleteResp.status} (non-fatal)`);
  }

  console.log("==> Walking skeleton PASSED!");
}

main().catch((err) => {
  console.error("FAILED:", err.message);
  process.exit(1);
});
