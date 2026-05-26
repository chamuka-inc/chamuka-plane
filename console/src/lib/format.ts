export function formatDuration(seconds: number): string {
  if (seconds < 60) return `${Math.round(seconds)}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h`;
  return `${Math.floor(seconds / 86400)}d`;
}

export function formatMemoryMb(mb: number): string {
  return mb >= 1024 ? `${mb / 1024} GB` : `${mb} MB`;
}

export function formatAge(isoTimestamp: string, now: Date = new Date()): string {
  const seconds = (now.getTime() - new Date(isoTimestamp).getTime()) / 1000;
  return formatDuration(Math.max(0, seconds));
}
