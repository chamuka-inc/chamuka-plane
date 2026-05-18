"use client";

import { useUsage } from "@/api/hooks";
import { StatCard } from "@/components/stat-card";
import { UsageChart } from "@/components/usage-chart";
import { formatBytes, formatCpuHours, formatCost } from "@/lib/format";

export default function AnalyticsPage() {
  const { data, isLoading, error } = useUsage({ period: "24h" });

  if (isLoading) return <div className="p-8 text-text-3">Loading usage data...</div>;
  if (error) return <div className="p-8 text-status-red">Failed to load usage data</div>;
  if (!data) return null;

  const { summary, records } = data;

  const bySandbox = new Map<string, { cpu_nsec: number; memory_mb_seconds: number; rx_bytes: number; tx_bytes: number }>();
  for (const r of records) {
    const existing = bySandbox.get(r.sandbox_id) ?? { cpu_nsec: 0, memory_mb_seconds: 0, rx_bytes: 0, tx_bytes: 0 };
    existing.cpu_nsec += r.cpu_nsec;
    existing.memory_mb_seconds += r.memory_mb_seconds;
    existing.rx_bytes += r.rx_bytes;
    existing.tx_bytes += r.tx_bytes;
    bySandbox.set(r.sandbox_id, existing);
  }

  return (
    <div className="max-w-6xl mx-auto px-6 py-8">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-semibold tracking-tight">Usage</h1>
        <span className="text-sm text-text-3 border border-border rounded px-3 py-1">
          Last 24h
        </span>
      </div>

      <div className="border border-border rounded-lg overflow-hidden mb-8">
        <div className="grid grid-cols-4">
          <StatCard
            label="CPU Hours"
            value={summary.total_cpu_hours.toFixed(1)}
            sub="+12% vs yesterday"
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
            sub="$0.20/CPU-hour"
          />
        </div>
      </div>

      <div className="mb-8">
        <UsageChart records={records} />
      </div>

      <div className="border border-border rounded-lg overflow-hidden">
        <table className="w-full">
          <thead>
            <tr className="border-b border-border">
              <th className="text-left text-xs text-text-4 uppercase tracking-wider px-6 py-3">Sandbox</th>
              <th className="text-left text-xs text-text-4 uppercase tracking-wider px-6 py-3">CPU</th>
              <th className="text-left text-xs text-text-4 uppercase tracking-wider px-6 py-3">Memory</th>
              <th className="text-left text-xs text-text-4 uppercase tracking-wider px-6 py-3">Network</th>
              <th className="text-left text-xs text-text-4 uppercase tracking-wider px-6 py-3">Cost</th>
            </tr>
          </thead>
          <tbody>
            {Array.from(bySandbox.entries())
              .sort(([, a], [, b]) => b.cpu_nsec - a.cpu_nsec)
              .map(([sandboxId, usage]) => (
                <tr key={sandboxId} className="border-b border-border hover:bg-bg-hover">
                  <td className="px-6 py-3 text-sm font-mono">{sandboxId}</td>
                  <td className="px-6 py-3 text-sm text-text-2">
                    {formatCpuHours(usage.cpu_nsec)}
                  </td>
                  <td className="px-6 py-3 text-sm text-text-2">
                    {(usage.memory_mb_seconds / 3600 / 1024).toFixed(1)} GBh
                  </td>
                  <td className="px-6 py-3 text-sm text-text-2">
                    {formatBytes(usage.rx_bytes + usage.tx_bytes)}
                  </td>
                  <td className="px-6 py-3 text-sm text-text-2">
                    {formatCost(usage.cpu_nsec / 3_600_000_000_000 * 0.20)}
                  </td>
                </tr>
              ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
