import { NextRequest, NextResponse } from "next/server";
import { getSandbox } from "@/lib/sandbox-registry";

export async function GET(
  _request: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  const result = await getSandbox(id);

  if (!result) {
    return NextResponse.json(
      { error: { code: "NOT_FOUND", message: `Sandbox ${id} not found`, request_id: "" } },
      { status: 404 },
    );
  }

  return NextResponse.json(result);
}
