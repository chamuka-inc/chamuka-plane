import { NextRequest, NextResponse } from "next/server";
import {
  CONTROLLER_URL,
  authHeaders,
  bffError,
  cacheConnectionFromResponse,
  resolveRuntimeImage,
} from "@/lib/controller";

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
    return bffError("BAD_REQUEST", "Invalid JSON", 400);
  }

  let payload = body;
  if (
    payload &&
    typeof payload === "object" &&
    "image" in payload &&
    typeof (payload as { image?: unknown }).image === "string"
  ) {
    const req = payload as { image: string };
    payload = { ...req, image: resolveRuntimeImage(req.image) };
  }

  const res = await fetch(`${CONTROLLER_URL}/pub/v1/sandboxes`, {
    method: "POST",
    headers: { ...authHeaders, "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });

  let data;
  try {
    data = await res.json();
  } catch {
    return bffError("UPSTREAM_ERROR", `Controller returned ${res.status}`, 502);
  }

  if (res.ok) cacheConnectionFromResponse(data);

  return NextResponse.json(data, { status: res.status });
}
