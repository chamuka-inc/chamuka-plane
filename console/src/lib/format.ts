export function formatDuration(seconds: number): string {
  if (seconds < 60) return `${Math.round(seconds)}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h`;
  return `${Math.floor(seconds / 86400)}d`;
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.floor(bytes / 1024)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${Math.floor(bytes / (1024 * 1024))} MB`;
  const gb = bytes / (1024 * 1024 * 1024);
  return `${Number.isInteger(gb) ? gb : gb.toFixed(1)} GB`;
}

export const NS_PER_HOUR = 3_600_000_000_000;

export function cpuNsecToHours(cpuNsec: number): number {
  return cpuNsec / NS_PER_HOUR;
}

export function formatCpuHours(cpuNsec: number): string {
  return `${cpuNsecToHours(cpuNsec).toFixed(1)}h`;
}

export function formatMemoryMb(mb: number): string {
  return mb >= 1024 ? `${mb / 1024} GB` : `${mb} MB`;
}

export function memoryMbSecToGbHours(mbSeconds: number): number {
  return mbSeconds / 3600 / 1024;
}

export function formatMemoryGbHours(mbSeconds: number): string {
  return `${memoryMbSecToGbHours(mbSeconds).toFixed(1)} GBh`;
}

export function estimateSandboxCost(
  sandboxCpuNsec: number,
  totalCpuHours: number,
  totalCost: number,
): number {
  if (totalCpuHours <= 0) return 0;
  return cpuNsecToHours(sandboxCpuNsec) * (totalCost / totalCpuHours);
}

export function formatCost(dollars: number): string {
  return `$${dollars.toFixed(2)}`;
}

export function formatAge(isoTimestamp: string, now: Date = new Date()): string {
  const seconds = (now.getTime() - new Date(isoTimestamp).getTime()) / 1000;
  return formatDuration(Math.max(0, seconds));
}

import type { NodeName } from "@/api/types";

export function resolveNodeName(name: NodeName): string {
  if (typeof name === "string") return name;
  if ("Drone" in name) return name.Drone;
  if ("Proxy" in name) return name.Proxy;
  return String(name);
}

type HeartbeatStatus = "healthy" | "stale" | "disconnected";

export function formatHeartbeatAge(seconds: number): {
  text: string;
  status: HeartbeatStatus;
} {
  const text = `${formatDuration(seconds)} ago`;
  let status: HeartbeatStatus = "healthy";
  if (seconds > 90) status = "disconnected";
  else if (seconds > 30) status = "stale";
  return { text, status };
}
