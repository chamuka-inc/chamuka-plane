import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { StatCard } from "@/components/stat-card";

describe("StatCard", () => {
  it("renders label, value, and sub", () => {
    render(<StatCard label="Running Sandboxes" value="12" sub="18 total" />);
    expect(screen.getByText("Running Sandboxes")).toBeDefined();
    expect(screen.getByText("12")).toBeDefined();
    expect(screen.getByText("18 total")).toBeDefined();
  });

  it("renders with suffix", () => {
    render(<StatCard label="CPU Today" value="4.2" suffix="h" />);
    expect(screen.getByText("4.2")).toBeDefined();
    expect(screen.getByText("h")).toBeDefined();
  });
});
