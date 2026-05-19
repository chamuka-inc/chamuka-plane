import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import QuickstartPage from "@/app/quickstart/page";
import type { Sandbox } from "@/api/types";

const mockUseSandboxes = vi.fn();
const mockPush = vi.fn();
const mockCreateMutate = vi.fn();

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: mockPush }),
}));

vi.mock("@/api/hooks", () => ({
  useSandboxes: (params?: { status?: string }) => mockUseSandboxes(params),
  useCreateSandbox: () => ({
    mutate: mockCreateMutate,
    isPending: false,
    error: null,
  }),
  useBindSecrets: () => ({
    mutate: vi.fn(),
  }),
}));

const readySandbox: Sandbox = {
  id: "sb-ready-1",
  backend_id: "bk-1",
  tenant: "default",
  status: "ready",
  runtime: "node24",
  trust: "untrusted",
  arch: "amd64",
  image: "sandbox-runtime:node24-latest",
  resources: { vcpus: 2, memory_mb: 4096 },
  ports: [3000],
  tags: {},
  persistent: false,
  created_at: "2026-05-19T00:00:00Z",
  updated_at: "2026-05-19T00:00:00Z",
  expires_at: "2026-05-19T01:00:00Z",
  cluster: "default",
};

function renderPage() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <QuickstartPage />
    </QueryClientProvider>,
  );
}

describe("QuickstartPage", () => {
  beforeEach(() => {
    mockPush.mockReset();
    mockCreateMutate.mockReset();
    mockUseSandboxes.mockReturnValue({
      data: { items: [] },
      isLoading: false,
      error: null,
    });
  });

  it("renders hero and all four steps", () => {
    renderPage();
    expect(screen.getByText("Run code in secure sandboxes")).toBeDefined();
    expect(screen.getByText("Create a sandbox")).toBeDefined();
    expect(screen.getByText("Run code")).toBeDefined();
    expect(screen.getByText("Interact with the sandbox")).toBeDefined();
    expect(screen.getByText("Connect to a running sandbox")).toBeDefined();
  });

  it("renders Try it live section", () => {
    renderPage();
    expect(screen.getByText("Try it live")).toBeDefined();
    expect(screen.getByText(/Boot a sandbox and open the terminal/)).toBeDefined();
    expect(screen.getByRole("button", { name: /Node.js 24/i })).toBeDefined();
  });

  it("shows empty state linking to try-live anchor", () => {
    renderPage();
    expect(screen.getByText(/No sandboxes running/)).toBeDefined();
    const link = screen.getByRole("link", { name: /Boot one below/i });
    expect(link.getAttribute("href")).toBe("#try-live");
  });

  it("renders live sandbox cards when API returns ready sandboxes", () => {
    mockUseSandboxes.mockReturnValue({
      data: { items: [readySandbox] },
      isLoading: false,
      error: null,
    });
    renderPage();
    expect(screen.getByText("sb-ready-1")).toBeDefined();
    expect(screen.getByRole("link", { name: "Open" }).getAttribute("href")).toBe(
      "/sandboxes/sb-ready-1",
    );
  });

  it("redirects to sandbox detail after create", async () => {
    const user = userEvent.setup();
    mockCreateMutate.mockImplementation((_body: unknown, opts: { onSuccess: (data: { sandbox: { id: string } }) => void }) => {
      opts?.onSuccess?.({ sandbox: { id: "sb-booted" } });
    });
    renderPage();
    await user.click(screen.getByRole("button", { name: /Node.js 24/i }));
    await waitFor(() => {
      expect(mockPush).toHaveBeenCalledWith("/sandboxes/sb-booted");
    });
  });

  it("switches to desktop step content when Desktop env is selected", async () => {
    const user = userEvent.setup();
    renderPage();
    const envGroup = screen.getByRole("group", { name: "Environment" });
    await user.click(envGroup.querySelectorAll("button")[1]);
    await waitFor(() => {
      expect(screen.getByText("Capture the screen")).toBeDefined();
      expect(screen.getByText("Automate the desktop")).toBeDefined();
    });
    const stepCode = screen
      .getAllByRole("code")
      .find((el) => el.textContent?.includes("screenshot"));
    expect(stepCode).toBeDefined();
  });

  it("defaults CLI create snippet to node24", () => {
    renderPage();
    expect(screen.getAllByRole("code")[0].textContent).toContain('"node24"');
  });

  it("uses python313 in CLI create snippet when Python is selected", async () => {
    const user = userEvent.setup();
    renderPage();
    await user.click(screen.getAllByRole("button", { name: "Python" })[0]);
    await waitFor(() => {
      expect(screen.getAllByRole("code")[0].textContent).toContain("python313");
    });
  });

  it("persists language selection across steps", async () => {
    const user = userEvent.setup();
    renderPage();
    await user.click(screen.getAllByRole("button", { name: "Python" })[0]);
    await waitFor(() => {
      const codes = screen.getAllByRole("code");
      expect(codes[0].textContent).toContain("from sandbox_platform");
      expect(codes[1].textContent).toContain("run_command");
    });
  });

  it("fetches only ready sandboxes", () => {
    renderPage();
    expect(mockUseSandboxes).toHaveBeenCalledWith({ status: "ready" });
  });

  it("does not render terminated sandboxes from API", () => {
    mockUseSandboxes.mockReturnValue({
      data: {
        items: [
          readySandbox,
          { ...readySandbox, id: "sb-dead", status: "terminated" as const },
        ],
      },
      isLoading: false,
      error: null,
    });
    renderPage();
    expect(screen.getByText("sb-ready-1")).toBeDefined();
    expect(screen.queryByText("sb-dead")).toBeNull();
  });
});
