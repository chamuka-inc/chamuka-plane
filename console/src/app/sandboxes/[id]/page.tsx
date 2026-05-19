"use client";

import { use, useState, useRef, useCallback } from "react";
import Link from "next/link";
import { useSandbox, useTerminateSandbox } from "@/api/hooks";
import type { ExecEntry } from "@/api/types";
import { TerminalView } from "@/components/terminal-view";
import { SdkSnippet } from "@/components/sdk-snippet";
import { ExecutionHistory } from "@/components/execution-history";
import { statusBorderColor, statusTextColor } from "@/components/sandbox-row";
import { formatMemoryMb } from "@/lib/format";
import { Code, Square, List } from "lucide-react";

export default function SandboxDetailPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const { data: sandbox, isLoading, error } = useSandbox(id);
  const terminateSandbox = useTerminateSandbox();
  const [showSnippet, setShowSnippet] = useState(false);
  const [showHistory, setShowHistory] = useState(false);
  const [execHistory, setExecHistory] = useState<ExecEntry[]>([]);
  const lastCommandRef = useRef<string | undefined>(undefined);

  const handleTerminate = useCallback(() => {
    terminateSandbox.mutate({ id });
  }, [id, terminateSandbox]);

  if (isLoading) return <div className="p-8 text-text-3">Loading...</div>;
  if (error || !sandbox) return <div className="p-8 text-status-red">Sandbox not found</div>;

  const isLive = sandbox.status === "ready" || sandbox.status === "waiting";

  return (
    <div className="flex flex-col h-[calc(100dvh-4rem)]">
      <div className="flex items-center justify-between px-4 py-2 border-b border-border shrink-0">
        <div className="flex items-center gap-3 min-w-0">
          <span className="text-xs text-text-3">
            <Link href="/sandboxes" className="hover:text-text">Sandboxes</Link>
            {" / "}
          </span>
          <span className="text-sm font-mono font-medium truncate">{id}</span>
          <span className={`text-xs px-2 py-0.5 rounded-md border ${statusBorderColor(sandbox.status)} ${statusTextColor(sandbox.status)}`}>
            {sandbox.status}
          </span>
          <span className="text-xs text-text-3 hidden sm:inline">
            {sandbox.runtime} · {sandbox.resources.vcpus} vCPU / {formatMemoryMb(sandbox.resources.memory_mb)}
          </span>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          {isLive && (
            <button
              onClick={() => { setShowHistory(!showHistory); if (!showHistory) setShowSnippet(false); }}
              className={`flex items-center gap-1.5 text-xs px-2.5 py-1.5 rounded-md border transition-colors ${
                showHistory
                  ? "border-accent-purple/40 text-accent-purple bg-accent-purple/10"
                  : "border-border text-text-3 hover:text-text hover:border-border-light"
              }`}
            >
              <List size={12} />
              <span className="hidden sm:inline">
                History{execHistory.length > 0 ? ` (${execHistory.length})` : ""}
              </span>
            </button>
          )}
          <button
            onClick={() => { setShowSnippet(!showSnippet); if (!showSnippet) setShowHistory(false); }}
            className={`flex items-center gap-1.5 text-xs px-2.5 py-1.5 rounded-md border transition-colors ${
              showSnippet
                ? "border-accent-blue/40 text-accent-blue bg-accent-blue/10"
                : "border-border text-text-3 hover:text-text hover:border-border-light"
            }`}
          >
            <Code size={12} />
            <span className="hidden sm:inline">Use in your app</span>
          </button>
          {isLive && (
            <button
              onClick={handleTerminate}
              disabled={terminateSandbox.isPending}
              className="flex items-center gap-1.5 text-xs px-2.5 py-1.5 rounded-md border border-border text-text-3 hover:text-status-red hover:border-status-red/30 transition-colors disabled:opacity-50"
            >
              <Square size={10} />
              <span className="hidden sm:inline">{terminateSandbox.isPending ? "Stopping…" : "Stop"}</span>
            </button>
          )}
        </div>
      </div>

      <div className="flex flex-1 min-h-0">
        <div className={`${showSnippet || showHistory ? "flex-1 min-w-0" : "w-full"}`}>
          {isLive ? (
            <TerminalView
              sandboxId={id}
              onCommand={(cmd) => {
                lastCommandRef.current = cmd;
              }}
              onExecComplete={(entry) => {
                setExecHistory((prev) => [...prev, entry]);
              }}
            />
          ) : (
            <div className="flex items-center justify-center h-full text-text-3 text-sm">
              Sandbox is {sandbox.status}
            </div>
          )}
        </div>

        {showHistory && (
          <div className="w-80 border-l border-border bg-bg shrink-0 overflow-hidden">
            <ExecutionHistory entries={execHistory} />
          </div>
        )}

        {showSnippet && (
          <div className="w-80 border-l border-border bg-bg shrink-0 overflow-y-auto p-4">
            <div className="text-xs text-text-3 mb-3">
              Reproduce this sandbox in your app
            </div>
            <SdkSnippet
              runtime={sandbox.runtime}
              image={sandbox.image}
              sandboxId={sandbox.id}
              commands={execHistory.map((e) => e.command)}
              lastCommand={lastCommandRef.current}
            />
          </div>
        )}
      </div>
    </div>
  );
}
