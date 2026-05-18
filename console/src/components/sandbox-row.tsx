import Link from "next/link";
import type { Sandbox, BackendStatus } from "@/api/types";
import { formatAge } from "@/lib/format";

const statusColor: Record<string, string> = {
  ready: "text-status-green",
  starting: "text-status-amber",
  loading: "text-status-amber",
  waiting: "text-status-amber",
  scheduled: "text-status-amber",
  terminating: "text-status-red",
  "hard-terminating": "text-status-red",
  terminated: "text-text-4",
};

const dotColor: Record<string, string> = {
  ready: "bg-status-green",
  starting: "bg-status-amber",
  loading: "bg-status-amber",
  waiting: "bg-status-amber",
  scheduled: "bg-status-amber",
  terminating: "bg-status-red",
  "hard-terminating": "bg-status-red",
  terminated: "bg-text-4",
};

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
            <span className={`w-2 h-2 rounded-full ${dotColor[sandbox.status]}`} />
            <span className={`text-xs ${statusColor[sandbox.status]}`}>
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
          {sandbox.resources.memory_mb >= 1024
            ? `${sandbox.resources.memory_mb / 1024} GB`
            : `${sandbox.resources.memory_mb} MB`}{" "}
          · {sandbox.drone_name ?? "unassigned"} · {formatAge(sandbox.created_at)}
        </div>
      </div>

      <div className="flex items-center gap-3">
        {tags.length > 0 && (
          <span className="text-xs font-mono text-text-4">
            {tags.map(([k, v]) => `${k}:${v}`).join(", ")}
          </span>
        )}

        {actionable && !terminated && (
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
