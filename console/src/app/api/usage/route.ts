import { NextRequest, NextResponse } from "next/server";
import { CONTROLLER_URL, authHeaders } from "@/lib/controller";

const EMPTY_USAGE = {
  records: [],
  summary: {
    total_cpu_hours: 0,
    total_memory_gb_hours: 0,
    total_rx_bytes: 0,
    total_tx_bytes: 0,
    estimated_cost: 0,
  },
};

export async function GET(request: NextRequest) {
  const { searchParams } = request.nextUrl;
  const period = searchParams.get("period") ?? "24h";
  const sandboxId = searchParams.get("sandbox_id");

  const params = new URLSearchParams({ period });
  if (sandboxId) params.set("sandbox_id", sandboxId);

  try {
    const res = await fetch(`${CONTROLLER_URL}/pub/v1/usage?${params}`, {
      headers: authHeaders,
    });

    let body;
    try {
      body = await res.json();
    } catch {
      return NextResponse.json(EMPTY_USAGE, { status: 502 });
    }

    if (!res.ok) {
      return NextResponse.json(EMPTY_USAGE, { status: res.status });
    }

    return NextResponse.json(body);
  } catch {
    return NextResponse.json(EMPTY_USAGE, { status: 502 });
  }
}
