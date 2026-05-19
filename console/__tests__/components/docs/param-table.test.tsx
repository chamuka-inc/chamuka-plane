import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { ParamTable } from "@/components/docs/param-table";

describe("ParamTable", () => {
  const params = [
    { name: "request", type: "CreateSandboxRequest", required: true, description: "Sandbox config" },
    { name: "auth", type: "AuthOptions", required: false, description: "Override default auth" },
  ];

  it("renders all parameter names", () => {
    render(<ParamTable params={params} />);
    expect(screen.getByText("request")).toBeTruthy();
    expect(screen.getByText("auth")).toBeTruthy();
  });

  it("renders type in monospace", () => {
    render(<ParamTable params={params} />);
    const typeCell = screen.getByText("CreateSandboxRequest");
    expect(typeCell.tagName).toBe("CODE");
  });

  it("shows required badge for required params", () => {
    render(<ParamTable params={params} />);
    expect(screen.getByText("required")).toBeTruthy();
  });

  it("renders descriptions", () => {
    render(<ParamTable params={params} />);
    expect(screen.getByText("Sandbox config")).toBeTruthy();
    expect(screen.getByText("Override default auth")).toBeTruthy();
  });
});
