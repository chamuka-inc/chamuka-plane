import { describe, it, expect, beforeAll, afterAll, afterEach } from "vitest";
import { http, HttpResponse } from "msw";
import { setupServer } from "msw/node";
import { SandboxClient } from "../src/client.js";
import type {
  CreateSandboxResponse,
  Sandbox as SandboxType,
  SnapshotResponse,
  Snapshot,
  NetworkPolicy,
  CredentialBinding,
} from "../src/generated/types.js";

const CONTROL_URL = "http://localhost:9900";
const DATA_PLANE_URL = "http://localhost:9901/tk-abc123/";

const mockSandbox: SandboxType = {
  id: "sbx-test001",
  backend_id: "bk-test001",
  tenant: "tenant-abc",
  status: "ready",
  runtime: "node24",
  trust: "untrusted",
  arch: "amd64",
  image: "node:24-slim",
  resources: { vcpus: 2, memory_mb: 4096 },
  ports: [3000],
  tags: {},
  persistent: false,
  created_at: "2026-05-17T10:00:00Z",
  updated_at: "2026-05-17T10:00:05Z",
  expires_at: "2026-05-17T10:05:00Z",
  ready_at: "2026-05-17T10:00:05Z",
  cluster: "default",
};

const mockCreateResponse: CreateSandboxResponse = {
  sandbox: mockSandbox,
  connection: {
    token: "tk-abc123",
    url: DATA_PLANE_URL,
    status_url: `${CONTROL_URL}/pub/v1/sandboxes/sbx-test001/status`,
  },
};

const mockSnapshot: Snapshot = {
  id: "snap-001",
  sandbox_id: "sbx-test001",
  image: "registry.internal/snap-001:latest",
  runtime: "node24",
  tag: "v1",
  size_bytes: 2048,
  created_at: "2026-05-17T10:01:00Z",
};

const mockCredentialBinding: CredentialBinding = {
  id: "cred-bind-001",
  secret_id: "secret-abc",
  env_var: "API_KEY",
  created_at: "2026-05-17T10:02:00Z",
};

const handlers = [
  http.post(`${CONTROL_URL}/pub/v1/sandboxes`, () => {
    return HttpResponse.json(mockCreateResponse, { status: 201 });
  }),

  http.post(`${CONTROL_URL}/pub/v1/sandboxes/:id/snapshot`, () => {
    const response: SnapshotResponse = { snapshot: mockSnapshot };
    return HttpResponse.json(response, { status: 201 });
  }),

  http.post(`${CONTROL_URL}/pub/v1/sandboxes/:id/network-policy`, async ({ request }) => {
    const body = await request.json();
    return HttpResponse.json(body);
  }),

  http.post(`${CONTROL_URL}/pub/v1/sandboxes/:id/credentials`, () => {
    return HttpResponse.json(mockCredentialBinding, { status: 201 });
  }),

  http.delete(`${CONTROL_URL}/pub/v1/sandboxes/:id/credentials/:credId`, () => {
    return new HttpResponse(null, { status: 204 });
  }),

  http.post(`${CONTROL_URL}/pub/v1/sandboxes/:id/extend`, () => {
    return HttpResponse.json(mockSandbox);
  }),

  http.delete(`${CONTROL_URL}/pub/v1/sandboxes/:id`, () => {
    return new HttpResponse(null, { status: 204 });
  }),
];

const server = setupServer(...handlers);

beforeAll(() => server.listen({ onUnhandledRequest: "bypass" }));
afterEach(() => server.resetHandlers());
afterAll(() => server.close());

describe("Sandbox instance control-plane methods", () => {
  async function createSandbox() {
    const client = new SandboxClient({ controlUrl: CONTROL_URL, token: "test-token" });
    return client.createSandbox({ runtime: "node24", resources: { vcpus: 2, memory_mb: 4096 } });
  }

  it("snapshot() creates a snapshot of this sandbox", async () => {
    const sandbox = await createSandbox();
    const result = await sandbox.snapshot({ tag: "v1" });

    expect(result.snapshot.id).toBe("snap-001");
    expect(result.snapshot.sandbox_id).toBe("sbx-test001");
  });

  it("setNetworkPolicy() updates the policy", async () => {
    const sandbox = await createSandbox();
    const policy: NetworkPolicy = {
      mode: "deny-all",
    };
    const result = await sandbox.setNetworkPolicy(policy);

    expect(result.mode).toBe("deny-all");
  });

  it("createCredential() binds a secret", async () => {
    const sandbox = await createSandbox();
    const result = await sandbox.createCredential({
      secret_id: "secret-abc",
      env_var: "API_KEY",
    });

    expect(result.id).toBe("cred-bind-001");
  });

  it("deleteCredential() removes a binding", async () => {
    const sandbox = await createSandbox();
    await expect(sandbox.deleteCredential("cred-bind-001")).resolves.toBeUndefined();
  });

  it("extend() extends the sandbox lifetime", async () => {
    const sandbox = await createSandbox();
    const result = await sandbox.extend(60_000);

    expect(result.id).toBe("sbx-test001");
  });

  it("stop() terminates the sandbox", async () => {
    const sandbox = await createSandbox();
    await expect(sandbox.stop()).resolves.toBeUndefined();
  });

  it("stop({ force: true }) hard-terminates", async () => {
    let calledUrl: string | undefined;
    server.use(
      http.delete(`${CONTROL_URL}/pub/v1/sandboxes/:id`, ({ request }) => {
        calledUrl = request.url;
        return new HttpResponse(null, { status: 204 });
      }),
    );

    const sandbox = await createSandbox();
    await sandbox.stop({ force: true });

    expect(calledUrl).toBeDefined();
    const url = new URL(calledUrl!);
    expect(url.searchParams.get("force")).toBe("true");
  });

  it("exposes sandbox metadata properties", async () => {
    const sandbox = await createSandbox();

    expect(sandbox.id).toBe("sbx-test001");
    expect(sandbox.backendId).toBe("bk-test001");
    expect(sandbox.status).toBe("ready");
    expect(sandbox.runtime).toBe("node24");
  });
});
