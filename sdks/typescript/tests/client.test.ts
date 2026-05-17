import { describe, it, expect, beforeAll, afterAll, afterEach } from "vitest";
import { http, HttpResponse } from "msw";
import { setupServer } from "msw/node";
import { SandboxClient } from "../src/client.js";
import type {
  Sandbox as SandboxType,
  CreateSandboxResponse,
  ListSandboxesResponse,
  SnapshotResponse,
  Snapshot,
  NetworkPolicy,
  CredentialBinding,
  CreateSecretResponse,
} from "../src/generated/types.js";

const CONTROL_URL = "http://localhost:9900";

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
  tags: { env: "dev" },
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
    url: "http://localhost:9901/tk-abc123/",
    status_url: `${CONTROL_URL}/pub/v1/sandboxes/sbx-test001/status`,
  },
};

const mockSnapshot: Snapshot = {
  id: "snap-001",
  sandbox_id: "sbx-test001",
  image: "registry.internal/snap-001:latest",
  runtime: "node24",
  tag: "checkpoint-1",
  size_bytes: 1048576,
  created_at: "2026-05-17T10:01:00Z",
};

const mockCredentialBinding: CredentialBinding = {
  id: "cred-bind-001",
  secret_id: "secret-abc",
  env_var: "API_KEY",
  created_at: "2026-05-17T10:02:00Z",
};

const mockSecretResponse: CreateSecretResponse = {
  id: "secret-new001",
  name: "my-api-key",
  created_at: "2026-05-17T10:03:00Z",
};

const handlers = [
  http.post(`${CONTROL_URL}/pub/v1/sandboxes`, () => {
    return HttpResponse.json(mockCreateResponse, { status: 201 });
  }),

  http.get(`${CONTROL_URL}/pub/v1/sandboxes`, ({ request }) => {
    const url = new URL(request.url);
    const response: ListSandboxesResponse = {
      items: [mockSandbox],
      next_cursor: url.searchParams.get("cursor") ? null : "cursor-page2",
    };
    return HttpResponse.json(response);
  }),

  http.get(`${CONTROL_URL}/pub/v1/sandboxes/:id`, ({ params }) => {
    if (params.id === "sbx-test001") {
      return HttpResponse.json(mockSandbox);
    }
    return HttpResponse.json(
      { id: "err-1", message: "Not found", kind: "not_found" },
      { status: 404 },
    );
  }),

  http.post(`${CONTROL_URL}/pub/v1/sandboxes/:id/snapshot`, () => {
    const response: SnapshotResponse = { snapshot: mockSnapshot };
    return HttpResponse.json(response, { status: 201 });
  }),

  http.post(`${CONTROL_URL}/pub/v1/sandboxes/:id/network-policy`, () => {
    const policy: NetworkPolicy = { mode: "rules", rules: [{ domains: ["*.example.com"], action: "allow" }] };
    return HttpResponse.json(policy);
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

  http.get(`${CONTROL_URL}/pub/v1/snapshots/:id`, ({ params }) => {
    if (params.id === "snap-001") {
      return HttpResponse.json(mockSnapshot);
    }
    return HttpResponse.json(
      { id: "err-2", message: "Not found", kind: "not_found" },
      { status: 404 },
    );
  }),

  http.delete(`${CONTROL_URL}/pub/v1/snapshots/:id`, () => {
    return new HttpResponse(null, { status: 204 });
  }),

  http.post(`${CONTROL_URL}/pub/v1/secrets`, () => {
    return HttpResponse.json(mockSecretResponse, { status: 201 });
  }),

  http.delete(`${CONTROL_URL}/pub/v1/secrets/:id`, () => {
    return new HttpResponse(null, { status: 204 });
  }),
];

const server = setupServer(...handlers);

beforeAll(() => server.listen({ onUnhandledRequest: "error" }));
afterEach(() => server.resetHandlers());
afterAll(() => server.close());

describe("SandboxClient", () => {
  function createClient() {
    return new SandboxClient({ controlUrl: CONTROL_URL, token: "test-token" });
  }

  describe("create", () => {
    it("creates a sandbox and returns a connected Sandbox instance", async () => {
      const client = createClient();
      const { sandbox, connection } = await client.create({
        runtime: "node24",
        resources: { vcpus: 2, memory_mb: 4096 },
      });

      expect(sandbox.id).toBe("sbx-test001");
      expect(sandbox.status).toBe("ready");
      expect(connection.token).toBe("tk-abc123");
      expect(connection.url).toBe("http://localhost:9901/tk-abc123/");
    });

    it("passes the full request body to the endpoint", async () => {
      let capturedBody: Record<string, unknown> | undefined;
      server.use(
        http.post(`${CONTROL_URL}/pub/v1/sandboxes`, async ({ request }) => {
          capturedBody = (await request.json()) as Record<string, unknown>;
          return HttpResponse.json(mockCreateResponse, { status: 201 });
        }),
      );

      const client = createClient();
      await client.create({
        runtime: "python313",
        trust: "paranoid",
        resources: { vcpus: 4, memory_mb: 8192 },
        env: { NODE_ENV: "production" },
        timeout_ms: 600_000,
        persistent: true,
        tags: { team: "backend" },
      });

      expect(capturedBody).toBeDefined();
      expect(capturedBody!.runtime).toBe("python313");
      expect(capturedBody!.trust).toBe("paranoid");
      expect(capturedBody!.resources).toEqual({ vcpus: 4, memory_mb: 8192 });
      expect(capturedBody!.env).toEqual({ NODE_ENV: "production" });
      expect(capturedBody!.timeout_ms).toBe(600_000);
      expect(capturedBody!.persistent).toBe(true);
      expect(capturedBody!.tags).toEqual({ team: "backend" });
    });
  });

  describe("list", () => {
    it("returns paginated sandbox list", async () => {
      const client = createClient();
      const result = await client.list();

      expect(result.items).toHaveLength(1);
      expect(result.items[0]!.id).toBe("sbx-test001");
      expect(result.next_cursor).toBe("cursor-page2");
    });

    it("passes query params for filtering", async () => {
      let capturedUrl: string | undefined;
      server.use(
        http.get(`${CONTROL_URL}/pub/v1/sandboxes`, ({ request }) => {
          capturedUrl = request.url;
          return HttpResponse.json({ items: [], next_cursor: null });
        }),
      );

      const client = createClient();
      await client.list({ status: "ready", limit: 50, cursor: "abc" });

      expect(capturedUrl).toBeDefined();
      const url = new URL(capturedUrl!);
      expect(url.searchParams.get("status")).toBe("ready");
      expect(url.searchParams.get("limit")).toBe("50");
      expect(url.searchParams.get("cursor")).toBe("abc");
    });
  });

  describe("get", () => {
    it("returns a sandbox by ID", async () => {
      const client = createClient();
      const sandbox = await client.get("sbx-test001");

      expect(sandbox.id).toBe("sbx-test001");
      expect(sandbox.runtime).toBe("node24");
      expect(sandbox.status).toBe("ready");
    });

    it("throws on 404", async () => {
      const client = createClient();
      await expect(client.get("sbx-nonexistent")).rejects.toThrow();
    });
  });

  describe("snapshot", () => {
    it("creates a snapshot of a sandbox", async () => {
      const client = createClient();
      const result = await client.snapshot("sbx-test001", { tag: "checkpoint-1" });

      expect(result.snapshot.id).toBe("snap-001");
      expect(result.snapshot.sandbox_id).toBe("sbx-test001");
      expect(result.snapshot.tag).toBe("checkpoint-1");
    });
  });

  describe("setNetworkPolicy", () => {
    it("updates the network policy", async () => {
      let capturedBody: Record<string, unknown> | undefined;
      server.use(
        http.post(
          `${CONTROL_URL}/pub/v1/sandboxes/:id/network-policy`,
          async ({ request }) => {
            capturedBody = (await request.json()) as Record<string, unknown>;
            return HttpResponse.json(capturedBody);
          },
        ),
      );

      const client = createClient();
      const policy: NetworkPolicy = {
        mode: "rules",
        rules: [{ domains: ["api.example.com"], action: "allow" }],
      };
      const result = await client.setNetworkPolicy("sbx-test001", policy);

      expect(capturedBody).toBeDefined();
      expect(capturedBody!.mode).toBe("rules");
      expect(result.mode).toBe("rules");
    });
  });

  describe("createCredential", () => {
    it("binds a credential to a sandbox", async () => {
      let capturedBody: Record<string, unknown> | undefined;
      server.use(
        http.post(
          `${CONTROL_URL}/pub/v1/sandboxes/:id/credentials`,
          async ({ request }) => {
            capturedBody = (await request.json()) as Record<string, unknown>;
            return HttpResponse.json(mockCredentialBinding, { status: 201 });
          },
        ),
      );

      const client = createClient();
      const result = await client.createCredential("sbx-test001", {
        secret_id: "secret-abc",
        env_var: "API_KEY",
      });

      expect(capturedBody).toBeDefined();
      expect(capturedBody!.secret_id).toBe("secret-abc");
      expect(result.id).toBe("cred-bind-001");
    });
  });

  describe("deleteCredential", () => {
    it("removes a credential binding", async () => {
      let calledPath: string | undefined;
      server.use(
        http.delete(
          `${CONTROL_URL}/pub/v1/sandboxes/:id/credentials/:credId`,
          ({ request }) => {
            calledPath = new URL(request.url).pathname;
            return new HttpResponse(null, { status: 204 });
          },
        ),
      );

      const client = createClient();
      await client.deleteCredential("sbx-test001", "cred-bind-001");

      expect(calledPath).toBe("/pub/v1/sandboxes/sbx-test001/credentials/cred-bind-001");
    });
  });

  describe("extend", () => {
    it("extends sandbox lifetime", async () => {
      let capturedBody: Record<string, unknown> | undefined;
      server.use(
        http.post(
          `${CONTROL_URL}/pub/v1/sandboxes/:id/extend`,
          async ({ request }) => {
            capturedBody = (await request.json()) as Record<string, unknown>;
            return HttpResponse.json(mockSandbox);
          },
        ),
      );

      const client = createClient();
      const result = await client.extend("sbx-test001", 300_000);

      expect(capturedBody).toBeDefined();
      expect(capturedBody!.additional_ms).toBe(300_000);
      expect(result.id).toBe("sbx-test001");
    });
  });

  describe("stop", () => {
    it("stops a sandbox with soft terminate", async () => {
      let calledUrl: string | undefined;
      server.use(
        http.delete(`${CONTROL_URL}/pub/v1/sandboxes/:id`, ({ request }) => {
          calledUrl = request.url;
          return new HttpResponse(null, { status: 204 });
        }),
      );

      const client = createClient();
      await client.stop("sbx-test001");

      expect(calledUrl).toBeDefined();
      const url = new URL(calledUrl!);
      expect(url.searchParams.has("force")).toBe(false);
    });

    it("stops a sandbox with force=true for hard terminate", async () => {
      let calledUrl: string | undefined;
      server.use(
        http.delete(`${CONTROL_URL}/pub/v1/sandboxes/:id`, ({ request }) => {
          calledUrl = request.url;
          return new HttpResponse(null, { status: 204 });
        }),
      );

      const client = createClient();
      await client.stop("sbx-test001", { force: true });

      expect(calledUrl).toBeDefined();
      const url = new URL(calledUrl!);
      expect(url.searchParams.get("force")).toBe("true");
    });
  });

  describe("getSnapshot", () => {
    it("fetches snapshot metadata", async () => {
      const client = createClient();
      const snapshot = await client.getSnapshot("snap-001");

      expect(snapshot.id).toBe("snap-001");
      expect(snapshot.sandbox_id).toBe("sbx-test001");
      expect(snapshot.image).toBe("registry.internal/snap-001:latest");
    });

    it("throws on 404", async () => {
      const client = createClient();
      await expect(client.getSnapshot("snap-nonexistent")).rejects.toThrow();
    });
  });

  describe("deleteSnapshot", () => {
    it("deletes a snapshot", async () => {
      let calledPath: string | undefined;
      server.use(
        http.delete(`${CONTROL_URL}/pub/v1/snapshots/:id`, ({ request }) => {
          calledPath = new URL(request.url).pathname;
          return new HttpResponse(null, { status: 204 });
        }),
      );

      const client = createClient();
      await client.deleteSnapshot("snap-001");

      expect(calledPath).toBe("/pub/v1/snapshots/snap-001");
    });
  });

  describe("createSecret", () => {
    it("creates a secret", async () => {
      let capturedBody: Record<string, unknown> | undefined;
      server.use(
        http.post(`${CONTROL_URL}/pub/v1/secrets`, async ({ request }) => {
          capturedBody = (await request.json()) as Record<string, unknown>;
          return HttpResponse.json(mockSecretResponse, { status: 201 });
        }),
      );

      const client = createClient();
      const result = await client.createSecret({
        name: "my-api-key",
        value: "sk-secret-value",
      });

      expect(capturedBody).toBeDefined();
      expect(capturedBody!.name).toBe("my-api-key");
      expect(capturedBody!.value).toBe("sk-secret-value");
      expect(result.id).toBe("secret-new001");
      expect(result.name).toBe("my-api-key");
    });
  });

  describe("deleteSecret", () => {
    it("deletes a secret", async () => {
      let calledPath: string | undefined;
      server.use(
        http.delete(`${CONTROL_URL}/pub/v1/secrets/:id`, ({ request }) => {
          calledPath = new URL(request.url).pathname;
          return new HttpResponse(null, { status: 204 });
        }),
      );

      const client = createClient();
      await client.deleteSecret("secret-abc");

      expect(calledPath).toBe("/pub/v1/secrets/secret-abc");
    });
  });

  describe("auth header", () => {
    it("sends Bearer token in Authorization header", async () => {
      let capturedAuth: string | null = null;
      server.use(
        http.get(`${CONTROL_URL}/pub/v1/sandboxes/:id`, ({ request }) => {
          capturedAuth = request.headers.get("Authorization");
          return HttpResponse.json(mockSandbox);
        }),
      );

      const client = createClient();
      await client.get("sbx-test001");

      expect(capturedAuth).toBe("Bearer test-token");
    });
  });
});
