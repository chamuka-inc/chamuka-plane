import { describe, it, expect, beforeAll, afterAll, afterEach } from "vitest";
import { http, HttpResponse } from "msw";
import { setupServer } from "msw/node";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { Sandbox } from "../src/sandbox.js";
import { SandboxClient } from "../src/client.js";
import type {
  ConnectResponse,
  ExecSyncResponse,
  CreateSandboxRequest,
  CreateSandboxResponse,
  NetworkPolicy,
  Snapshot,
} from "../src/generated/types.js";

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

describe("contract: SandboxClient create with fixtures", () => {
  it("sends the full create-sandbox-request fixture verbatim", async () => {
    const fixture = loadFixture<CreateSandboxRequest>("api/create-sandbox-request.full.json");
    const responseFixture = loadFixture<CreateSandboxResponse>("api/create-sandbox-response.json");
    let capturedBody: Record<string, unknown> | undefined;

    server.use(
      http.post(`${CONTROL_URL}/pub/v1/sandboxes`, async ({ request }) => {
        capturedBody = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json(responseFixture, { status: 201 });
      }),
    );

    const client = new SandboxClient({ controlUrl: CONTROL_URL, token: "test" });
    await client.create(fixture);

    expect(capturedBody).toBeDefined();
    expect(capturedBody!.runtime).toBe(fixture.runtime);
    expect(capturedBody!.image).toBe(fixture.image);
    expect(capturedBody!.trust).toBe(fixture.trust);
    expect(capturedBody!.arch).toBe(fixture.arch);
    expect(capturedBody!.resources).toEqual(fixture.resources);
    expect(capturedBody!.timeout_ms).toBe(fixture.timeout_ms);
    expect(capturedBody!.source).toEqual(fixture.source);
    expect(capturedBody!.ports).toEqual(fixture.ports);
    expect(capturedBody!.env).toEqual(fixture.env);
    expect(capturedBody!.tags).toEqual(fixture.tags);
    expect(capturedBody!.persistent).toBe(fixture.persistent);
    expect(capturedBody!.initial_network_policy).toEqual(fixture.initial_network_policy);
    expect(capturedBody!.use_static_token).toBe(fixture.use_static_token);
    expect(capturedBody!.key).toEqual(fixture.key);
  });

  it("parses the create-sandbox-response fixture correctly", async () => {
    const responseFixture = loadFixture<CreateSandboxResponse>("api/create-sandbox-response.json");

    server.use(
      http.post(`${CONTROL_URL}/pub/v1/sandboxes`, () => {
        return HttpResponse.json(responseFixture, { status: 201 });
      }),
    );

    const client = new SandboxClient({ controlUrl: CONTROL_URL, token: "test" });
    const result = await client.create({ runtime: "node24", resources: { vcpus: 2, memory_mb: 4096 } });

    expect(result.sandbox.id).toBe(responseFixture.sandbox.id);
    expect(result.sandbox.backend_id).toBe(responseFixture.sandbox.backend_id);
    expect(result.sandbox.status).toBe(responseFixture.sandbox.status);
    expect(result.connection.token).toBe(responseFixture.connection.token);
    expect(result.connection.url).toBe(responseFixture.connection.url);
    expect(result.connection.status_url).toBe(responseFixture.connection.status_url);
  });
});

describe("contract: SandboxClient list with sandbox fixtures", () => {
  it("parses the sandbox fixture as a list item", async () => {
    const sandboxFixture = loadFixture<Record<string, unknown>>("sandbox/full.json");

    server.use(
      http.get(`${CONTROL_URL}/pub/v1/sandboxes`, () => {
        return HttpResponse.json({ items: [sandboxFixture], next_cursor: null });
      }),
    );

    const client = new SandboxClient({ controlUrl: CONTROL_URL, token: "test" });
    const result = await client.list();

    expect(result.items).toHaveLength(1);
    expect(result.items[0]!.id).toBe(sandboxFixture.id);
    expect(result.items[0]!.runtime).toBe(sandboxFixture.runtime);
    expect(result.items[0]!.trust).toBe(sandboxFixture.trust);
    expect(result.items[0]!.persistent).toBe(sandboxFixture.persistent);
  });
});

describe("contract: SandboxClient network policy with fixtures", () => {
  it("sends the rules network policy fixture verbatim", async () => {
    const fixture = loadFixture<NetworkPolicy>("network/rules.json");
    let capturedBody: Record<string, unknown> | undefined;

    server.use(
      http.post(`${CONTROL_URL}/pub/v1/sandboxes/:id/network-policy`, async ({ request }) => {
        capturedBody = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json(capturedBody);
      }),
    );

    const client = new SandboxClient({ controlUrl: CONTROL_URL, token: "test" });
    const result = await client.setNetworkPolicy("sbx-test", fixture);

    expect(capturedBody).toBeDefined();
    expect(capturedBody!.mode).toBe("rules");
    expect(capturedBody!.rules).toEqual(fixture.rules);
    expect(result.mode).toBe("rules");
  });

  it("sends allow-all fixture", async () => {
    const fixture = loadFixture<NetworkPolicy>("network/allow-all.json");
    let capturedBody: Record<string, unknown> | undefined;

    server.use(
      http.post(`${CONTROL_URL}/pub/v1/sandboxes/:id/network-policy`, async ({ request }) => {
        capturedBody = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json(capturedBody);
      }),
    );

    const client = new SandboxClient({ controlUrl: CONTROL_URL, token: "test" });
    await client.setNetworkPolicy("sbx-test", fixture);

    expect(capturedBody).toBeDefined();
    expect(capturedBody!.mode).toBe("allow-all");
  });

  it("sends deny-all fixture", async () => {
    const fixture = loadFixture<NetworkPolicy>("network/deny-all.json");
    let capturedBody: Record<string, unknown> | undefined;

    server.use(
      http.post(`${CONTROL_URL}/pub/v1/sandboxes/:id/network-policy`, async ({ request }) => {
        capturedBody = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json(capturedBody);
      }),
    );

    const client = new SandboxClient({ controlUrl: CONTROL_URL, token: "test" });
    await client.setNetworkPolicy("sbx-test", fixture);

    expect(capturedBody).toBeDefined();
    expect(capturedBody!.mode).toBe("deny-all");
  });
});

describe("contract: SandboxClient snapshot with fixtures", () => {
  it("parses the snapshot fixture correctly", async () => {
    const fixture = loadFixture<Record<string, unknown>>("snapshot/full.json");

    server.use(
      http.get(`${CONTROL_URL}/pub/v1/snapshots/:id`, () => {
        return HttpResponse.json(fixture);
      }),
    );

    const client = new SandboxClient({ controlUrl: CONTROL_URL, token: "test" });
    const result = await client.getSnapshot("snap-abc123");

    expect(result.id).toBe(fixture.id);
    expect(result.sandbox_id).toBe(fixture.sandbox_id);
    expect(result.image_ref).toBe(fixture.image_ref);
    expect(result.digest).toBe(fixture.digest);
    expect(result.base_image).toBe(fixture.base_image);
    expect(result.runtime).toBe(fixture.runtime);
    expect(result.name).toBe(fixture.name);
    expect(result.tags).toEqual(fixture.tags);
    expect(result.size_bytes).toBe(fixture.size_bytes);
    expect(result.arch).toBe(fixture.arch);
    expect(result.created_at).toBe(fixture.created_at);
    expect(result.expires_at).toBe(fixture.expires_at);
  });
});
