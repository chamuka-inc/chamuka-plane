import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SdkSnippet } from "@/components/sdk-snippet";
import { DEFAULT_CONFIG, type SandboxConfigValues } from "@/components/sandbox-config";

const baseProps = {
  runtime: "node24",
  image: "sandbox-runtime:node24-latest",
  sandboxId: "sb-test",
};

function configWith(overrides: Partial<SandboxConfigValues>): SandboxConfigValues {
  return { ...DEFAULT_CONFIG, ...overrides };
}

describe("SdkSnippet — secrets", () => {
  it("generates no credential code when secrets is empty", () => {
    render(<SdkSnippet {...baseProps} config={configWith({ secrets: [] })} />);
    const code = screen.getByRole("code")?.textContent ?? "";
    expect(code).not.toContain("createSecret");
    expect(code).not.toContain("createCredential");
  });

  it("generates TypeScript credential binding code", () => {
    render(
      <SdkSnippet
        {...baseProps}
        config={configWith({
          secrets: [
            {
              destination: "api.openai.com",
              header_name: "Authorization",
              header_format: "Bearer {value}",
              value: "sk-test",
            },
          ],
        })}
      />,
    );
    const code = screen.getByRole("code")?.textContent ?? "";
    expect(code).toContain("createSecret");
    expect(code).toContain("createCredential");
    expect(code).toContain("api.openai.com");
    expect(code).toContain("process.env.");
    expect(code).not.toContain("sk-test");
  });

  it("generates Python credential binding code", async () => {
    const user = userEvent.setup();
    const { unmount } = render(
      <SdkSnippet
        {...baseProps}
        config={configWith({
          secrets: [
            {
              destination: "api.openai.com",
              header_name: "Authorization",
              header_format: "Bearer {value}",
              value: "sk-test",
            },
          ],
        })}
      />,
    );

    await user.click(screen.getByText("Python"));

    const code = screen.getByRole("code")?.textContent ?? "";
    expect(code).toContain("create_secret");
    expect(code).toContain("create_credential");
    expect(code).toContain("api.openai.com");
    expect(code).toContain("os.environ");
    expect(code).not.toContain("sk-test");

    unmount();
  });
});
