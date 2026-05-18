import Link from "next/link";
import type { Sandbox, BackendStatus } from "@/api/types";
import { formatAge, formatMemoryMb } from "@/lib/format";

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

const severityStyles: Record<Severity, { text: string; dot: string; border: string }> = {
  green:  { text: "text-status-green", dot: "bg-status-green", border: "border-status-green/30" },
  amber:  { text: "text-status-amber", dot: "bg-status-amber", border: "border-status-amber/30" },
  red:    { text: "text-status-red",   dot: "bg-status-red",   border: "border-status-red/30" },
  muted:  { text: "text-text-4",       dot: "bg-text-4",       border: "border-border" },
};

export function statusTextColor(status: BackendStatus): string {
  return severityStyles[statusSeverity[status] ?? "muted"].text;
}

export function statusBorderColor(status: BackendStatus): string {
  return severityStyles[statusSeverity[status] ?? "muted"].border;
}

function isActionable(status: BackendStatus): boolean {
  return status === "ready" || status === "waiting";
}

type SandboxRowProps = {
  sandbox: Sandbox;
};

export function SandboxRow({ sandbox }: SandboxRowProps) {
  const terminated = sandbox.status === "terminated";
  const actionable = isActionable(sandbox.status);
  const tags = Object.entries(sandbox.tags);
  const isDesktop = sandbox.runtime === "desktop";
  const severity = statusSeverity[sandbox.status] ?? "muted";

  return (
    <div
      className={`flex items-center justify-between px-6 py-4 border-b border-border hover:bg-bg-hover transition-colors ${
        terminated ? "opacity-50" : ""
      }`}
    >
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-3">
          <Link
            href={`/sandboxes/${sandbox.id}`}
            className="font-mono text-sm font-medium hover:underline"
          >
            {sandbox.id}
          </Link>
          <span className="flex items-center gap-1.5">
            <span className={`w-2 h-2 rounded-full ${severityStyles[severity].dot}`} />
            <span className={`text-xs ${severityStyles[severity].text}`}>
              {sandbox.status}
            </span>
          </span>
          {sandbox.persistent && (
            <span className="text-xs bg-accent-pink/15 text-accent-pink px-2 py-0.5 rounded">
              persistent
            </span>
          )}
        </div>
        <div className="text-xs text-text-3 mt-1">
          {sandbox.runtime} · {sandbox.trust} · {sandbox.resources.vcpus} vCPU /{" "}
          {formatMemoryMb(sandbox.resources.memory_mb)}{" "}
          · {sandbox.drone_name ?? "unassigned"} · {formatAge(sandbox.created_at)}
        </div>
      </div>

      <div className="flex items-center gap-3">
        {tags.length > 0 && (
          <span className="text-xs font-mono text-text-4">
            {tags.map(([k, v]) => `${k}:${v}`).join(", ")}
          </span>
        )}

        {actionable && (
          <>
            {isDesktop && (
              <Link
                href={`/sandboxes/${sandbox.id}?tab=vnc`}
                className="text-xs px-3 py-1 rounded border border-border text-text-2 hover:border-border-light transition-colors"
              >
                VNC
              </Link>
            )}
            <Link
              href={`/sandboxes/${sandbox.id}/terminal`}
              className="text-xs px-3 py-1 rounded border border-border text-text-2 hover:border-border-light transition-colors"
            >
              Terminal
            </Link>
          </>
        )}
      </div>
    </div>
  );
}
