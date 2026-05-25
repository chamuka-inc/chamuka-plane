import { NextResponse } from "next/server";
import { getConnectionUrl, setConnectionUrl } from "./connection-cache";

export const CONTROLLER_URL =
  process.env.CONTROLLER_URL ?? "http://localhost:8080";

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
