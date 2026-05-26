import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { LiveSandboxCard } from "@/components/live-sandbox-card";
import type { Sandbox } from "@/api/types";

const sandbox: Sandbox = {
  id: "sb-live-1",
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

describe("LiveSandboxCard", () => {
  it("renders sandbox id and runtime", () => {
    render(
      <LiveSandboxCard
        sandbox={sandbox}
        language="typescript"
        onLanguageChange={() => {}}
      />,
    );
    expect(screen.getByText("sb-live-1")).toBeDefined();
    expect(screen.getByText("node24")).toBeDefined();
  });

  it("Open links to detail page", () => {
    render(
      <LiveSandboxCard
        sandbox={sandbox}
        language="typescript"
        onLanguageChange={() => {}}
      />,
    );
    const open = screen.getByRole("link", { name: "Open" });
    expect(open.getAttribute("href")).toBe("/sandboxes/sb-live-1");
  });

  it("shows Sandbox.get in connect snippet", () => {
    render(
      <LiveSandboxCard
        sandbox={sandbox}
        language="typescript"
        onLanguageChange={() => {}}
      />,
    );
    expect(screen.getByRole("code").textContent).toContain('Sandbox.get("sb-live-1")');
  });

  it("copies sandbox id", async () => {
    const user = userEvent.setup();
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("navigator", { ...navigator, clipboard: { writeText } });

    render(
      <LiveSandboxCard
        sandbox={sandbox}
        language="typescript"
        onLanguageChange={() => {}}
      />,
    );
    await user.click(screen.getByRole("button", { name: /Copy ID/i }));
    expect(writeText).toHaveBeenCalledWith("sb-live-1");
  });
});
