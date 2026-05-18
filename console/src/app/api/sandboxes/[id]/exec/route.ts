import { NextRequest, NextResponse } from "next/server";
import { getConnectionUrl } from "@/lib/connection-cache";

export async function POST(
  request: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  const proxyUrl = getConnectionUrl(id)?.replace(/\/$/, "");

  if (!proxyUrl) {
    return NextResponse.json(
      { error: { code: "NO_CONNECTION", message: `No connection URL cached for sandbox ${id}. Re-create the sandbox to establish a connection.`, request_id: "" } },
      { status: 404 },
    );
  }

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
