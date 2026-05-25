import { afterEach, describe, expect, it } from "vitest";
import { resolveRuntimeImage } from "@/lib/controller";

describe("resolveRuntimeImage", () => {
  const originalRegistry = process.env.RUNTIME_IMAGE_REGISTRY;

  afterEach(() => {
    if (originalRegistry === undefined) {
      delete process.env.RUNTIME_IMAGE_REGISTRY;
    } else {
      process.env.RUNTIME_IMAGE_REGISTRY = originalRegistry;
    }
  });

  it("leaves sandbox-runtime tags unchanged when no local registry is configured", () => {
    delete process.env.RUNTIME_IMAGE_REGISTRY;

    expect(resolveRuntimeImage("sandbox-runtime:node24-latest")).toBe(
      "sandbox-runtime:node24-latest",
    );
  });

  it("prefixes sandbox-runtime tags when a local registry is configured", () => {
    process.env.RUNTIME_IMAGE_REGISTRY = "192.168.56.1:5000";

    expect(resolveRuntimeImage("sandbox-runtime:node24-latest")).toBe(
      "192.168.56.1:5000/sandbox-runtime:node24-latest",
    );
  });

  it("leaves already-qualified images unchanged", () => {
    process.env.RUNTIME_IMAGE_REGISTRY = "192.168.56.1:5000";

    expect(
      resolveRuntimeImage("192.168.56.1:5000/sandbox-runtime:node24-latest"),
    ).toBe("192.168.56.1:5000/sandbox-runtime:node24-latest");
  });

  it("leaves non-runtime images unchanged", () => {
    process.env.RUNTIME_IMAGE_REGISTRY = "192.168.56.1:5000";

    expect(resolveRuntimeImage("node:24-slim")).toBe("node:24-slim");
  });
});
