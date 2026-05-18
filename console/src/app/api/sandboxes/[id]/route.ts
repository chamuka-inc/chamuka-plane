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
