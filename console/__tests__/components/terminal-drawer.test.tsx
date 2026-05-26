import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

vi.mock("@/components/terminal-view", () => ({
  TerminalView: ({ sandboxId }: { sandboxId: string }) => (
    <div data-testid="terminal-view">terminal:{sandboxId}</div>
  ),
}));

import { TerminalDrawer } from "@/components/terminal-drawer";

describe("TerminalDrawer", () => {
  it("renders collapsed by default with Terminal label", () => {
    render(<TerminalDrawer sandboxId="sb-test-1" />);
    expect(screen.getByText("Terminal")).toBeDefined();
    expect(screen.queryByTestId("terminal-view")).toBeNull();
  });

  it("expands on click to show TerminalView", async () => {
    const user = userEvent.setup();
    render(<TerminalDrawer sandboxId="sb-test-1" />);

    await user.click(screen.getByText("Terminal"));
    expect(screen.getByTestId("terminal-view")).toBeDefined();
  });

  it("collapses when clicking the collapse button", async () => {
    const user = userEvent.setup();
    render(<TerminalDrawer sandboxId="sb-test-1" />);

    await user.click(screen.getByText("Terminal"));
    expect(screen.getByTestId("terminal-view")).toBeDefined();

    await user.click(screen.getByRole("button", { name: /collapse/i }));
    expect(screen.queryByTestId("terminal-view")).toBeNull();
  });

  it("collapses when Escape is pressed", async () => {
    const user = userEvent.setup();
    render(<TerminalDrawer sandboxId="sb-test-1" />);

    await user.click(screen.getByText("Terminal"));
    expect(screen.getByTestId("terminal-view")).toBeDefined();

    await user.keyboard("{Escape}");
    expect(screen.queryByTestId("terminal-view")).toBeNull();
  });
});
