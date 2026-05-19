"use client";

import { use, useState, useRef, useCallback, useEffect } from "react";
import Link from "next/link";
import { useSandbox, useTerminateSandbox } from "@/api/hooks";
import type { ExecEntry } from "@/api/types";
import { TerminalView } from "@/components/terminal-view";
import { DesktopView } from "@/components/desktop-view";
import { TerminalDrawer } from "@/components/terminal-drawer";
import { SdkSnippet } from "@/components/sdk-snippet";
import { ExecutionHistory } from "@/components/execution-history";
import { statusBorderColor, statusTextColor } from "@/lib/sandbox-status";
import { formatMemoryMb } from "@/lib/format";
import { useMediaQuery } from "@/lib/use-media-query";
import { usePersistedLanguage } from "@/lib/use-persisted-language";
import { ColdStartTimer } from "@/components/cold-start-timer";
import { Code, Square, List } from "lucide-react";

export default function SandboxDetailPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const isTransitioning = useRef(true);
  const { data: sandbox, isLoading, error } = useSandbox(id, {
    refetchInterval: isTransitioning.current ? 1_000 : 10_000,
  });
  const terminateSandbox = useTerminateSandbox();
  const isSmUp = useMediaQuery("(min-width: 640px)");
  const [language, setLanguage] = usePersistedLanguage();
  const [showSnippet, setShowSnippet] = useState(false);
  const [snippetInitialized, setSnippetInitialized] = useState(false);
  const [showHistory, setShowHistory] = useState(false);
  const [execHistory, setExecHistory] = useState<ExecEntry[]>([]);
  const lastCommandRef = useRef<string | undefined>(undefined);
  const [stopping, setStopping] = useState(false);

  useEffect(() => {
    if (!snippetInitialized) {
      setShowSnippet(isSmUp);
      setSnippetInitialized(true);
    }
  }, [isSmUp, snippetInitialized]);

  const handleTerminate = useCallback(() => {
    setStopping(true);
    isTransitioning.current = true;
    terminateSandbox.mutate({ id });
  }, [id, terminateSandbox]);

  const handleCommand = useCallback((cmd: string) => {
    lastCommandRef.current = cmd;
  }, []);

  const handleExecComplete = useCallback((entry: ExecEntry) => {
    setExecHistory((prev) => [...prev, entry]);
  }, []);

  if (isLoading) return <div className="p-8 text-text-3">Loading...</div>;
  if (error || !sandbox) return <div className="p-8 text-status-red">Sandbox not found</div>;

  const isLive = sandbox.status === "ready" || sandbox.status === "waiting";
  const isBooting = sandbox.status === "scheduled" || sandbox.status === "loading" || sandbox.status === "starting";
  const isStopping = stopping && sandbox.status !== "terminated";
  const isDesktop = sandbox.runtime === "desktop";

  if (sandbox.status === "terminated") {
    isTransitioning.current = false;
  } else if (isLive && !stopping) {
    isTransitioning.current = false;
  }

  return (
    <div className="flex flex-col h-[calc(100dvh-4rem)] relative">
      <div className="flex items-center justify-between px-4 py-2 border-b border-border shrink-0">
        <div className="flex items-center gap-3 min-w-0">
          <span className="text-xs text-text-3 hidden sm:inline">
            <Link href="/" className="hover:text-text">Home</Link>
            {" / "}
          </span>
          <span className="text-sm font-mono font-medium truncate">{id}</span>
          <span className={`text-xs px-2 py-0.5 rounded-md border shrink-0 ${statusBorderColor(sandbox.status)} ${statusTextColor(sandbox.status)}`}>
            {sandbox.status}
          </span>
          <span className="text-xs text-text-3 hidden sm:inline">
            {sandbox.runtime} · {sandbox.resources.vcpus} vCPU / {formatMemoryMb(sandbox.resources.memory_mb)}
          </span>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          {isLive && (
            <button
              type="button"
              onClick={() => {
                setShowHistory(!showHistory);
                if (!showHistory) setShowSnippet(false);
              }}
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
          {isSmUp && (
            <button
              type="button"
              onClick={() => {
                setShowSnippet(!showSnippet);
                if (!showSnippet) setShowHistory(false);
              }}
              className={`flex items-center gap-1.5 text-xs px-2.5 py-1.5 rounded-md border transition-colors ${
                showSnippet
                  ? "border-accent-blue/40 text-accent-blue bg-accent-blue/10"
                  : "border-border text-text-3 hover:text-text hover:border-border-light"
              }`}
            >
              <Code size={12} />
              <span className="hidden sm:inline">Use in your app</span>
            </button>
          )}
          {(isLive || isStopping) && (
            <button
              type="button"
              onClick={handleTerminate}
              disabled={isStopping}
              className={`flex items-center gap-1.5 text-xs px-2.5 py-1.5 rounded-md border transition-colors disabled:opacity-50 ${
                isStopping
                  ? "border-status-red/30 text-status-red bg-status-red/10"
                  : "border-border text-text-3 hover:text-status-red hover:border-status-red/30"
              }`}
            >
              <Square size={10} />
              <span className="hidden sm:inline">{isStopping ? "Stopping…" : "Stop"}</span>
            </button>
          )}
        </div>
      </div>

      <div className={`flex flex-1 min-h-0 ${showSnippet && !isSmUp ? "flex-col" : "flex-row"}`}>
        <div
          className={`min-h-0 ${
            (showSnippet || showHistory) && isSmUp ? "flex-1 min-w-0" : "w-full flex-1"
          }`}
        >
          {isStopping ? (
            <div className="flex flex-col items-center justify-center h-full gap-3">
              <div className="w-2 h-2 rounded-full bg-status-red animate-pulse" />
              <p className="text-sm text-text-2">Stopping sandbox…</p>
            </div>
          ) : isLive && isDesktop ? (
            <div className="flex flex-col h-full">
              <DesktopView sandboxId={id} />
              <TerminalDrawer
                sandboxId={id}
                onCommand={handleCommand}
                onExecComplete={handleExecComplete}
              />
            </div>
          ) : isLive ? (
            <TerminalView
              sandboxId={id}
              onCommand={handleCommand}
              onExecComplete={handleExecComplete}
            />
          ) : isBooting ? (
            <div className="flex flex-col items-center justify-center h-full gap-3">
              <p className="text-sm text-text-2">Sandbox is {sandbox.status}…</p>
              <ColdStartTimer running={true} />
            </div>
          ) : (
            <div className="flex items-center justify-center h-full text-text-3 text-sm">
              Sandbox is {sandbox.status}
            </div>
          )}
        </div>

        {showHistory && isSmUp && (
          <div className="w-80 border-l border-border bg-bg shrink-0 overflow-hidden">
            <ExecutionHistory entries={execHistory} />
          </div>
        )}

        {showSnippet && (
          <div
            className={`bg-bg shrink-0 overflow-y-auto p-4 ${
              isSmUp
                ? "w-[200px] lg:w-[260px] border-l border-border"
                : "w-full border-t border-border"
            }`}
          >
            <div className="text-xs text-text-3 mb-3">Connect from your app</div>
            <SdkSnippet
              runtime={sandbox.runtime}
              image={sandbox.image}
              sandboxId={sandbox.id}
              variant="connect"
              language={language}
              onLanguageChange={setLanguage}
            />
          </div>
        )}
      </div>

      {!isSmUp && !showSnippet && (
        <div className="sticky bottom-0 left-0 right-0 p-3 border-t border-border bg-bg/95 backdrop-blur-sm shrink-0">
          <button
            type="button"
            onClick={() => setShowSnippet(true)}
            className="w-full flex items-center justify-center gap-2 text-sm py-2.5 rounded-lg border border-accent-blue/40 text-accent-blue bg-accent-blue/10"
          >
            <Code size={14} />
            Use in code
          </button>
        </div>
      )}
    </div>
  );
}
