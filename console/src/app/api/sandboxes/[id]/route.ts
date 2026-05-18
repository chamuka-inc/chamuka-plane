import { NextRequest, NextResponse } from "next/server";

const CONTROLLER_URL = process.env.CONTROLLER_URL ?? "http://localhost:8080";
const AUTH_TENANT = process.env.AUTH_TENANT ?? "tenant-default";
const AUTH_USER = process.env.AUTH_USER ?? "admin";

const authHeaders = {
  "x-auth-tenant": AUTH_TENANT,
  "x-auth-user": AUTH_USER,
};

export async function GET(
  _request: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;

  const res = await fetch(
    `${CONTROLLER_URL}/pub/v1/sandboxes/${encodeURIComponent(id)}`,
    { headers: authHeaders },
  );

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
