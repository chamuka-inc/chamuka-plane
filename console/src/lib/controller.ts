import { NextResponse } from "next/server";
import { getConnectionUrl, setConnectionUrl } from "./connection-cache";

export const CONTROLLER_URL =
  process.env.CONTROLLER_URL ?? "http://localhost:8080";

/** Optional local registry for runtime images; environment-specific. */
export function runtimeImageRegistry(): string {
  return process.env.RUNTIME_IMAGE_REGISTRY?.trim() ?? "";
}

/**
 * Prefix sandbox-runtime images with the local registry when drones cannot
 * resolve bare tags (Vagrant fleet pulls from host registry, not Docker Hub).
 */
export function resolveRuntimeImage(image: string): string {
  const registry = runtimeImageRegistry();
  if (!registry || !image.startsWith("sandbox-runtime:")) {
    return image;
  }
  // Already registry-qualified (host:port/..., docker.io/..., ghcr.io/...).
  if (image.includes("/")) {
    const host = image.slice(0, image.indexOf("/"));
    if (host.includes(".") || host.includes(":") || host === "localhost") {
      return image;
    }
  }
  return `${registry}/${image}`;
}

export const authHeaders = {
  "x-auth-tenant": process.env.AUTH_TENANT ?? "tenant-default",
  "x-auth-user": process.env.AUTH_USER ?? "admin",
};

export function bffError(
  code: string,
  message: string,
  status: number,
): NextResponse {
  return NextResponse.json(
    { error: { code, message, request_id: "" } },
    { status },
  );
}

export function cacheConnectionFromResponse(data: {
  sandbox?: { id?: string };
  connection?: { url?: string };
}): void {
  if (data.sandbox?.id && data.connection?.url) {
    setConnectionUrl(data.sandbox.id, data.connection.url);
  }
}

export async function resolveConnectionUrl(
  sandboxId: string,
): Promise<string | undefined> {
  const cached = getConnectionUrl(sandboxId);
  if (cached) return cached;

  const res = await fetch(
    `${CONTROLLER_URL}/pub/v1/sandboxes/${encodeURIComponent(sandboxId)}`,
    { headers: authHeaders },
  );
  if (!res.ok) return undefined;

  const data = await res.json().catch(() => null);
  if (data?.connection?.url) {
    setConnectionUrl(sandboxId, data.connection.url as string);
    return data.connection.url as string;
  }
  return undefined;
}
