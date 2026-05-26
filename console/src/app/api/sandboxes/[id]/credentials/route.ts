import { NextRequest, NextResponse } from "next/server";
import { CONTROLLER_URL, authHeaders, bffError } from "@/lib/controller";

export async function POST(
  request: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;

  let body: unknown;
  try {
    body = await request.json();
  } catch {
    return bffError("BAD_REQUEST", "Invalid JSON", 400);
  }

  const res = await fetch(
    `${CONTROLLER_URL}/pub/v1/sandboxes/${encodeURIComponent(id)}/credentials`,
    {
      method: "POST",
      headers: { ...authHeaders, "Content-Type": "application/json" },
      body: JSON.stringify(body),
    },
  );

  let data;
  try {
    data = await res.json();
  } catch {
    return bffError("UPSTREAM_ERROR", `Controller returned ${res.status}`, 502);
  }

  return NextResponse.json(data, { status: res.status });
}
