import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { BootSandboxPanel } from "@/components/boot-sandbox-panel";

const mockMutate = vi.fn();
const mockBindMutate = vi.fn();

vi.mock("@/api/hooks", () => ({
  useCreateSandbox: () => ({
    mutate: mockMutate,
    isPending: false,
    error: null,
  }),
  useBindSecrets: () => ({
    mutate: mockBindMutate,
  }),
}));

function renderPanel(onBooted = vi.fn()) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <BootSandboxPanel onBooted={onBooted} />
    </QueryClientProvider>,
  );
}

describe("BootSandboxPanel", () => {
  beforeEach(() => {
    mockMutate.mockReset();
    mockBindMutate.mockReset();
    mockMutate.mockImplementation((_body: unknown, opts: { onSuccess: (data: { sandbox: { id: string; image: string } }) => void }) => {
      opts?.onSuccess?.({
        sandbox: { id: "sb-new-1", image: "192.168.56.1:5000/sandbox-runtime:node24-latest" },
      });
    });
  });

  it("renders four runtime options", () => {
    renderPanel();
    expect(screen.getByRole("button", { name: /Node.js 24/i })).toBeDefined();
    expect(screen.getByRole("button", { name: /Python 3.13/i })).toBeDefined();
    expect(screen.getByRole("button", { name: /Agent/i })).toBeDefined();
    expect(screen.getByRole("button", { name: /Desktop/i })).toBeDefined();
  });

  it("calls onBooted after successful create", async () => {
    const user = userEvent.setup();
    const onBooted = vi.fn();
    renderPanel(onBooted);

    await user.click(screen.getByRole("button", { name: /Node.js 24/i }));

    expect(mockMutate).toHaveBeenCalled();
    expect(onBooted).toHaveBeenCalledWith({
      id: "sb-new-1",
      runtime: "node24",
      image: "192.168.56.1:5000/sandbox-runtime:node24-latest",
    });
  });

  it("sets interactive VNC when booting desktop runtime", async () => {
    const user = userEvent.setup();
    renderPanel();

    await user.click(screen.getByRole("button", { name: /Desktop/i }));

    const callArgs = mockMutate.mock.calls[0][0];
    expect(callArgs.env.VNC_VIEWONLY).toBe("0");
    expect(callArgs.env.VNC_ENABLED).toBeUndefined();
  });

  it("does not inject VNC env vars for non-desktop runtimes", async () => {
    const user = userEvent.setup();
    renderPanel();

    await user.click(screen.getByRole("button", { name: /Node.js 24/i }));

    const callArgs = mockMutate.mock.calls[0][0];
    expect(callArgs.env?.VNC_ENABLED).toBeUndefined();
  });
});
