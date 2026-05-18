"use client";

import { useSandboxes } from "@/api/hooks";
import { SandboxRow } from "@/components/sandbox-row";

export default function SandboxListPage() {
  const { data, isLoading, error } = useSandboxes();

  return (
    <div className="max-w-6xl mx-auto px-6 py-8">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-semibold tracking-tight">Sandboxes</h1>
        {data && (
          <span className="text-sm text-text-3">
            {data.items.length} sandboxes ·{" "}
            {data.items.filter((s) => s.status === "ready").length} running
          </span>
        )}
      </div>

      <div className="border border-border rounded-lg overflow-hidden">
        {isLoading && (
          <div className="px-6 py-12 text-center text-text-3">Loading sandboxes...</div>
        )}
        {error && (
          <div className="px-6 py-12 text-center text-status-red">
            Failed to load sandboxes
          </div>
        )}
        {data?.items.map((sandbox) => (
          <SandboxRow key={sandbox.id} sandbox={sandbox} />
        ))}
        {data && data.items.length === 0 && (
          <div className="px-6 py-12 text-center text-text-3">
            No sandboxes. Create one with the SDK:{" "}
            <code className="text-text-2 bg-bg-2 px-2 py-0.5 rounded">
              Sandbox.create()
            </code>
          </div>
        )}
      </div>
    </div>
  );
}
