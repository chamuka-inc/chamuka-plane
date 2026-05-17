/**
 * Walking Skeleton — Base Test
 *
 * Requires: docker compose up -d (walking-skeleton/docker-compose.yml)
 * and the sandbox-runtime:node24 image built and available.
 */

import { PROXY_URL, createAndWait, rebaseUrl, terminate } from "./helpers";

interface ExecSyncResponse {
  cmd_id: string;
  exit_code: number;
  stdout: string;
  stderr: string;
  duration_ms: number;
}

async function main() {
  console.log("==> Creating sandbox...");
  const { id, connection } = await createAndWait();
  console.log(`==> Sandbox created: ${id}`);
  console.log(`==> Sandbox ready!`);

  const proxyPath = new URL(rebaseUrl(connection.url)).pathname;
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
  await terminate(id);

  console.log("==> Walking skeleton PASSED!");
}

main().catch((err) => {
  console.error("FAILED:", err.message);
  process.exit(1);
});
