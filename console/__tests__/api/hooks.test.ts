import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createElement, type ReactNode } from "react";
import { useSandboxes, useFleet, useUsage, useSandbox } from "@/api/hooks";

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return ({ children }: { children: ReactNode }) =>
    createElement(QueryClientProvider, { client: queryClient }, children);
}

describe("useSandboxes", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("fetches sandbox list", async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () =>
        Promise.resolve({
          items: [{ id: "sb-1", status: "ready", runtime: "node24" }],
        }),
    });

    const { result } = renderHook(() => useSandboxes(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.items).toHaveLength(1);
    expect(result.current.data?.items[0].id).toBe("sb-1");
  });
});

describe("useSandbox", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("fetches single sandbox", async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () =>
        Promise.resolve({ id: "sb-7f3d1a", status: "ready" }),
    });

    const { result } = renderHook(() => useSandbox("sb-7f3d1a"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.id).toBe("sb-7f3d1a");
  });
});

describe("useFleet", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("fetches fleet state", async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () =>
        Promise.resolve({
          clusters: [
            { name: "iad-x86-default", drones: [], proxies: [] },
          ],
        }),
    });

    const { result } = renderHook(() => useFleet(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.clusters).toHaveLength(1);
  });
});

describe("useUsage", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("fetches usage data", async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () =>
        Promise.resolve({
          records: [],
          summary: {
            total_cpu_hours: 4.2,
            total_memory_gb_hours: 8.6,
            total_rx_bytes: 0,
            total_tx_bytes: 0,
            estimated_cost: 0.84,
          },
        }),
    });

    const { result } = renderHook(() => useUsage({ period: "24h" }), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.summary.total_cpu_hours).toBe(4.2);
  });
});
