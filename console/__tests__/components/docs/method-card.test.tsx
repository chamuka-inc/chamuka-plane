import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { MethodCard } from "@/components/docs/method-card";

describe("MethodCard", () => {
  it("renders method name as heading", () => {
    render(
      <MethodCard
        name="Sandbox.create"
        signature="static create(request: CreateSandboxRequest): Promise<Sandbox>"
        description="Create a new sandbox."
      >
        <p>children content</p>
      </MethodCard>,
    );
    const heading = screen.getByRole("heading", { name: "Sandbox.create" });
    expect(heading).toBeTruthy();
  });

  it("renders signature in monospace", () => {
    render(
      <MethodCard
        name="Sandbox.create"
        signature="static create(request: CreateSandboxRequest): Promise<Sandbox>"
        description="Create a new sandbox."
      />,
    );
    const sig = screen.getByText(/static create/);
    expect(sig.className).toContain("font-mono");
  });

  it("renders description", () => {
    render(
      <MethodCard
        name="Sandbox.create"
        signature="static create(): void"
        description="Create a new sandbox."
      />,
    );
    expect(screen.getByText("Create a new sandbox.")).toBeTruthy();
  });

  it("renders children", () => {
    render(
      <MethodCard
        name="Sandbox.create"
        signature="static create(): void"
        description="Create."
      >
        <p>param table here</p>
      </MethodCard>,
    );
    expect(screen.getByText("param table here")).toBeTruthy();
  });

  it("generates anchor id from method name", () => {
    render(
      <MethodCard
        name="Sandbox.create"
        signature="static create(): void"
        description="Create."
      />,
    );
    const heading = screen.getByRole("heading", { name: "Sandbox.create" });
    expect(heading.id).toBe("sandbox-create");
  });
});
