"use client";

import { useFleet } from "@/api/hooks";
import { ClusterCard } from "@/components/cluster-card";
import { DroneTable } from "@/components/drone-table";

export default function FleetPage() {
  const { data, isLoading, error } = useFleet();

  if (isLoading) return <div className="p-8 text-text-3">Loading fleet data...</div>;
  if (error) return <div className="p-8 text-status-red">Failed to load fleet data</div>;
  if (!data) return null;

  const { clusters } = data;
  const allDrones = clusters.flatMap((c) =>
    c.drones.map((d) => ({ ...d, clusterName: c.name })),
  );

  return (
    <div className="max-w-6xl mx-auto px-6 py-8">
      <h1 className="text-xl font-semibold tracking-tight mb-6">Fleet</h1>

      <div className="grid grid-cols-4 gap-4 mb-8">
        {clusters.map((cluster) => (
          <ClusterCard key={cluster.name} cluster={cluster} />
        ))}
        <div className="border border-border rounded-lg p-5">
          <div className="text-xs text-text-3 mb-1">Total Fleet</div>
          <div className="flex items-baseline gap-2 mb-3">
            <span className="text-2xl font-semibold">{allDrones.length}</span>
            <span className="text-sm text-text-3">drones</span>
          </div>
          <div className="text-xs text-text-4">
            {allDrones.reduce((s, d) => s + d.backend_count, 0)} backends
          </div>
        </div>
      </div>

      <div className="border border-border rounded-lg overflow-hidden">
        {clusters.map((cluster) => (
          <DroneTable
            key={cluster.name}
            drones={cluster.drones}
            clusterName={cluster.name}
          />
        ))}
      </div>
    </div>
  );
}
