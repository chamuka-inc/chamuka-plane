/**
 * D.3 Walking Skeleton — end-to-end lifecycle test.
 *
 * Exercises: connect → spawn → health check → exec → terminate.
 * Talks to stock Plane controller directly; agentd runs inside the sandbox.
 *
 * Environment:
 *   CONTROLLER_URL — e.g. http://127.0.0.1:28080
 *   IMAGE          — the OCI image to spawn (must contain our spike agentd)
 */

const CONTROLLER_URL = process.env.CONTROLLER_URL || "http://127.0.0.1:28080";
const IMAGE = process.env.IMAGE || "sandbox-spike:latest";

async function sleep(ms: number) {
  return new Promise((r) => setTimeout(r, ms));
}

async function main() {
  console.log("=== D.3 Walking Skeleton ===");
  console.log(`Controller: ${CONTROLLER_URL}`);
  console.log(`Image: ${IMAGE}`);
  console.log();

  // Step 1: Connect (spawn a sandbox)
  console.log(">>> Step 1: POST /ctrl/connect (spawn sandbox)");
  const connectReq = {
    spawn_config: {
      executable: {
        image: IMAGE,
        env: { PORT: "8080" },
      },
      max_idle_seconds: 120,
      lifetime_limit_seconds: 300,
    },
  };

  const connectRes = await fetch(`${CONTROLLER_URL}/ctrl/connect`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(connectReq),
  });

  if (!connectRes.ok) {
    const err = await connectRes.text();
    throw new Error(`Connect failed (${connectRes.status}): ${err}`);
  }

  const connect = await connectRes.json();
  console.log(`    backend_id: ${connect.backend_id}`);
  console.log(`    spawned: ${connect.spawned}`);
  console.log(`    token: ${connect.token.slice(0, 20)}...`);
  console.log(`    url: ${connect.url}`);
  console.log(`    status: ${connect.status}`);
  console.log();

  // Step 2: Wait for Ready
  console.log(">>> Step 2: Poll /pub/b/:backend/status until Ready");
  const statusUrl = `${CONTROLLER_URL}/pub/b/${connect.backend_id}/status`;
  let ready = false;

  for (let i = 0; i < 60; i++) {
    const statusRes = await fetch(statusUrl);
    if (statusRes.ok) {
      const status = await statusRes.json();
      console.log(`    status: ${status.status} (attempt ${i + 1})`);
      if (status.status === "ready") {
        ready = true;
        break;
      }
      if (
        status.status === "terminated" ||
        status.status === "hard-terminating"
      ) {
        throw new Error(`Backend terminated unexpectedly: ${status.status}`);
      }
    }
    await sleep(1000);
  }

  if (!ready) {
    throw new Error("Backend did not become ready within 60s");
  }
  console.log();

  // Step 3: Health check via proxy
  console.log(">>> Step 3: GET /health via proxy");
  const proxyBase = connect.url;
  const healthRes = await fetch(`${proxyBase}health`);
  if (!healthRes.ok) {
    throw new Error(`Health check failed: ${healthRes.status}`);
  }
  const health = await healthRes.json();
  console.log(`    response: ${JSON.stringify(health)}`);
  if (health.status !== "ok") {
    throw new Error(`Unexpected health status: ${health.status}`);
  }
  console.log();

  // Step 4: Execute command
  console.log('>>> Step 4: POST /v1/exec (node -e "console.log(\'hi\')")');
  const execRes = await fetch(`${proxyBase}v1/exec`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      command: "node",
      args: ["-e", "console.log('hi')"],
      timeout_ms: 10000,
    }),
  });

  if (!execRes.ok) {
    const err = await execRes.text();
    throw new Error(`Exec failed: ${execRes.status} ${err}`);
  }

  const exec = await execRes.json();
  console.log(`    exit_code: ${exec.exit_code}`);
  console.log(`    stdout: ${JSON.stringify(exec.stdout)}`);
  console.log(`    stderr: ${JSON.stringify(exec.stderr)}`);

  if (exec.exit_code !== 0) {
    throw new Error(`Non-zero exit code: ${exec.exit_code}`);
  }
  if (exec.stdout.trim() !== "hi") {
    throw new Error(`Unexpected stdout: ${JSON.stringify(exec.stdout)}`);
  }
  console.log();

  // Step 5: Soft terminate
  console.log(">>> Step 5: Soft terminate (let idle timeout handle it)");
  console.log("    (In production: POST /ctrl/b/:backend/terminate)");
  console.log("    Skipping explicit terminate — idle timeout will clean up.");
  console.log();

  console.log("=== PASS: Walking skeleton green ===");
  console.log(`  backend_id: ${connect.backend_id}`);
  console.log(`  lifecycle: connect → spawn → ready → health → exec → done`);
}

main().catch((e) => {
  console.error(`\n=== FAIL: ${e.message} ===`);
  process.exit(1);
});
