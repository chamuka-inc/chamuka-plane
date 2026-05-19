import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { Topbar } from "@/components/topbar";

describe("Topbar navigation", () => {
  it("renders Sandbox brand link to /", () => {
    render(<Topbar />);
    const link = screen.getByRole("link", { name: /sandbox/i });
    expect(link.getAttribute("href")).toBe("/");
  });

  it("renders Docs link to /docs/quickstart", () => {
    render(<Topbar />);
    const link = screen.getByRole("link", { name: /docs/i });
    expect(link.getAttribute("href")).toBe("/docs/quickstart");
  });
});
