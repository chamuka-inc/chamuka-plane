import { NextRequest, NextResponse } from "next/server";
import { createSandbox, listSandboxes } from "@/lib/sandbox-registry";

export async function GET(request: NextRequest) {
  const { searchParams } = request.nextUrl;
  const status = searchParams.get("status") ?? undefined;
  const limit = Number(searchParams.get("limit") ?? "100");

  const result = await listSandboxes({ status, limit });
  return NextResponse.json(result);
}

export async function POST(request: NextRequest) {
  let body: Record<string, unknown>;
  try {
    body = await request.json();
  } catch {
    return NextResponse.json(
      { error: { code: "BAD_REQUEST", message: "Invalid JSON", request_id: "" } },
      { status: 400 },
    );
  }

  try {
    const result = await createSandbox(body);
    return NextResponse.json(result, { status: 201 });
  } catch (err) {
    const message = err instanceof Error ? err.message : "Unknown error";
    return NextResponse.json(
      { error: { code: "CREATE_FAILED", message, request_id: "" } },
      { status: 502 },
    );
  }
}
