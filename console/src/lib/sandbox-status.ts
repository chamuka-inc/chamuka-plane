import type { BackendStatus } from "@/api/types";

type Severity = "green" | "amber" | "red" | "muted";

const statusSeverity: Record<BackendStatus, Severity> = {
  ready: "green",
  starting: "amber",
  loading: "amber",
  waiting: "amber",
  scheduled: "amber",
  terminating: "red",
  "hard-terminating": "red",
  terminated: "muted",
};

const severityStyles: Record<Severity, { text: string; border: string }> = {
  green: { text: "text-status-green", border: "border-status-green/30" },
  amber: { text: "text-status-amber", border: "border-status-amber/30" },
  red: { text: "text-status-red", border: "border-status-red/30" },
  muted: { text: "text-text-4", border: "border-border" },
};

export function statusTextColor(status: BackendStatus): string {
  return severityStyles[statusSeverity[status] ?? "muted"].text;
}

export function statusBorderColor(status: BackendStatus): string {
  return severityStyles[statusSeverity[status] ?? "muted"].border;
}
