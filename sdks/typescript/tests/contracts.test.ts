import { describe, it, expect, beforeAll, afterAll, afterEach } from "vitest";
import { http, HttpResponse } from "msw";
import { setupServer } from "msw/node";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { Sandbox } from "../src/sandbox.js";
import type { ConnectResponse, ExecSyncResponse } from "../src/generated/types.js";

const FIXTURES_DIR = resolve(import.meta.dirname, "../../../contracts/fixtures");
const CONTROL_URL = "http://localhost:9910";
const DATA_PLANE_URL = "http://localhost:9911/tk-contract/";

function loadFixture<T>(path: string): T {
  const content = readFileSync(resolve(FIXTURES_DIR, path), "utf-8");
  return JSON.parse(content) as T;
}

const mockConnectResponse: ConnectResponse = {
  backend_id: "bk-contract01",
  spawned: true,
  status: "scheduled",
  token: "tk-contract",
  url: DATA_PLANE_URL,
  status_url: `${CONTROL_URL}/pub/b/bk-contract01/status`,
};

const server = setupServer();

beforeAll(() => server.listen({ onUnhandledRequest: "error" }));
afterEach(() => server.resetHandlers());
afterAll(() => server.close());

describe("contract: exec-request fixtures", () => {
  it("sends a body matching the full exec-request fixture shape", async () => {
    const fixture = loadFixture<Record<string, unknown>>("agentd/exec-request.full.json");
    let capturedBody: Record<string, unknown> | undefined;

    server.use(
      http.post(`${CONTROL_URL}/ctrl/connect`, () => {
        return HttpResponse.json(mockConnectResponse);
      }),
      http.post(`${DATA_PLANE_URL}v1/exec`, async ({ request }) => {
        capturedBody = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({
          cmd_id: "cmd-fixture01",
          exit_code: 0,
          stdout: "",
          stderr: "",
          duration_ms: 10,
        });
      }),
    );

    const sandbox = await Sandbox.create(
      { controlUrl: CONTROL_URL },
      { runtime: "node24" },
    );

    await sandbox.runCommand(fixture.cmd as string, {
      args: fixture.args as string[],
      env: fixture.env as Record<string, string>,
      cwd: fixture.cwd as string,
      stdin: fixture.stdin as string,
      timeout_ms: fixture.timeout_ms as number,
      user: fixture.user as string,
    });

    expect(capturedBody).toBeDefined();
    expect(capturedBody!.cmd).toBe(fixture.cmd);
    expect(capturedBody!.args).toEqual(fixture.args);
    expect(capturedBody!.env).toEqual(fixture.env);
    expect(capturedBody!.cwd).toBe(fixture.cwd);
    expect(capturedBody!.stdin).toBe(fixture.stdin);
    expect(capturedBody!.timeout_ms).toBe(fixture.timeout_ms);
    expect(capturedBody!.user).toBe(fixture.user);
    expect(capturedBody!.detached).toBe(false);
  });

  it("sends minimal exec-request (cmd only)", async () => {
    const fixture = loadFixture<Record<string, unknown>>("agentd/exec-request.minimal.json");
    let capturedBody: Record<string, unknown> | undefined;

    server.use(
      http.post(`${CONTROL_URL}/ctrl/connect`, () => {
        return HttpResponse.json(mockConnectResponse);
      }),
      http.post(`${DATA_PLANE_URL}v1/exec`, async ({ request }) => {
        capturedBody = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({
          cmd_id: "cmd-min01",
          exit_code: 0,
          stdout: "",
          stderr: "",
          duration_ms: 5,
        });
      }),
    );

    const sandbox = await Sandbox.create(
      { controlUrl: CONTROL_URL },
      { runtime: "node24" },
    );

    await sandbox.runCommand(fixture.cmd as string);

    expect(capturedBody).toBeDefined();
    expect(capturedBody!.cmd).toBe(fixture.cmd);
    expect(capturedBody!.detached).toBe(false);
    expect(capturedBody!.args).toBeUndefined();
  });
});

describe("contract: exec-sync-response fixture", () => {
  it("parses the fixture response correctly", async () => {
    const fixture = loadFixture<ExecSyncResponse>("agentd/exec-sync-response.json");

    server.use(
      http.post(`${CONTROL_URL}/ctrl/connect`, () => {
        return HttpResponse.json(mockConnectResponse);
      }),
      http.post(`${DATA_PLANE_URL}v1/exec`, () => {
        return HttpResponse.json(fixture);
      }),
    );

    const sandbox = await Sandbox.create(
      { controlUrl: CONTROL_URL },
      { runtime: "node24" },
    );

    const result = await sandbox.runCommand("python3");

    expect(result.cmdId).toBe(fixture.cmd_id);
    expect(result.exitCode).toBe(fixture.exit_code);
    expect(result.stdout).toBe(fixture.stdout);
    expect(result.stderr).toBe(fixture.stderr);
    expect(result.durationMs).toBe(fixture.duration_ms);
  });
});

describe("contract: create-sandbox-response fixture", () => {
  it("the fixture response shape matches ConnectResponse expectations", () => {
    const fixture = loadFixture<{
      sandbox: Record<string, unknown>;
      connection: Record<string, unknown>;
    }>("api/create-sandbox-response.json");

    expect(fixture.connection.token).toBeDefined();
    expect(fixture.connection.url).toBeDefined();
    expect(fixture.connection.status_url).toBeDefined();
    expect(fixture.sandbox.id).toBeDefined();
    expect(fixture.sandbox.status).toBeDefined();
  });
});

describe("contract: sandbox fixture", () => {
  it("full sandbox fixture has all required fields", () => {
    const fixture = loadFixture<Record<string, unknown>>("sandbox/full.json");

    expect(fixture.id).toBeDefined();
    expect(fixture.backend_id).toBeDefined();
    expect(fixture.tenant).toBeDefined();
    expect(fixture.status).toBeDefined();
    expect(fixture.runtime).toBeDefined();
    expect(fixture.trust).toBeDefined();
    expect(fixture.arch).toBeDefined();
    expect(fixture.image).toBeDefined();
    expect(fixture.resources).toBeDefined();
    expect(fixture.ports).toBeDefined();
    expect(fixture.persistent).toBeDefined();
    expect(fixture.created_at).toBeDefined();
    expect(fixture.updated_at).toBeDefined();
    expect(fixture.expires_at).toBeDefined();
    expect(fixture.cluster).toBeDefined();
  });
});
