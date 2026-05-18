import { NextRequest, NextResponse } from "next/server";
import { getSandbox } from "@/lib/sandbox-registry";

export async function POST(
  request: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  const entry = await getSandbox(id);

  if (!entry?.connection?.url) {
    return NextResponse.json(
      { error: { code: "NOT_FOUND", message: `Sandbox ${id} not found or no connection`, request_id: "" } },
      { status: 404 },
    );
  }

  const proxyUrl = entry.connection.url.replace(/\/$/, "");

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
