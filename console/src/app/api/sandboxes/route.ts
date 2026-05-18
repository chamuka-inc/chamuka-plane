import { NextRequest, NextResponse } from "next/server";

const CONTROLLER_URL = process.env.CONTROLLER_URL ?? "http://localhost:8080";
const AUTH_TENANT = process.env.AUTH_TENANT ?? "tenant-default";
const AUTH_USER = process.env.AUTH_USER ?? "admin";

export async function GET(request: NextRequest) {
  const { searchParams } = request.nextUrl;
  const limit = searchParams.get("limit") ?? "100";
  const status = searchParams.get("status");
  const cursor = searchParams.get("cursor");

  const params = new URLSearchParams({ limit });
  if (status) params.set("status", status);
  if (cursor) params.set("cursor", cursor);

  const res = await fetch(`${CONTROLLER_URL}/pub/v1/sandboxes?${params}`, {
    headers: {
      "X-Auth-Tenant": AUTH_TENANT,
      "X-Auth-User": AUTH_USER,
    },
  });

  const body = await res.json();
  return NextResponse.json(body, { status: res.status });
}
