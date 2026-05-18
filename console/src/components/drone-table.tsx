import type { DroneState } from "@/api/types";
import { formatHeartbeatAge, resolveNodeName } from "@/lib/format";

type DroneTableProps = {
  drones: DroneState[];
  clusterName: string;
};

const heartbeatColor = {
  healthy: "text-status-green",
  stale: "text-status-amber",
  disconnected: "text-status-red",
};

export function DroneTable({ drones, clusterName }: DroneTableProps) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full min-w-[600px]">
        <thead>
          <tr className="border-b border-border">
            <th className="text-left text-xs text-text-4 uppercase tracking-wider px-4 sm:px-6 py-3">Name</th>
            <th className="text-left text-xs text-text-4 uppercase tracking-wider px-4 sm:px-6 py-3">Cluster</th>
            <th className="text-left text-xs text-text-4 uppercase tracking-wider px-4 sm:px-6 py-3">Backends</th>
            <th className="text-left text-xs text-text-4 uppercase tracking-wider px-4 sm:px-6 py-3">Heartbeat</th>
            <th className="text-left text-xs text-text-4 uppercase tracking-wider px-4 sm:px-6 py-3">Status</th>
          </tr>
        </thead>
        <tbody>
          {drones.map((drone) => {
            const hb = formatHeartbeatAge(drone.last_heartbeat_age);
            const name = resolveNodeName(drone.node.name);
            return (
              <tr key={name} className="border-b border-border hover:bg-bg-hover">
                <td className="px-4 sm:px-6 py-3 text-sm font-mono">{name}</td>
                <td className="px-4 sm:px-6 py-3 text-sm text-text-2">{clusterName}</td>
                <td className="px-4 sm:px-6 py-3 text-sm text-text-2">{drone.backend_count}</td>
                <td className={`px-4 sm:px-6 py-3 text-sm ${heartbeatColor[hb.status]}`}>
                  {hb.text}
                </td>
                <td className={`px-4 sm:px-6 py-3 text-sm ${heartbeatColor[hb.status]}`}>
                  {drone.draining
                    ? "draining"
                    : hb.status === "healthy"
                      ? "connected"
                      : hb.status}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
