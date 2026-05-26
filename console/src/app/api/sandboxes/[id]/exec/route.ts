import { NextRequest, NextResponse } from "next/server";
import { bffError, resolveConnectionUrl } from "@/lib/controller";

export async function POST(
  request: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  const connUrl = await resolveConnectionUrl(id);
  const proxyUrl = connUrl?.replace(/\/$/, "");

  if (!proxyUrl) {
    return bffError("NO_CONNECTION", `No connection URL for sandbox ${id}`, 404);
  }

  let body: unknown;
  try {
    body = await request.json();
  } catch {
    return bffError("BAD_REQUEST", "Invalid JSON", 400);
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
      return bffError("UPSTREAM_ERROR", `agentd returned ${res.status}`, 502);
    }
    return NextResponse.json(data, { status: res.status });
  } catch (err) {
    const message = err instanceof Error ? err.message : "Unknown error";
    return bffError("EXEC_FAILED", message, 502);
  }
}
