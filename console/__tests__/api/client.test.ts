import { describe, it, expect, vi, beforeEach } from "vitest";
import { createBffClient } from "@/api/client";

describe("BffClient", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("fetches sandboxes list from BFF route", async () => {
    const mockSandboxes = [
      {
        id: "sb-7f3d1a",
        backend_id: "bk-abc",
        tenant: "tenant-default",
        status: "ready",
        runtime: "node24",
        trust: "untrusted",
        arch: "amd64",
        image: "sandbox-runtime:node24",
        resources: { vcpus: 2, memory_mb: 4096 },
        ports: [3000],
        tags: { "team": "platform" },
        persistent: false,
        created_at: "2026-05-18T14:00:00Z",
        updated_at: "2026-05-18T14:02:00Z",
        expires_at: "2026-05-18T14:05:00Z",
        ready_at: "2026-05-18T14:02:01Z",
        drone_name: "dr-ec2-iad-01",
        cluster: "iad-x86-default",
      },
    ];

    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ items: mockSandboxes }),
    });

    const client = createBffClient();
    const result = await client.listSandboxes();

    expect(fetch).toHaveBeenCalledWith("/api/sandboxes?limit=100", {
      headers: { "Content-Type": "application/json" },
    });
    expect(result.items).toHaveLength(1);
    expect(result.items[0].id).toBe("sb-7f3d1a");
  });

  it("fetches single sandbox by id", async () => {
    const mockSandbox = {
      id: "sb-7f3d1a",
      backend_id: "bk-abc",
      tenant: "tenant-default",
      status: "ready",
      runtime: "node24",
      trust: "untrusted",
      arch: "amd64",
      image: "sandbox-runtime:node24",
      resources: { vcpus: 2, memory_mb: 4096 },
      ports: [3000],
      tags: {},
      persistent: false,
      created_at: "2026-05-18T14:00:00Z",
      updated_at: "2026-05-18T14:02:00Z",
      expires_at: "2026-05-18T14:05:00Z",
      cluster: "iad-x86-default",
    };

    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ sandbox: mockSandbox }),
    });

    const client = createBffClient();
    const result = await client.getSandbox("sb-7f3d1a");

    expect(fetch).toHaveBeenCalledWith("/api/sandboxes/sb-7f3d1a", {
      headers: { "Content-Type": "application/json" },
    });
    expect(result.id).toBe("sb-7f3d1a");
  });

  it("throws on non-ok response", async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
      json: () =>
        Promise.resolve({
          error: { code: "INTERNAL", message: "db error", request_id: "r-1" },
        }),
    });

    const client = createBffClient();
    await expect(client.listSandboxes()).rejects.toThrow("INTERNAL");
  });
});
