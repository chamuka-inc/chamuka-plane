import type { ClusterState } from "@/api/types";

type ClusterCardProps = {
  cluster: ClusterState;
};

const BACKENDS_PER_DRONE = 12;

export function ClusterCard({ cluster }: ClusterCardProps) {
  const totalSlots = cluster.drones.length * BACKENDS_PER_DRONE;
  const usedSlots = cluster.drones.reduce((sum, d) => sum + d.backend_count, 0);
  const pct = totalSlots > 0 ? (usedSlots / totalSlots) * 100 : 0;

  return (
    <div className="border border-border rounded-lg p-5">
      <div className="text-xs text-text-3 mb-1">{cluster.name}</div>
      <div className="flex items-baseline gap-2 mb-3">
        <span className="text-2xl font-semibold">{cluster.drones.length}</span>
        <span className="text-sm text-text-3">
          {cluster.drones.length === 1 ? "drone" : "drones"}
        </span>
      </div>
      <div className="h-1 bg-bg-2 rounded-full overflow-hidden mb-2">
        <div
          className="h-full bg-text rounded-full transition-all"
          style={{ width: `${Math.min(pct, 100)}%` }}
        />
      </div>
      <div className="text-xs text-text-4">
        {usedSlots}/{totalSlots} slots
      </div>
    </div>
  );
}
