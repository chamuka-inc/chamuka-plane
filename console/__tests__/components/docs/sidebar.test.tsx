import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { DocsSidebar } from "@/components/docs/sidebar";

describe("DocsSidebar", () => {
  it("renders all section titles", () => {
    render(<DocsSidebar />);
    expect(screen.getByText("Getting Started")).toBeTruthy();
    expect(screen.getByText("Reference")).toBeTruthy();
    expect(screen.getByText("Guides")).toBeTruthy();
  });

  it("renders all nav links", () => {
    render(<DocsSidebar />);
    const links = screen.getAllByRole("link");
    expect(links.length).toBe(6);
    expect(links[0].getAttribute("href")).toBe("/docs/quickstart");
  });

  it("highlights active link based on pathname prop", () => {
    render(<DocsSidebar pathname="/docs/sdk-reference" />);
    const activeLink = screen.getByText("SDK Reference").closest("a");
    expect(activeLink?.className).toContain("text-text");
    expect(activeLink?.className).toContain("border-accent-blue");
  });
});
