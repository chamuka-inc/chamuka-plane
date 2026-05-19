import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { Callout } from "@/components/docs/callout";

describe("Callout", () => {
  it("renders children text", () => {
    render(<Callout type="info">Auth is resolved automatically.</Callout>);
    expect(screen.getByText("Auth is resolved automatically.")).toBeTruthy();
  });

  it("applies info styling by default", () => {
    const { container } = render(<Callout type="info">Info text</Callout>);
    const el = container.firstElementChild!;
    expect(el.className).toContain("border-accent-blue");
  });

  it("applies warning styling", () => {
    const { container } = render(<Callout type="warning">Warn text</Callout>);
    const el = container.firstElementChild!;
    expect(el.className).toContain("border-status-amber");
  });

  it("applies tip styling", () => {
    const { container } = render(<Callout type="tip">Tip text</Callout>);
    const el = container.firstElementChild!;
    expect(el.className).toContain("border-status-green");
  });
});
