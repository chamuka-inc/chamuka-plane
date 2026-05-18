"use client";

import { useState } from "react";
import { useSandboxes, useCreateSandbox } from "@/api/hooks";
import { SandboxRow } from "@/components/sandbox-row";

const PRESETS = [
  { label: "Node.js 24", image: "sandbox-runtime:node24-latest", runtime: "node24" },
  { label: "Agent", image: "sandbox-runtime:agent-latest", runtime: "agent" },
  { label: "Test", image: "sandbox-runtime:test-latest", runtime: "test" },
] as const;

export default function SandboxListPage() {
  const { data, isLoading, error } = useSandboxes();
  const createSandbox = useCreateSandbox();
  const [creating, setCreating] = useState(false);

  function handleCreate(preset: (typeof PRESETS)[number]) {
    setCreating(true);
    createSandbox.mutate(
      { image: preset.image, runtime: preset.runtime },
      { onSettled: () => setCreating(false) },
    );
  }

  return (
    <div className="max-w-6xl mx-auto px-6 py-8">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-semibold tracking-tight">Sandboxes</h1>
        <div className="flex items-center gap-3">
          {data && (
            <span className="text-sm text-text-3">
              {data.items.length} sandboxes ·{" "}
              {data.items.filter((s) => s.status === "ready").length} running
            </span>
          )}
          <div className="relative group">
            <button
              disabled={creating}
              className="px-3 py-1.5 text-sm bg-text text-bg rounded-md hover:opacity-90 disabled:opacity-50"
            >
              {creating ? "Creating..." : "New sandbox"}
            </button>
            <div className="absolute right-0 top-full mt-1 bg-bg border border-border rounded-lg shadow-lg opacity-0 pointer-events-none group-hover:opacity-100 group-hover:pointer-events-auto transition-opacity z-10">
              {PRESETS.map((p) => (
                <button
                  key={p.runtime}
                  onClick={() => handleCreate(p)}
                  disabled={creating}
                  className="block w-full text-left px-4 py-2 text-sm hover:bg-bg-2 first:rounded-t-lg last:rounded-b-lg whitespace-nowrap disabled:opacity-50"
                >
                  {p.label}
                </button>
              ))}
            </div>
          </div>
        </div>
      </div>

      {createSandbox.error && (
        <div className="mb-4 px-4 py-3 text-sm bg-status-red/10 text-status-red border border-status-red/20 rounded-lg">
          Failed to create sandbox: {createSandbox.error.message}
        </div>
      )}

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
        {data && data.items.length === 0 && !creating && (
          <div className="px-6 py-12 text-center text-text-3">
            No sandboxes yet
          </div>
        )}
      </div>
    </div>
  );
}
