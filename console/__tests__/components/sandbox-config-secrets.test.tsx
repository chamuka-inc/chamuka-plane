import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {
  SandboxConfig,
  DEFAULT_CONFIG,
  type SandboxConfigValues,
} from "@/components/sandbox-config";

function renderConfig(
  overrides: Partial<SandboxConfigValues> = {},
  onChange?: (v: SandboxConfigValues) => void,
) {
  const value = { ...DEFAULT_CONFIG, ...overrides };
  const handleChange = onChange ?? vi.fn();
  return { ...render(<SandboxConfig value={value} onChange={handleChange} />), handleChange };
}

async function openPanel(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByText("Configure"));
}

describe("SandboxConfig — Secrets", () => {
  let user: ReturnType<typeof userEvent.setup>;
  beforeEach(() => { user = userEvent.setup(); });

  it("shows secrets section when config panel is opened", async () => {
    renderConfig();
    await openPanel(user);
    expect(screen.getByText("Secrets")).toBeDefined();
    expect(screen.getByText(/injected by proxy/i)).toBeDefined();
  });

  it("adds a secret entry via Enter key", async () => {
    const handleChange = vi.fn();
    renderConfig({}, handleChange);
    await openPanel(user);

    await user.type(screen.getByPlaceholderText("api.example.com"), "api.openai.com");
    await user.type(screen.getByPlaceholderText("Authorization"), "Authorization");
    await user.type(screen.getByPlaceholderText("secret value"), "Bearer sk-test-123");
    await user.keyboard("{Enter}");

    expect(handleChange).toHaveBeenCalledWith(
      expect.objectContaining({
        secrets: [
          {
            destination: "api.openai.com",
            header_name: "Authorization",
            header_format: "Bearer {value}",
            value: "sk-test-123",
          },
        ],
      }),
    );
  });

  it("masks the secret value after entry", async () => {
    renderConfig({
      secrets: [
        { destination: "api.openai.com", header_name: "Authorization", header_format: "{value}", value: "sk-secret" },
      ],
    });
    await openPanel(user);

    expect(screen.getByText("api.openai.com")).toBeDefined();
    expect(screen.queryByText("sk-secret")).toBeNull();
    expect(screen.getByText(/\*{6,}/)).toBeDefined();
  });

  it("removes a secret entry when clicking X", async () => {
    const handleChange = vi.fn();
    renderConfig({
      secrets: [
        { destination: "api.openai.com", header_name: "Authorization", header_format: "{value}", value: "sk-1" },
        { destination: "api.stripe.com", header_name: "Authorization", header_format: "{value}", value: "sk-2" },
      ],
    }, handleChange);
    await openPanel(user);

    const openaiRow = screen.getByText("api.openai.com").closest("div")!;
    await user.click(within(openaiRow).getByRole("button"));

    expect(handleChange).toHaveBeenCalledWith(
      expect.objectContaining({
        secrets: [
          { destination: "api.stripe.com", header_name: "Authorization", header_format: "{value}", value: "sk-2" },
        ],
      }),
    );
  });

  it("prevents add when destination is empty", async () => {
    const handleChange = vi.fn();
    renderConfig({}, handleChange);
    await openPanel(user);

    await user.type(screen.getByPlaceholderText("secret value"), "some-secret");
    await user.keyboard("{Enter}");

    expect(handleChange).not.toHaveBeenCalled();
  });

  it("prevents add when value is empty", async () => {
    const handleChange = vi.fn();
    renderConfig({}, handleChange);
    await openPanel(user);

    await user.type(screen.getByPlaceholderText("api.example.com"), "api.example.com");
    await user.keyboard("{Enter}");

    expect(handleChange).not.toHaveBeenCalled();
  });

  it("auto-detects Bearer prefix in header format", async () => {
    const handleChange = vi.fn();
    renderConfig({}, handleChange);
    await openPanel(user);

    await user.type(screen.getByPlaceholderText("api.example.com"), "api.openai.com");
    await user.type(screen.getByPlaceholderText("Authorization"), "Authorization");
    await user.type(screen.getByPlaceholderText("secret value"), "Bearer sk-my-key");
    await user.keyboard("{Enter}");

    const call = handleChange.mock.calls[0][0];
    expect(call.secrets[0].header_format).toBe("Bearer {value}");
    expect(call.secrets[0].value).toBe("sk-my-key");
  });

  it("uses plain {value} format when no Bearer prefix", async () => {
    const handleChange = vi.fn();
    renderConfig({}, handleChange);
    await openPanel(user);

    await user.type(screen.getByPlaceholderText("api.example.com"), "api.stripe.com");
    await user.type(screen.getByPlaceholderText("Authorization"), "X-Api-Key");
    await user.type(screen.getByPlaceholderText("secret value"), "raw-key-123");
    await user.keyboard("{Enter}");

    const call = handleChange.mock.calls[0][0];
    expect(call.secrets[0].header_format).toBe("{value}");
    expect(call.secrets[0].value).toBe("raw-key-123");
  });

  it("counts secrets as an override", () => {
    renderConfig({
      secrets: [
        { destination: "api.openai.com", header_name: "Authorization", header_format: "{value}", value: "sk-1" },
      ],
    });
    const badge = screen.getByText("1");
    expect(badge).toBeDefined();
  });

  it("clears secrets on reset to defaults", async () => {
    const handleChange = vi.fn();
    renderConfig({
      secrets: [
        { destination: "api.openai.com", header_name: "Authorization", header_format: "{value}", value: "sk-1" },
      ],
    }, handleChange);
    await openPanel(user);

    await user.click(screen.getByText("Reset to defaults"));

    expect(handleChange).toHaveBeenCalledWith(
      expect.objectContaining({ secrets: [] }),
    );
  });
});
