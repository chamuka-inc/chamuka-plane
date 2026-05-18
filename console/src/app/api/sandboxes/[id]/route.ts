import { NextRequest, NextResponse } from "next/server";

const CONTROLLER_URL = process.env.CONTROLLER_URL ?? "http://localhost:8080";
const AUTH_TENANT = process.env.AUTH_TENANT ?? "tenant-default";
const AUTH_USER = process.env.AUTH_USER ?? "admin";

export async function GET(
  _request: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  const res = await fetch(`${CONTROLLER_URL}/pub/v1/sandboxes/${id}`, {
    headers: {
      "X-Auth-Tenant": AUTH_TENANT,
      "X-Auth-User": AUTH_USER,
    },
  });

  const body = await res.json();
  return NextResponse.json(body, { status: res.status });
}
