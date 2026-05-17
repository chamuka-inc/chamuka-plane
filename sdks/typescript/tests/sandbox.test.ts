import { describe, it, expect, beforeAll, afterAll, afterEach } from "vitest";
import { http, HttpResponse } from "msw";
import { setupServer } from "msw/node";
import { Sandbox } from "../src/sandbox.js";
import type { ConnectResponse } from "../src/generated/types.js";

const CONTROL_URL = "http://localhost:9900";
const DATA_PLANE_URL = "http://localhost:9901/tk-abc123/";

const mockConnectResponse: ConnectResponse = {
  backend_id: "bk-test001",
  spawned: true,
  status: "scheduled",
  token: "tk-abc123",
  url: DATA_PLANE_URL,
  status_url: `${CONTROL_URL}/pub/b/bk-test001/status`,
};

const handlers = [
  http.post(`${CONTROL_URL}/ctrl/connect`, () => {
    return HttpResponse.json(mockConnectResponse);
  }),

  http.post(`${DATA_PLANE_URL}v1/exec`, async ({ request }) => {
    const body = (await request.json()) as Record<string, unknown>;
    if (body.detached) {
      return HttpResponse.json(
        { cmd_id: "cmd-spawn01", status: "running" },
        { status: 202 },
      );
    }
    return HttpResponse.json({
      cmd_id: "cmd-sync01",
      exit_code: 0,
      stdout: "hello world\n",
      stderr: "",
      duration_ms: 42,
    });
  }),

  http.get(`${DATA_PLANE_URL}v1/exec/:cmdId`, ({ params }) => {
    return HttpResponse.json({
      cmd_id: params.cmdId,
      status: "exited",
      exit_code: 0,
    });
  }),

  http.post(`${DATA_PLANE_URL}v1/exec/:cmdId/stdin`, () => {
    return HttpResponse.json({});
  }),

  http.post(`${DATA_PLANE_URL}v1/exec/:cmdId/signal`, () => {
    return new HttpResponse(null, { status: 204 });
  }),

  http.get(`${DATA_PLANE_URL}v1/fs/read`, () => {
    return new HttpResponse("file content here", {
      headers: { "Content-Type": "application/octet-stream" },
    });
  }),

  http.put(`${DATA_PLANE_URL}v1/fs/write`, () => {
    return new HttpResponse(null, { status: 201 });
  }),

  http.post(`${DATA_PLANE_URL}v1/fs/mkdir`, () => {
    return HttpResponse.json({});
  }),

  http.delete(`${DATA_PLANE_URL}v1/fs/rm`, () => {
    return new HttpResponse(null, { status: 204 });
  }),

  http.get(`${DATA_PLANE_URL}v1/fs/list`, () => {
    return HttpResponse.json({
      path: "/workspace",
      entries: [
        { name: "main.py", type: "file", size: 1024, mode: 0o644, modified_at: "2026-03-10T14:30:00Z" },
        { name: "src", type: "dir", size: null, mode: 0o755, modified_at: "2026-03-10T14:30:00Z" },
      ],
    });
  }),

  http.post(`${DATA_PLANE_URL}v1/fs/move`, () => {
    return HttpResponse.json({});
  }),

  http.get(`${DATA_PLANE_URL}v1/ports`, () => {
    return HttpResponse.json({
      listening: [
        { port: 3000, protocol: "tcp", process_pid: 42, process_name: "node" },
      ],
    });
  }),
];

const server = setupServer(...handlers);

beforeAll(() => server.listen({ onUnhandledRequest: "error" }));
afterEach(() => server.resetHandlers());
afterAll(() => server.close());

describe("Sandbox.create", () => {
  it("connects via /ctrl/connect and returns a Sandbox instance", async () => {
    const sandbox = await Sandbox.create(
      { controlUrl: CONTROL_URL },
      { runtime: "node24" },
    );

    expect(sandbox.backendId).toBe("bk-test001");
    expect(sandbox.url).toBe(DATA_PLANE_URL);
    expect(sandbox.statusUrl).toBe(`${CONTROL_URL}/pub/b/bk-test001/status`);
  });

  it("passes spawn_config with pool and executable to /ctrl/connect", async () => {
    let capturedBody: Record<string, unknown> | undefined;
    server.use(
      http.post(`${CONTROL_URL}/ctrl/connect`, async ({ request }) => {
        capturedBody = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json(mockConnectResponse);
      }),
    );

    await Sandbox.create(
      { controlUrl: CONTROL_URL },
      {
        runtime: "python313",
        pool: "paranoid",
        env: { KEY: "val" },
        timeout_ms: 60_000,
      },
    );

    expect(capturedBody).toBeDefined();
    const spawn = capturedBody!.spawn_config as Record<string, unknown>;
    expect(spawn.pool).toBe("paranoid");
    expect(spawn.lifetime_limit_seconds).toBe(60);
    expect((spawn.executable as Record<string, unknown>).image).toBe("python:3.13-slim");
    expect((spawn.executable as Record<string, unknown>).env).toEqual({ KEY: "val" });
  });
});

describe("Sandbox.runCommand", () => {
  it("executes a synchronous command and returns result", async () => {
    const sandbox = await Sandbox.create(
      { controlUrl: CONTROL_URL },
      { runtime: "node24" },
    );

    const result = await sandbox.runCommand("echo", { args: ["hello", "world"] });

    expect(result.cmdId).toBe("cmd-sync01");
    expect(result.exitCode).toBe(0);
    expect(result.stdout).toBe("hello world\n");
    expect(result.stderr).toBe("");
    expect(result.durationMs).toBe(42);
    expect(result.truncated).toBe(false);
  });

  it("passes environment variables and cwd", async () => {
    let capturedBody: Record<string, unknown> | undefined;
    server.use(
      http.post(`${DATA_PLANE_URL}v1/exec`, async ({ request }) => {
        capturedBody = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({
          cmd_id: "cmd-env01",
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

    await sandbox.runCommand("ls", {
      cwd: "/app",
      env: { PATH: "/usr/bin" },
    });

    expect(capturedBody).toBeDefined();
    expect(capturedBody!.cwd).toBe("/app");
    expect(capturedBody!.env).toEqual({ PATH: "/usr/bin" });
    expect(capturedBody!.detached).toBe(false);
  });
});

describe("Sandbox.spawn", () => {
  it("spawns a detached process", async () => {
    const sandbox = await Sandbox.create(
      { controlUrl: CONTROL_URL },
      { runtime: "node24" },
    );

    const result = await sandbox.spawn("node", { args: ["server.js"] });
    expect(result.cmdId).toBe("cmd-spawn01");
  });
});

describe("Sandbox.getProcessStatus", () => {
  it("returns process status", async () => {
    const sandbox = await Sandbox.create(
      { controlUrl: CONTROL_URL },
      { runtime: "node24" },
    );

    const status = await sandbox.getProcessStatus("cmd-test01");
    expect(status.status).toBe("exited");
    expect(status.exitCode).toBe(0);
  });
});

describe("Sandbox.writeStdin", () => {
  it("writes data to stdin", async () => {
    let capturedBody: Record<string, unknown> | undefined;
    server.use(
      http.post(`${DATA_PLANE_URL}v1/exec/:cmdId/stdin`, async ({ request }) => {
        capturedBody = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({});
      }),
    );

    const sandbox = await Sandbox.create(
      { controlUrl: CONTROL_URL },
      { runtime: "node24" },
    );

    await sandbox.writeStdin("cmd-01", "hello\n");
    expect(capturedBody).toBeDefined();
    expect(capturedBody!.data).toBe("hello\n");
  });
});

describe("Sandbox.signal", () => {
  it("sends a signal to a process", async () => {
    let capturedBody: Record<string, unknown> | undefined;
    server.use(
      http.post(`${DATA_PLANE_URL}v1/exec/:cmdId/signal`, async ({ request }) => {
        capturedBody = (await request.json()) as Record<string, unknown>;
        return new HttpResponse(null, { status: 204 });
      }),
    );

    const sandbox = await Sandbox.create(
      { controlUrl: CONTROL_URL },
      { runtime: "node24" },
    );

    await sandbox.signal("cmd-01", "TERM");
    expect(capturedBody).toBeDefined();
    expect(capturedBody!.signal).toBe("TERM");
  });
});

describe("Sandbox filesystem operations", () => {
  it("reads a file and returns bytes", async () => {
    const sandbox = await Sandbox.create(
      { controlUrl: CONTROL_URL },
      { runtime: "node24" },
    );

    const bytes = await sandbox.readFile("/workspace/main.py");
    const text = new TextDecoder().decode(bytes);
    expect(text).toBe("file content here");
  });

  it("readTextFile returns string directly", async () => {
    const sandbox = await Sandbox.create(
      { controlUrl: CONTROL_URL },
      { runtime: "node24" },
    );

    const text = await sandbox.readTextFile("/workspace/main.py");
    expect(text).toBe("file content here");
  });

  it("writes a file via multipart upload", async () => {
    const sandbox = await Sandbox.create(
      { controlUrl: CONTROL_URL },
      { runtime: "node24" },
    );

    await expect(
      sandbox.writeFile("/workspace/output.txt", "hello"),
    ).resolves.toBeUndefined();
  });

  it("creates a directory", async () => {
    let capturedBody: Record<string, unknown> | undefined;
    server.use(
      http.post(`${DATA_PLANE_URL}v1/fs/mkdir`, async ({ request }) => {
        capturedBody = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({});
      }),
    );

    const sandbox = await Sandbox.create(
      { controlUrl: CONTROL_URL },
      { runtime: "node24" },
    );

    await sandbox.mkdir("/workspace/new-dir", { recursive: true });
    expect(capturedBody).toBeDefined();
    expect(capturedBody!.path).toBe("/workspace/new-dir");
    expect(capturedBody!.recursive).toBe(true);
  });

  it("removes a file", async () => {
    const sandbox = await Sandbox.create(
      { controlUrl: CONTROL_URL },
      { runtime: "node24" },
    );

    await expect(
      sandbox.rm("/workspace/old.txt"),
    ).resolves.toBeUndefined();
  });

  it("lists directory entries", async () => {
    const sandbox = await Sandbox.create(
      { controlUrl: CONTROL_URL },
      { runtime: "node24" },
    );

    const entries = await sandbox.listDir("/workspace");
    expect(entries).toHaveLength(2);
    expect(entries[0]!.name).toBe("main.py");
    expect(entries[0]!.type).toBe("file");
    expect(entries[1]!.name).toBe("src");
    expect(entries[1]!.type).toBe("dir");
  });

  it("moves a file", async () => {
    let capturedBody: Record<string, unknown> | undefined;
    server.use(
      http.post(`${DATA_PLANE_URL}v1/fs/move`, async ({ request }) => {
        capturedBody = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({});
      }),
    );

    const sandbox = await Sandbox.create(
      { controlUrl: CONTROL_URL },
      { runtime: "node24" },
    );

    await sandbox.move("/workspace/a.txt", "/workspace/b.txt");
    expect(capturedBody).toBeDefined();
    expect(capturedBody!.source).toBe("/workspace/a.txt");
    expect(capturedBody!.destination).toBe("/workspace/b.txt");
  });
});

describe("Sandbox.listPorts", () => {
  it("returns listening ports", async () => {
    const sandbox = await Sandbox.create(
      { controlUrl: CONTROL_URL },
      { runtime: "node24" },
    );

    const ports = await sandbox.listPorts();
    expect(ports).toHaveLength(1);
    expect(ports[0]!.port).toBe(3000);
    expect(ports[0]!.protocol).toBe("tcp");
    expect(ports[0]!.process_name).toBe("node");
  });
});

describe("Sandbox.waitForPort", () => {
  it("resolves immediately when port is available", async () => {
    const sandbox = await Sandbox.create(
      { controlUrl: CONTROL_URL },
      { runtime: "node24" },
    );

    await expect(sandbox.waitForPort(3000)).resolves.toBeUndefined();
  });

  it("times out when port never appears", async () => {
    server.use(
      http.get(`${DATA_PLANE_URL}v1/ports`, () => {
        return HttpResponse.json({ listening: [] });
      }),
    );

    const sandbox = await Sandbox.create(
      { controlUrl: CONTROL_URL },
      { runtime: "node24" },
    );

    await expect(
      sandbox.waitForPort(9999, 1000),
    ).rejects.toThrow("port 9999 did not become available within 1000ms");
  });
});

describe("Sandbox.fromConnection", () => {
  it("creates a sandbox from an existing connection", () => {
    const sandbox = Sandbox.fromConnection(DATA_PLANE_URL, mockConnectResponse);
    expect(sandbox.backendId).toBe("bk-test001");
    expect(sandbox.url).toBe(DATA_PLANE_URL);
  });
});
