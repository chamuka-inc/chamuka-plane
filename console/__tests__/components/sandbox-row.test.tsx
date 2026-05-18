import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { SandboxRow } from "@/components/sandbox-row";
import type { Sandbox } from "@/api/types";

const readySandbox: Sandbox = {
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
  tags: { team: "platform" },
  persistent: false,
  created_at: "2026-05-18T14:00:00Z",
  updated_at: "2026-05-18T14:02:00Z",
  expires_at: "2026-05-18T14:05:00Z",
  ready_at: "2026-05-18T14:02:01Z",
  drone_name: "dr-ec2-iad-01",
  cluster: "iad-x86-default",
};

describe("SandboxRow", () => {
  it("renders sandbox id and status", () => {
    render(<SandboxRow sandbox={readySandbox} />);
    expect(screen.getByText("sb-7f3d1a")).toBeDefined();
    expect(screen.getByText("ready")).toBeDefined();
  });

  it("renders runtime and trust tier", () => {
    render(<SandboxRow sandbox={readySandbox} />);
    expect(screen.getByText(/node24/)).toBeDefined();
    expect(screen.getByText(/untrusted/)).toBeDefined();
  });

  it("shows Terminal button when ready", () => {
    render(<SandboxRow sandbox={readySandbox} />);
    expect(screen.getByText("Terminal")).toBeDefined();
  });

  it("hides Terminal button when terminated", () => {
    const terminated = { ...readySandbox, status: "terminated" as const };
    render(<SandboxRow sandbox={terminated} />);
    expect(screen.queryByText("Terminal")).toBeNull();
  });

  it("shows persistent badge when persistent", () => {
    const persistent = { ...readySandbox, persistent: true };
    render(<SandboxRow sandbox={persistent} />);
    expect(screen.getByText("persistent")).toBeDefined();
  });

  it("renders tags", () => {
    render(<SandboxRow sandbox={readySandbox} />);
    expect(screen.getByText("team:platform")).toBeDefined();
  });

  it("dims terminated sandboxes", () => {
    const terminated = { ...readySandbox, status: "terminated" as const };
    const { container } = render(<SandboxRow sandbox={terminated} />);
    expect(container.firstElementChild?.classList.contains("opacity-50")).toBe(true);
  });
});
