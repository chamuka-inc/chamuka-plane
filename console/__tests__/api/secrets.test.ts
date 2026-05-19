import { describe, it, expect, vi, beforeEach } from "vitest";
import { createBffClient } from "@/api/client";

describe("BFF secret routes", () => {
  beforeEach(() => { vi.restoreAllMocks(); });

  it("createSecret calls /api/secrets", async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ id: "secret-abc", created_at: "2026-05-19T00:00:00Z" }),
    });

    const client = createBffClient();
    const result = await client.createSecret({ value: "sk-test", name: "my-key" });

    expect(result.id).toBe("secret-abc");
    expect(global.fetch).toHaveBeenCalledWith(
      "/api/secrets",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ value: "sk-test", name: "my-key" }),
      }),
    );
  });

  it("createCredentialBinding calls /api/sandboxes/:id/credentials", async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ id: "cred-xyz" }),
    });

    const client = createBffClient();
    const result = await client.createCredentialBinding("sb-123", {
      secret_id: "secret-abc",
      destination: "api.openai.com",
      header_name: "Authorization",
      header_format: "Bearer {value}",
    });

    expect(result.id).toBe("cred-xyz");
    expect(global.fetch).toHaveBeenCalledWith(
      "/api/sandboxes/sb-123/credentials",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({
          secret_id: "secret-abc",
          destination: "api.openai.com",
          header_name: "Authorization",
          header_format: "Bearer {value}",
        }),
      }),
    );
  });

  it("createSecret surfaces errors as BffError", async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
      statusText: "Internal Server Error",
      json: () => Promise.resolve({ error: { code: "INTERNAL", message: "KMS unavailable" } }),
    });

    const client = createBffClient();
    await expect(client.createSecret({ value: "sk-test" }))
      .rejects.toThrow("INTERNAL");
  });
});
