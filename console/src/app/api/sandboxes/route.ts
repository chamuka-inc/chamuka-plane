import { NextRequest, NextResponse } from "next/server";
import { setConnectionUrl } from "@/lib/connection-cache";

const CONTROLLER_URL = process.env.CONTROLLER_URL ?? "http://localhost:8080";
const AUTH_TENANT = process.env.AUTH_TENANT ?? "tenant-default";
const AUTH_USER = process.env.AUTH_USER ?? "admin";

const authHeaders = {
  "x-auth-tenant": AUTH_TENANT,
  "x-auth-user": AUTH_USER,
};

export async function GET(request: NextRequest) {
  const { searchParams } = request.nextUrl;
  const qs = searchParams.toString();
  const url = `${CONTROLLER_URL}/pub/v1/sandboxes${qs ? `?${qs}` : ""}`;

  const res = await fetch(url, { headers: authHeaders });
  const body = await res.json().catch(() => ({ items: [] }));
  return NextResponse.json(body, { status: res.status });
}

export async function POST(request: NextRequest) {
  let body: unknown;
  try {
    body = await request.json();
  } catch {
    return NextResponse.json(
      { error: { code: "BAD_REQUEST", message: "Invalid JSON", request_id: "" } },
      { status: 400 },
    );
  }

  const res = await fetch(`${CONTROLLER_URL}/pub/v1/sandboxes`, {
    method: "POST",
    headers: { ...authHeaders, "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });

  let data;
  try {
    data = await res.json();
  } catch {
    return NextResponse.json(
      { error: { code: "UPSTREAM_ERROR", message: `Controller returned ${res.status}`, request_id: "" } },
      { status: 502 },
    );
  }

  if (res.ok && data.sandbox?.id && data.connection?.url) {
    setConnectionUrl(data.sandbox.id, data.connection.url);
  }

  return NextResponse.json(data, { status: res.status });
}
