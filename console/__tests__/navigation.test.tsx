import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { Topbar } from "@/components/topbar";

describe("Topbar navigation", () => {
  it("renders Sandbox brand link to /", () => {
    render(<Topbar />);
    const link = screen.getByRole("link", { name: /sandbox/i });
    expect(link.getAttribute("href")).toBe("/");
  });

  it("does not render a separate Quickstart nav item", () => {
    render(<Topbar />);
    expect(screen.queryByText("Quickstart")).toBeNull();
  });
});
