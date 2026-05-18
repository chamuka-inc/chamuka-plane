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

const NS_PER_HOUR = 3_600_000_000_000;

export function formatCpuHours(cpuNsec: number): string {
  const hours = cpuNsec / NS_PER_HOUR;
  return `${hours.toFixed(1)}h`;
}

export function formatCost(dollars: number): string {
  return `$${dollars.toFixed(2)}`;
}

export function formatAge(isoTimestamp: string, now: Date = new Date()): string {
  const seconds = (now.getTime() - new Date(isoTimestamp).getTime()) / 1000;
  return formatDuration(Math.max(0, seconds));
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
