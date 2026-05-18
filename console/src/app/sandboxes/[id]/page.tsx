"use client";

import { use } from "react";
import Link from "next/link";
import { useSandbox } from "@/api/hooks";
import { statusBorderColor, statusTextColor } from "@/components/sandbox-row";
import { formatMemoryMb } from "@/lib/format";

export default function SandboxDetailPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const { data: sandbox, isLoading, error } = useSandbox(id);

  if (isLoading) return <div className="p-8 text-text-3">Loading...</div>;
  if (error || !sandbox) return <div className="p-8 text-status-red">Sandbox not found</div>;

  return (
    <div className="max-w-6xl mx-auto px-6 py-8">
      <div className="text-sm text-text-3 mb-2">
        <Link href="/sandboxes" className="hover:text-text">Sandboxes</Link>
        {" / "}
        <span className="text-text">{id}</span>
      </div>

      <div className="flex items-center gap-4 mb-8">
        <h1 className="text-2xl font-semibold font-mono tracking-tight">{id}</h1>
        <span className={`text-xs px-2 py-0.5 rounded border ${statusBorderColor(sandbox.status)} ${statusTextColor(sandbox.status)}`}>
          {sandbox.status}
        </span>
        {sandbox.persistent && (
          <span className="text-xs px-2 py-0.5 rounded bg-accent-pink/15 text-accent-pink">
            persistent
          </span>
        )}
      </div>

      <div className="flex gap-8 text-sm text-text-3 mb-8">
        <span>runtime <span className="text-text">{sandbox.runtime}</span></span>
        <span>resources <span className="text-text">{sandbox.resources.vcpus} vCPU / {formatMemoryMb(sandbox.resources.memory_mb)}</span></span>
        <span>arch <span className="text-text">{sandbox.arch}</span></span>
        <span>drone <span className="text-accent-blue">{sandbox.drone_name ?? "—"}</span></span>
        <span>trust <span className="text-text">{sandbox.trust}</span></span>
      </div>

      <div className="flex gap-3">
        <Link
          href={`/sandboxes/${id}/terminal`}
          className="px-4 py-2 rounded border border-border text-sm hover:border-border-light transition-colors"
        >
          Terminal
        </Link>
      </div>
    </div>
  );
}
