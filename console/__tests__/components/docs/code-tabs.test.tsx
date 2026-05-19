import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { CodeTabs } from "@/components/docs/code-tabs";

describe("CodeTabs", () => {
  beforeEach(() => {
    localStorage.removeItem("console-snippet-lang");
  });

  it("renders TypeScript tab content by default", () => {
    render(
      <CodeTabs
        ts='console.log("hello");'
        python='print("hello")'
      />,
    );
    expect(screen.getByRole("code").textContent).toContain("console.log");
  });

  it("switches to Python when Python tab clicked", async () => {
    const user = userEvent.setup();
    render(
      <CodeTabs
        ts='console.log("hello");'
        python='print("hello")'
      />,
    );
    await user.click(screen.getByText("Python"));
    expect(screen.getByRole("code").textContent).toContain("print");
  });

  it("copies active code to clipboard", async () => {
    const user = userEvent.setup();
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("navigator", { ...navigator, clipboard: { writeText } });

    render(
      <CodeTabs
        ts='console.log("hello");'
        python='print("hello")'
      />,
    );
    await user.click(screen.getByLabelText("Copy code"));
    expect(writeText).toHaveBeenCalledWith('console.log("hello");');
  });
});
