"use client";

import {
  ResponsiveContainer,
  AreaChart,
  Area,
  XAxis,
  YAxis,
  Tooltip,
} from "recharts";
import type { UsageRecord } from "@/api/types";
import { formatCpuHours } from "@/lib/format";

type UsageChartProps = {
  records: UsageRecord[];
};

export function UsageChart({ records }: UsageChartProps) {
  const data = records.map((r) => ({
    time: new Date(r.bucket_start).toLocaleTimeString("en-US", {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    }),
    cpuHours: r.cpu_nsec / 3_600_000_000_000,
  }));

  return (
    <div className="border border-border rounded-lg p-5">
      <div className="flex items-center gap-3 mb-4">
        <span className="text-sm font-medium">CPU Usage</span>
        <span className="text-xs text-text-4 font-mono">usage_record.cpu_nsec</span>
      </div>
      <ResponsiveContainer width="100%" height={200}>
        <AreaChart data={data}>
          <defs>
            <linearGradient id="cpuGrad" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="#ededed" stopOpacity={0.15} />
              <stop offset="100%" stopColor="#ededed" stopOpacity={0} />
            </linearGradient>
          </defs>
          <XAxis
            dataKey="time"
            tick={{ fill: "#444", fontSize: 11 }}
            axisLine={{ stroke: "#222" }}
            tickLine={false}
          />
          <YAxis
            tick={{ fill: "#444", fontSize: 11 }}
            axisLine={false}
            tickLine={false}
            tickFormatter={(v: number) => `${v.toFixed(0)}h`}
          />
          <Tooltip
            contentStyle={{
              background: "#111",
              border: "1px solid #222",
              borderRadius: "6px",
              fontSize: "12px",
              color: "#ededed",
            }}
            formatter={(value: number) => [formatCpuHours(value * 3_600_000_000_000), "CPU"]}
          />
          <Area
            type="monotone"
            dataKey="cpuHours"
            stroke="#ededed"
            strokeWidth={1.5}
            fill="url(#cpuGrad)"
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}
