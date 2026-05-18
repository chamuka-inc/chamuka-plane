import { NextResponse } from "next/server";
import { CONTROLLER_URL } from "@/lib/controller";

const DEFAULT_CLUSTER = process.env.DEFAULT_CLUSTER ?? "iad-x86-default";

const CLUSTERS = (process.env.CLUSTERS ?? DEFAULT_CLUSTER)
  .split(",")
  .map((c) => c.trim());

export async function GET() {
  const results = await Promise.allSettled(
    CLUSTERS.map(async (cluster) => {
      const res = await fetch(
        `${CONTROLLER_URL}/ctrl/c/${cluster}/state`,
      );
      if (!res.ok) {
        throw new Error(`Failed to fetch cluster ${cluster}: ${res.status}`);
      }
      const data = await res.json();
      return { name: cluster, ...data };
    }),
  );

  const clusters = [];
  for (const [i, r] of results.entries()) {
    if (r.status === "fulfilled") {
      clusters.push(r.value);
    } else {
      console.error(`Fleet: cluster ${CLUSTERS[i]} failed:`, r.reason);
    }
  }

  return NextResponse.json({ clusters });
}
