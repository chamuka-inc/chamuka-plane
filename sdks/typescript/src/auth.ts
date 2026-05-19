import { SandboxError } from "./errors.js";

export type SandboxAuth = {
  controlUrl?: string;
  token?: string;
  tenant?: string;
};

export type ResolvedAuth = {
  controlUrl: string;
  token?: string | undefined;
  tenant?: string | undefined;
};

export function resolveAuth(explicit?: SandboxAuth): ResolvedAuth {
  const controlUrl =
    explicit?.controlUrl ?? process.env.SANDBOX_CONTROL_URL;
  if (!controlUrl) {
    throw new SandboxError(
      "No control URL. Set SANDBOX_CONTROL_URL or pass { controlUrl } explicitly.",
    );
  }
  return {
    controlUrl,
    token: explicit?.token ?? process.env.SANDBOX_TOKEN,
    tenant: explicit?.tenant ?? process.env.SANDBOX_TENANT,
  };
}
