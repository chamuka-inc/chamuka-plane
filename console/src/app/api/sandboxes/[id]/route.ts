import { NextRequest, NextResponse } from "next/server";
import {
  CONTROLLER_URL,
  authHeaders,
  bffError,
  cacheConnectionFromResponse,
} from "@/lib/controller";

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
    return bffError("UPSTREAM_ERROR", `Controller returned ${res.status}`, 502);
  }

  if (res.ok) cacheConnectionFromResponse(body);

  return NextResponse.json(body, { status: res.status });
}

export async function DELETE(
  request: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  const force = request.nextUrl.searchParams.get("force") === "true";
  const qs = force ? "?force=true" : "";

  const res = await fetch(
    `${CONTROLLER_URL}/pub/v1/sandboxes/${encodeURIComponent(id)}${qs}`,
    { method: "DELETE", headers: authHeaders },
  );

  if (res.status === 204) {
    return new NextResponse(null, { status: 204 });
  }

  let body;
  try {
    body = await res.json();
  } catch {
    return bffError("UPSTREAM_ERROR", `Controller returned ${res.status}`, 502);
  }

  return NextResponse.json(body, { status: res.status });
}
