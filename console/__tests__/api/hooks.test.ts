import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createElement, type ReactNode } from "react";
import { useSandboxes, useSandbox } from "@/api/hooks";

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
        Promise.resolve({ sandbox: { id: "sb-7f3d1a", status: "ready" } }),
    });

    const { result } = renderHook(() => useSandbox("sb-7f3d1a"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.id).toBe("sb-7f3d1a");
  });
});
