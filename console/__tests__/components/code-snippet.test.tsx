import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { CodeSnippet } from "@/components/code-snippet";

describe("CodeSnippet", () => {
  it("renders code content", () => {
    render(
      <CodeSnippet
        code={'console.log("hi")'}
        language="typescript"
        onLanguageChange={() => {}}
      />,
    );
    expect(screen.getByRole("code").textContent).toContain('console.log("hi")');
  });

  it("switches language via tabs", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <CodeSnippet
        code="ts code"
        language="typescript"
        onLanguageChange={onChange}
      />,
    );
    await user.click(screen.getByText("Python"));
    expect(onChange).toHaveBeenCalledWith("python");
  });

  it("copies code to clipboard", async () => {
    const user = userEvent.setup();
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("navigator", { ...navigator, clipboard: { writeText } });

    render(
      <CodeSnippet
        code="copy me"
        language="typescript"
        onLanguageChange={() => {}}
      />,
    );
    await user.click(screen.getByLabelText("Copy code"));
    expect(writeText).toHaveBeenCalledWith("copy me");
  });
});
