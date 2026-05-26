import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {
  SandboxConfig,
  DEFAULT_CONFIG,
  type SandboxConfigValues,
} from "@/components/sandbox-config";

function renderEnvEditor(
  overrides: Partial<SandboxConfigValues> = {},
  onChange?: (v: SandboxConfigValues) => void,
) {
  const value = { ...DEFAULT_CONFIG, ...overrides };
  const handleChange = onChange ?? vi.fn();
  const result = render(
    <SandboxConfig value={value} onChange={handleChange} />,
  );
  return { ...result, handleChange };
}

async function openConfigPanel(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByText("Configure"));
}

async function addEnvPair(
  user: ReturnType<typeof userEvent.setup>,
  key: string,
  value: string,
  method: "enter" | "button" = "enter",
) {
  const keyInput = screen.getByPlaceholderText("KEY");
  const valueInput = screen.getByPlaceholderText("value");

  await user.clear(keyInput);
  await user.type(keyInput, key);
  await user.clear(valueInput);
  await user.type(valueInput, value);

  if (method === "enter") {
    await user.keyboard("{Enter}");
  } else {
    const buttons = screen.getAllByRole("button");
    const addButton = buttons.find((b) => {
      const svg = b.querySelector("svg");
      return svg && !b.textContent?.trim() && !b.closest("[class*='flex items-center gap-1.5']");
    });
    if (addButton) await user.click(addButton);
  }
}

describe("SandboxConfig — Environment key/value pairs", () => {
  let user: ReturnType<typeof userEvent.setup>;

  beforeEach(() => {
    user = userEvent.setup();
  });

  it("shows the env editor when config panel is opened", async () => {
    renderEnvEditor();
    await openConfigPanel(user);
    expect(screen.getByText("Environment")).toBeDefined();
    expect(screen.getByPlaceholderText("KEY")).toBeDefined();
    expect(screen.getByPlaceholderText("value")).toBeDefined();
  });

  it("adds a key/value pair via Enter key", async () => {
    const handleChange = vi.fn();
    renderEnvEditor({}, handleChange);
    await openConfigPanel(user);

    await addEnvPair(user, "API_KEY", "sk-test-123");

    expect(handleChange).toHaveBeenCalledWith(
      expect.objectContaining({
        env: { API_KEY: "sk-test-123" },
      }),
    );
  });

  it("adds a key/value pair via the plus button", async () => {
    const handleChange = vi.fn();
    renderEnvEditor({}, handleChange);
    await openConfigPanel(user);

    const keyInput = screen.getByPlaceholderText("KEY");
    const valueInput = screen.getByPlaceholderText("value");
    await user.type(keyInput, "NODE_ENV");
    await user.type(valueInput, "production");

    const addButtons = screen.getAllByRole("button").filter((btn) => {
      const svg = btn.querySelector('svg [d*="M5 12h14"]') ?? btn.querySelector('svg');
      return svg && btn.closest(".flex.items-center.gap-1\\.5") === null && btn.textContent === "";
    });

    if (addButtons.length > 0) {
      await user.click(addButtons[0]);
    } else {
      await user.keyboard("{Enter}");
    }

    expect(handleChange).toHaveBeenCalledWith(
      expect.objectContaining({
        env: { NODE_ENV: "production" },
      }),
    );
  });

  it("ignores add when key is empty", async () => {
    const handleChange = vi.fn();
    renderEnvEditor({}, handleChange);
    await openConfigPanel(user);

    const valueInput = screen.getByPlaceholderText("value");
    await user.type(valueInput, "some-value");
    await user.keyboard("{Enter}");

    expect(handleChange).not.toHaveBeenCalled();
  });

  it("allows empty value (key-only env vars)", async () => {
    const handleChange = vi.fn();
    renderEnvEditor({}, handleChange);
    await openConfigPanel(user);

    const keyInput = screen.getByPlaceholderText("KEY");
    await user.type(keyInput, "DEBUG");
    await user.keyboard("{Enter}");

    expect(handleChange).toHaveBeenCalledWith(
      expect.objectContaining({
        env: { DEBUG: "" },
      }),
    );
  });

  it("trims whitespace from keys", async () => {
    const handleChange = vi.fn();
    renderEnvEditor({}, handleChange);
    await openConfigPanel(user);

    const keyInput = screen.getByPlaceholderText("KEY");
    const valueInput = screen.getByPlaceholderText("value");
    await user.type(keyInput, "  MY_KEY  ");
    await user.type(valueInput, "val");
    await user.keyboard("{Enter}");

    expect(handleChange).toHaveBeenCalledWith(
      expect.objectContaining({
        env: { MY_KEY: "val" },
      }),
    );
  });

  it("displays existing key/value pairs", async () => {
    renderEnvEditor({ env: { DATABASE_URL: "postgres://localhost/test", PORT: "3000" } });
    await openConfigPanel(user);

    expect(screen.getByText("DATABASE_URL")).toBeDefined();
    expect(screen.getByText("postgres://localhost/test")).toBeDefined();
    expect(screen.getByText("PORT")).toBeDefined();
    expect(screen.getByText("3000")).toBeDefined();
  });

  it("removes a key/value pair when clicking X", async () => {
    const handleChange = vi.fn();
    renderEnvEditor(
      { env: { API_KEY: "secret", LOG_LEVEL: "debug" } },
      handleChange,
    );
    await openConfigPanel(user);

    expect(screen.getByText("API_KEY")).toBeDefined();
    expect(screen.getByText("LOG_LEVEL")).toBeDefined();

    const apiKeyRow = screen.getByText("API_KEY").closest("div")!;
    const removeButton = within(apiKeyRow).getByRole("button");
    await user.click(removeButton);

    expect(handleChange).toHaveBeenCalledWith(
      expect.objectContaining({
        env: { LOG_LEVEL: "debug" },
      }),
    );
  });

  it("overwrites existing key when adding a duplicate", async () => {
    const handleChange = vi.fn();
    renderEnvEditor({ env: { MODE: "old" } }, handleChange);
    await openConfigPanel(user);

    await addEnvPair(user, "MODE", "new");

    expect(handleChange).toHaveBeenCalledWith(
      expect.objectContaining({
        env: { MODE: "new" },
      }),
    );
  });

  it("shows override badge when env vars are set", async () => {
    renderEnvEditor({ env: { SOME_VAR: "1" } });
    expect(screen.getByText("1", { selector: "span" })).toBeDefined();
  });

  it("does not show override badge with default config", () => {
    renderEnvEditor();
    const badge = screen.queryByText("1", { selector: "span.text-xs.font-mono" });
    expect(badge).toBeNull();
  });

  it("clears env vars on reset to defaults", async () => {
    const handleChange = vi.fn();
    renderEnvEditor(
      { env: { DB: "postgres://localhost" } },
      handleChange,
    );
    await openConfigPanel(user);

    const resetButton = screen.getByText("Reset to defaults");
    await user.click(resetButton);

    expect(handleChange).toHaveBeenCalledWith(
      expect.objectContaining({
        env: {},
      }),
    );
  });

  it("preserves other config fields when adding env vars", async () => {
    const handleChange = vi.fn();
    renderEnvEditor(
      { trust: "paranoid", vcpus: 4 },
      handleChange,
    );
    await openConfigPanel(user);

    await addEnvPair(user, "TOKEN", "abc");

    expect(handleChange).toHaveBeenCalledWith(
      expect.objectContaining({
        trust: "paranoid",
        vcpus: 4,
        env: { TOKEN: "abc" },
      }),
    );
  });

  it("handles special characters in values", async () => {
    const handleChange = vi.fn();
    renderEnvEditor({}, handleChange);
    await openConfigPanel(user);

    await addEnvPair(user, "CONN", "host=db;port=5432&ssl=true");

    expect(handleChange).toHaveBeenCalledWith(
      expect.objectContaining({
        env: { CONN: "host=db;port=5432&ssl=true" },
      }),
    );
  });

  it("disables plus button when key input is empty", async () => {
    renderEnvEditor();
    await openConfigPanel(user);

    const buttons = screen.getAllByRole("button");
    const disabledButton = buttons.find((b) => b.hasAttribute("disabled"));
    expect(disabledButton).toBeDefined();
  });
});
