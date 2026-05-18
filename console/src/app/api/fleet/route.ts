import { NextResponse } from "next/server";

const CONTROLLER_INTERNAL_URL =
  process.env.CONTROLLER_INTERNAL_URL ?? "http://localhost:8080";
const DEFAULT_CLUSTER = process.env.DEFAULT_CLUSTER ?? "iad-x86-default";

const CLUSTERS = (process.env.CLUSTERS ?? DEFAULT_CLUSTER).split(",");

export async function GET() {
  const results = await Promise.allSettled(
    CLUSTERS.map(async (cluster) => {
      const res = await fetch(
        `${CONTROLLER_INTERNAL_URL}/ctrl/c/${cluster.trim()}/state`,
      );
      if (!res.ok) {
        throw new Error(`Failed to fetch cluster ${cluster}: ${res.status}`);
      }
      const data = await res.json();
      return { name: cluster.trim(), ...data };
    }),
  );

  const clusters = results
    .filter((r): r is PromiseFulfilledResult<unknown> => r.status === "fulfilled")
    .map((r) => r.value);

  return NextResponse.json({ clusters });
}
