import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { Sandbox } from "../../src/sandbox.js";
import type { ConnectResponse } from "../../src/generated/types.js";
import { execSync, spawn, type ChildProcess } from "node:child_process";
import { resolve } from "node:path";

const WORKSPACE_ROOT = resolve(import.meta.dirname, "../../../..");
const AGENTD_BINARY = resolve(WORKSPACE_ROOT, "crates/target/debug/agentd");

let agentdProcess: ChildProcess | undefined;
let agentdPort: number | undefined;
let sandbox: Sandbox;

beforeAll(async () => {
  execSync("cargo build -p agentd", {
    cwd: resolve(WORKSPACE_ROOT, "crates"),
    stdio: "pipe",
    env: { ...process.env, SQLX_OFFLINE: "true" },
  });

  const port = 18080 + Math.floor(Math.random() * 1000);
  agentdPort = port;

  agentdProcess = spawn(AGENTD_BINARY, [], {
    env: {
      ...process.env,
      AGENTD_PORT: String(port),
      AGENTD_HOST: "127.0.0.1",
    },
    stdio: "pipe",
  });

  await waitForReady(`http://127.0.0.1:${port}/health`, 10_000);

  const fakeConnectResponse: ConnectResponse = {
    backend_id: "bk-e2e-test",
    spawned: true,
    status: "ready",
    token: "tk-e2e",
    url: `http://127.0.0.1:${port}`,
    status_url: "http://localhost/status",
  };

  sandbox = Sandbox.fromConnection(
    `http://127.0.0.1:${port}`,
    fakeConnectResponse,
  );
}, 60_000);

afterAll(() => {
  if (agentdProcess) {
    agentdProcess.kill("SIGTERM");
  }
});

describe("e2e: runCommand", () => {
  it("executes echo and returns output", async () => {
    const result = await sandbox.runCommand("echo", {
      args: ["hello", "from", "sdk"],
    });

    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain("hello from sdk");
  });

  it("captures stderr", async () => {
    const result = await sandbox.runCommand("sh", {
      args: ["-c", "echo error >&2; exit 1"],
    });

    expect(result.exitCode).toBe(1);
    expect(result.stderr).toContain("error");
  });

  it("passes environment variables", async () => {
    const result = await sandbox.runCommand("sh", {
      args: ["-c", "echo $TEST_VAR"],
      env: { TEST_VAR: "sdk-works" },
    });

    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain("sdk-works");
  });

  it("respects cwd", async () => {
    const result = await sandbox.runCommand("pwd", { cwd: "/tmp" });
    expect(result.stdout.trim()).toMatch(/\/?tmp$/);
  });
});

describe("e2e: spawn + status", () => {
  it("spawns a detached process and queries status", async () => {
    const spawned = await sandbox.spawn("sleep", { args: ["10"] });
    expect(spawned.cmdId).toBeDefined();

    const status = await sandbox.getProcessStatus(spawned.cmdId);
    expect(status.status).toBe("running");

    await sandbox.signal(spawned.cmdId, "KILL");
  });
});

describe("e2e: filesystem", () => {
  it("writes and reads a file", async () => {
    await sandbox.writeFile("/tmp/sdk-test.txt", "hello sdk");
    const content = await sandbox.readTextFile("/tmp/sdk-test.txt");
    expect(content).toBe("hello sdk");
  });

  it("creates a directory and lists it", async () => {
    await sandbox.mkdir("/tmp/sdk-dir", { recursive: true });
    await sandbox.writeFile("/tmp/sdk-dir/a.txt", "aaa");

    const entries = await sandbox.listDir("/tmp/sdk-dir");
    expect(entries.some((e) => e.name === "a.txt")).toBe(true);
  });

  it("moves a file", async () => {
    await sandbox.writeFile("/tmp/sdk-move-src.txt", "moveme");
    await sandbox.move("/tmp/sdk-move-src.txt", "/tmp/sdk-move-dst.txt");

    const content = await sandbox.readTextFile("/tmp/sdk-move-dst.txt");
    expect(content).toBe("moveme");
  });

  it("removes a file", async () => {
    await sandbox.writeFile("/tmp/sdk-rm.txt", "deleteme");
    await sandbox.rm("/tmp/sdk-rm.txt");

    await expect(
      sandbox.readFile("/tmp/sdk-rm.txt"),
    ).rejects.toThrow();
  });
});

describe("e2e: ports", () => {
  it("lists ports (may be empty on test machine)", async () => {
    const ports = await sandbox.listPorts();
    expect(Array.isArray(ports)).toBe(true);
  });
});

async function waitForReady(url: string, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(url);
      if (response.ok) return;
    } catch {
      // not ready yet
    }
    await new Promise((resolve) => setTimeout(resolve, 200));
  }
  throw new Error(`agentd did not become ready at ${url} within ${timeoutMs}ms`);
}
