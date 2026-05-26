"use client";

import { useState, useCallback, useRef, useEffect } from "react";
import type { ExecEntry } from "@/api/types";
import { Copy, Check, ChevronRight } from "lucide-react";

type ExecutionHistoryProps = {
  entries: ExecEntry[];
};

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(2)}s`;
}

function ExecRow({ entry }: { entry: ExecEntry }) {
  const [expanded, setExpanded] = useState(false);
  const ok = entry.exitCode === 0;

  return (
    <div className="border-b border-border last:border-b-0">
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full flex items-center gap-2 px-3 py-2 text-left hover:bg-bg-hover transition-colors"
      >
        <ChevronRight
          size={12}
          className={`text-text-4 shrink-0 transition-transform ${expanded ? "rotate-90" : ""}`}
        />
        <span className="text-xs font-mono text-text truncate flex-1">
          {entry.command}
        </span>
        <span className={`text-xs font-mono tabular-nums shrink-0 ${ok ? "text-text-3" : "text-status-red"}`}>
          {formatDuration(entry.durationMs)}
        </span>
        <span
          className={`text-xs font-mono w-5 text-center shrink-0 ${
            ok ? "text-status-green" : "text-status-red"
          }`}
        >
          {entry.exitCode}
        </span>
      </button>
      {expanded && (
        <div className="px-3 pb-3 space-y-2">
          {entry.stdout && (
            <pre className="text-xs font-mono text-text-2 bg-bg-1 rounded-md p-2 overflow-x-auto max-h-40 overflow-y-auto whitespace-pre-wrap">
              {entry.stdout}
            </pre>
          )}
          {entry.stderr && (
            <pre className="text-xs font-mono text-status-red/80 bg-status-red/5 rounded-md p-2 overflow-x-auto max-h-40 overflow-y-auto whitespace-pre-wrap">
              {entry.stderr}
            </pre>
          )}
          {!entry.stdout && !entry.stderr && (
            <span className="text-xs text-text-4">No output</span>
          )}
        </div>
      )}
    </div>
  );
}

export function ExecutionHistory({ entries }: ExecutionHistoryProps) {
  const [copied, setCopied] = useState(false);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [entries.length]);

  const handleCopyAll = useCallback(() => {
    const text = entries.map((e) => e.command).join("\n");
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }, [entries]);

  if (entries.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-text-4 text-xs">
        Run commands to see history
      </div>
    );
  }

  const totalMs = entries.reduce((sum, e) => sum + e.durationMs, 0);
  const failures = entries.filter((e) => e.exitCode !== 0).length;

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between px-3 py-2 border-b border-border shrink-0">
        <div className="flex items-center gap-2">
          <span className="text-xs text-text-3">
            {entries.length} cmd{entries.length !== 1 ? "s" : ""}
          </span>
          <span className="text-xs text-text-4">·</span>
          <span className="text-xs font-mono tabular-nums text-text-3">
            {formatDuration(totalMs)}
          </span>
          {failures > 0 && (
            <>
              <span className="text-xs text-text-4">·</span>
              <span className="text-xs text-status-red">
                {failures} failed
              </span>
            </>
          )}
        </div>
        <button
          onClick={handleCopyAll}
          className="text-text-3 hover:text-text transition-colors p-1"
          aria-label="Copy all commands"
        >
          {copied ? <Check size={12} className="text-status-green" /> : <Copy size={12} />}
        </button>
      </div>
      <div className="flex-1 overflow-y-auto min-h-0">
        {entries.map((entry) => (
          <ExecRow key={entry.id} entry={entry} />
        ))}
        <div ref={bottomRef} />
      </div>
    </div>
  );
}
