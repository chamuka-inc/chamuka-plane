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

  it("fetches fleet state from BFF route", async () => {
    const mockFleet = {
      clusters: [
        {
          name: "iad-x86-default",
          drones: [
            {
              ready: true,
              draining: false,
              last_heartbeat_age: 3,
              backend_count: 6,
              node: {
                name: "dr-ec2-iad-01",
                plane_version: "0.5.0",
                plane_hash: "abc123",
                controller: "co-1",
                controller_heartbeat_age: 1,
              },
            },
          ],
          proxies: [],
        },
      ],
    };

    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(mockFleet),
    });

    const client = createBffClient();
    const result = await client.getFleetState();

    expect(fetch).toHaveBeenCalledWith("/api/fleet", {
      headers: { "Content-Type": "application/json" },
    });
    expect(result.clusters).toHaveLength(1);
    expect(result.clusters[0].drones).toHaveLength(1);
  });

  it("fetches usage records from BFF route", async () => {
    const mockUsage = {
      records: [
        {
          sandbox_id: "sb-7f3d1a",
          bucket_start: "2026-05-18T13:00:00Z",
          cpu_nsec: 3600000000000,
          memory_mb_seconds: 14400000,
          rx_bytes: 1024000,
          tx_bytes: 512000,
        },
      ],
      summary: {
        total_cpu_hours: 4.2,
        total_memory_gb_hours: 8.6,
        total_rx_bytes: 1027604480,
        total_tx_bytes: 440401920,
        estimated_cost: 0.84,
      },
    };

    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(mockUsage),
    });

    const client = createBffClient();
    const result = await client.getUsage({ period: "24h" });

    expect(fetch).toHaveBeenCalledWith("/api/usage?period=24h", {
      headers: { "Content-Type": "application/json" },
    });
    expect(result.summary.total_cpu_hours).toBe(4.2);
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
