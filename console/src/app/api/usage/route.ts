import { NextRequest, NextResponse } from "next/server";

const CONTROLLER_URL = process.env.CONTROLLER_URL ?? "http://localhost:8080";
const AUTH_TENANT = process.env.AUTH_TENANT ?? "tenant-default";
const AUTH_USER = process.env.AUTH_USER ?? "admin";

export async function GET(request: NextRequest) {
  const { searchParams } = request.nextUrl;
  const period = searchParams.get("period") ?? "24h";
  const sandboxId = searchParams.get("sandbox_id");

  const params = new URLSearchParams({ period });
  if (sandboxId) params.set("sandbox_id", sandboxId);

  const res = await fetch(`${CONTROLLER_URL}/pub/v1/usage?${params}`, {
    headers: {
      "X-Auth-Tenant": AUTH_TENANT,
      "X-Auth-User": AUTH_USER,
    },
  });

  let body;
  try {
    body = await res.json();
  } catch {
    return NextResponse.json(
      { error: { code: "UPSTREAM_ERROR", message: `Controller returned ${res.status}`, request_id: "" } },
      { status: 502 },
    );
  }
  return NextResponse.json(body, { status: res.status });
}
