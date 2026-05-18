"use client";

import { useMemo } from "react";
import { useSandboxes, useFleet, useUsage } from "@/api/hooks";
import { useRole } from "@/lib/role";
import { StatCard } from "@/components/stat-card";
import { ActivityFeed } from "@/components/activity-feed";
import { ClusterCard } from "@/components/cluster-card";
import { formatCost, formatAge } from "@/lib/format";
import Link from "next/link";

export default function Dashboard() {
  const { isOperator } = useRole();
  const { data: sandboxData } = useSandboxes();
  const { data: fleetData } = useFleet();
  const { data: usageData } = useUsage({ period: "24h" });

  const sandboxes = sandboxData?.items ?? [];
  const readySandboxes = useMemo(
    () => sandboxes.filter((s) => s.status === "ready"),
    [sandboxes],
  );
  const clusters = fleetData?.clusters ?? [];
  const { totalDrones, healthyDrones, staleDrones } = useMemo(() => {
    const total = clusters.reduce((sum, c) => sum + c.drones.length, 0);
    const healthy = clusters.reduce(
      (sum, c) => sum + c.drones.filter((d) => d.last_heartbeat_age <= 30).length,
      0,
    );
    return { totalDrones: total, healthyDrones: healthy, staleDrones: total - healthy };
  }, [clusters]);

  return (
    <div className="max-w-6xl mx-auto px-6 py-8">
      <div className="border border-border rounded-lg overflow-hidden mb-8">
        <div className={`grid ${isOperator ? "grid-cols-4" : "grid-cols-3"}`}>
          <StatCard
            label="Running Sandboxes"
            value={String(readySandboxes.length)}
            sub={`${sandboxes.length} total`}
          />
          <StatCard
            label="CPU Today"
            value={usageData?.summary.total_cpu_hours.toFixed(1) ?? "—"}
            suffix="h"
            sub={usageData ? `~${formatCost(usageData.summary.estimated_cost)}` : ""}
          />
          <StatCard
            label="Snapshots"
            value="—"
            sub="—"
          />
          {isOperator && (
            <StatCard
              label="Fleet"
              value={String(healthyDrones)}
              suffix={`/${totalDrones}`}
              sub={
                staleDrones > 0
                  ? `connected · ${staleDrones} stale`
                  : "all connected"
              }
              subColor={staleDrones > 0 ? "text-status-amber" : "text-text-4"}
            />
          )}
        </div>
      </div>

      {isOperator && clusters.length > 0 && (
        <div className="mb-8">
          <h2 className="text-sm font-medium mb-4">Cluster Capacity</h2>
          <div className="grid grid-cols-4 gap-4">
            {clusters.map((cluster) => (
              <ClusterCard key={cluster.name} cluster={cluster} />
            ))}
          </div>
        </div>
      )}

      <div className="grid grid-cols-2 gap-6">
        <ActivityFeed sandboxes={sandboxes} />

        <div className="border border-border rounded-lg p-5">
          <h3 className="text-sm font-medium mb-4">Quick Access</h3>
          <div className="space-y-2">
            {readySandboxes
              .slice(0, 5)
              .map((s) => (
                <Link
                  key={s.id}
                  href={`/sandboxes/${s.id}`}
                  className="flex items-center justify-between px-3 py-2 rounded hover:bg-bg-hover transition-colors"
                >
                  <span className="font-mono text-sm">{s.id}</span>
                  <span className="text-xs text-text-3">
                    {s.runtime} · {s.trust} · {formatAge(s.created_at)}
                  </span>
                </Link>
              ))}
          </div>
        </div>
      </div>
    </div>
  );
}
