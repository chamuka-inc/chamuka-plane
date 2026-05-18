import { NextRequest, NextResponse } from "next/server";
import { getConnectionUrl, setConnectionUrl } from "@/lib/connection-cache";

const CONTROLLER_URL = process.env.CONTROLLER_URL ?? "http://localhost:8080";
const AUTH_TENANT = process.env.AUTH_TENANT ?? "tenant-default";
const AUTH_USER = process.env.AUTH_USER ?? "admin";

async function resolveConnectionUrl(sandboxId: string): Promise<string | undefined> {
  const cached = getConnectionUrl(sandboxId);
  if (cached) return cached;

  const res = await fetch(
    `${CONTROLLER_URL}/pub/v1/sandboxes/${encodeURIComponent(sandboxId)}`,
    { headers: { "x-auth-tenant": AUTH_TENANT, "x-auth-user": AUTH_USER } },
  );
  if (!res.ok) return undefined;

  const data = await res.json().catch(() => null);
  if (data?.connection?.url) {
    setConnectionUrl(sandboxId, data.connection.url);
    return data.connection.url as string;
  }
  return undefined;
}

export async function POST(
  request: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  const connUrl = await resolveConnectionUrl(id);
  const proxyUrl = connUrl?.replace(/\/$/, "");

  if (!proxyUrl) {
    return NextResponse.json(
      { error: { code: "NO_CONNECTION", message: `No connection URL for sandbox ${id}`, request_id: "" } },
      { status: 404 },
    );
  }

  let body: unknown;
  try {
    body = await request.json();
  } catch {
    return NextResponse.json(
      { error: { code: "BAD_REQUEST", message: "Invalid JSON", request_id: "" } },
      { status: 400 },
    );
  }

  try {
    const res = await fetch(`${proxyUrl}/v1/exec`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });

    let data;
    try {
      data = await res.json();
    } catch {
      return NextResponse.json(
        { error: { code: "UPSTREAM_ERROR", message: `agentd returned ${res.status}`, request_id: "" } },
        { status: 502 },
      );
    }
    return NextResponse.json(data, { status: res.status });
  } catch (err) {
    const message = err instanceof Error ? err.message : "Unknown error";
    return NextResponse.json(
      { error: { code: "EXEC_FAILED", message, request_id: "" } },
      { status: 502 },
    );
  }
}
