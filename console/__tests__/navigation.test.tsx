import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { Topbar } from "@/components/topbar";

const mockUsePathname = vi.fn();

vi.mock("next/navigation", () => ({
  usePathname: () => mockUsePathname(),
}));

describe("Topbar navigation", () => {
  beforeEach(() => {
    mockUsePathname.mockReturnValue("/quickstart");
  });

  it("shows Quickstart nav item", () => {
    render(<Topbar />);
    expect(screen.getAllByText("Quickstart").length).toBeGreaterThan(0);
    expect(screen.queryByText("Playground")).toBeNull();
  });

  it("links Quickstart to /quickstart", () => {
    render(<Topbar />);
    const links = screen.getAllByRole("link", { name: "Quickstart" });
    expect(links[0].getAttribute("href")).toBe("/quickstart");
  });

  it("marks Quickstart active on /quickstart", () => {
    mockUsePathname.mockReturnValue("/quickstart");
    render(<Topbar />);
    const link = screen.getAllByRole("link", { name: "Quickstart" })[0];
    expect(link.getAttribute("aria-current")).toBe("page");
  });

  it("marks Quickstart active on sandbox detail routes", () => {
    mockUsePathname.mockReturnValue("/sandboxes/sb-abc");
    render(<Topbar />);
    const link = screen.getAllByRole("link", { name: "Quickstart" })[0];
    expect(link.getAttribute("aria-current")).toBe("page");
  });
});
