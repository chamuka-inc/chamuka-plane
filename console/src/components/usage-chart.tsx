"use client";

import { useMemo } from "react";
import {
  ResponsiveContainer,
  AreaChart,
  Area,
  XAxis,
  YAxis,
  Tooltip,
} from "recharts";
import type { UsageRecord } from "@/api/types";
import { cpuNsecToHours } from "@/lib/format";
import { tokens } from "@/lib/theme";

type UsageChartProps = {
  records: UsageRecord[];
};

export function UsageChart({ records }: UsageChartProps) {
  const data = useMemo(
    () =>
      records.map((r) => ({
        time: new Date(r.bucket_start).toLocaleTimeString("en-US", {
          hour: "2-digit",
          minute: "2-digit",
          hour12: false,
        }),
        cpuHours: cpuNsecToHours(r.cpu_nsec),
      })),
    [records],
  );

  return (
    <div className="border border-border rounded-lg p-4 sm:p-5">
      <div className="flex items-center gap-3 mb-4">
        <span className="text-sm font-medium">CPU Usage</span>
        <span className="text-xs text-text-4 font-mono hidden sm:inline">usage_record.cpu_nsec</span>
      </div>
      <ResponsiveContainer width="100%" height={200}>
        <AreaChart data={data}>
          <defs>
            <linearGradient id="cpuGrad" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor={tokens.text} stopOpacity={0.15} />
              <stop offset="100%" stopColor={tokens.text} stopOpacity={0} />
            </linearGradient>
          </defs>
          <XAxis
            dataKey="time"
            tick={{ fill: tokens.text4, fontSize: 11 }}
            axisLine={{ stroke: tokens.border }}
            tickLine={false}
          />
          <YAxis
            tick={{ fill: tokens.text4, fontSize: 11 }}
            axisLine={false}
            tickLine={false}
            tickFormatter={(v: number) => `${v.toFixed(0)}h`}
          />
          <Tooltip
            contentStyle={{
              background: tokens.bg2,
              border: `1px solid ${tokens.border}`,
              borderRadius: tokens.radiusMd,
              fontSize: "12px",
              color: tokens.text,
            }}
            formatter={(value: number) => [`${value.toFixed(1)}h`, "CPU"]}
          />
          <Area
            type="monotone"
            dataKey="cpuHours"
            stroke={tokens.text}
            strokeWidth={1.5}
            fill="url(#cpuGrad)"
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}
