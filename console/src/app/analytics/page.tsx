"use client";

import { useMemo } from "react";
import { useUsage } from "@/api/hooks";
import { StatCard } from "@/components/stat-card";
import { UsageChart } from "@/components/usage-chart";
import {
  formatBytes,
  formatCpuHours,
  formatCost,
  formatMemoryGbHours,
  estimateSandboxCost,
} from "@/lib/format";

export default function AnalyticsPage() {
  const { data, isLoading, error } = useUsage({ period: "24h" });

  const { bySandbox, costPerSandbox, sortedEntries } = useMemo(() => {
    const records = data?.records ?? [];
    const estimatedCost = data?.summary.estimated_cost ?? 0;
    const map = new Map<string, { cpu_nsec: number; memory_mb_seconds: number; rx_bytes: number; tx_bytes: number }>();
    for (const r of records) {
      const existing = map.get(r.sandbox_id) ?? { cpu_nsec: 0, memory_mb_seconds: 0, rx_bytes: 0, tx_bytes: 0 };
      existing.cpu_nsec += r.cpu_nsec;
      existing.memory_mb_seconds += r.memory_mb_seconds;
      existing.rx_bytes += r.rx_bytes;
      existing.tx_bytes += r.tx_bytes;
      map.set(r.sandbox_id, existing);
    }
    const cost = map.size > 0 ? estimatedCost / map.size : 0;
    const sorted = Array.from(map.entries()).sort(([, a], [, b]) => b.cpu_nsec - a.cpu_nsec);
    return { bySandbox: map, costPerSandbox: cost, sortedEntries: sorted };
  }, [data?.records, data?.summary.estimated_cost]);

  if (isLoading) return <div className="p-8 text-text-3">Loading usage data...</div>;
  if (error) return <div className="p-8 text-status-red">Failed to load usage data</div>;
  if (!data) return null;

  const { summary, records } = data;

  return (
    <div className="max-w-6xl mx-auto px-4 sm:px-6 py-6 sm:py-8">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-semibold tracking-tight">Usage</h1>
        <span className="text-sm text-text-3 border border-border rounded-md px-3 py-1">
          Last 24h
        </span>
      </div>

      <div className="border border-border rounded-lg overflow-hidden bg-border mb-6 sm:mb-8">
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-px">
          <StatCard
            label="CPU Hours"
            value={summary.total_cpu_hours.toFixed(1)}
            sub={`across ${bySandbox.size} sandbox${bySandbox.size === 1 ? "" : "es"}`}
          />
          <StatCard
            label="Memory"
            value={summary.total_memory_gb_hours.toFixed(1)}
            suffix="GBh"
            sub={`avg ${Math.round(summary.total_memory_gb_hours * 1024 / Math.max(bySandbox.size, 1))} MB/sandbox`}
          />
          <StatCard
            label="Network I/O"
            value={(
              (summary.total_rx_bytes + summary.total_tx_bytes) /
              (1024 * 1024 * 1024)
            ).toFixed(1)}
            suffix="GB"
            sub={`rx ${formatBytes(summary.total_rx_bytes)} · tx ${formatBytes(summary.total_tx_bytes)}`}
          />
          <StatCard
            label="Est. Cost"
            value={formatCost(summary.estimated_cost)}
            sub={bySandbox.size > 0 ? `~${formatCost(costPerSandbox)}/sandbox` : ""}
          />
        </div>
      </div>

      <div className="mb-6 sm:mb-8">
        <UsageChart records={records} />
      </div>

      <div className="border border-border rounded-lg overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full min-w-[600px]">
            <thead>
              <tr className="border-b border-border">
                <th className="text-left text-xs text-text-4 uppercase tracking-wider px-4 sm:px-6 py-3">Sandbox</th>
                <th className="text-left text-xs text-text-4 uppercase tracking-wider px-4 sm:px-6 py-3">CPU</th>
                <th className="text-left text-xs text-text-4 uppercase tracking-wider px-4 sm:px-6 py-3">Memory</th>
                <th className="text-left text-xs text-text-4 uppercase tracking-wider px-4 sm:px-6 py-3">Network</th>
                <th className="text-left text-xs text-text-4 uppercase tracking-wider px-4 sm:px-6 py-3">Cost</th>
              </tr>
            </thead>
            <tbody>
              {sortedEntries.map(([sandboxId, usage]) => (
                  <tr key={sandboxId} className="border-b border-border hover:bg-bg-hover">
                    <td className="px-4 sm:px-6 py-3 text-sm font-mono">{sandboxId}</td>
                    <td className="px-4 sm:px-6 py-3 text-sm text-text-2">
                      {formatCpuHours(usage.cpu_nsec)}
                    </td>
                    <td className="px-4 sm:px-6 py-3 text-sm text-text-2">
                      {formatMemoryGbHours(usage.memory_mb_seconds)}
                    </td>
                    <td className="px-4 sm:px-6 py-3 text-sm text-text-2">
                      {formatBytes(usage.rx_bytes + usage.tx_bytes)}
                    </td>
                    <td className="px-4 sm:px-6 py-3 text-sm text-text-2">
                      {formatCost(estimateSandboxCost(usage.cpu_nsec, summary.total_cpu_hours, summary.estimated_cost))}
                    </td>
                  </tr>
                ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
