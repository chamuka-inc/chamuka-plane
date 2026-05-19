"use client";

import { useState, useCallback, useRef } from "react";
import { useCreateSandbox, useTerminateSandbox, useBindSecrets } from "@/api/hooks";
import type { ExecEntry } from "@/api/types";
import { TerminalView } from "@/components/terminal-view";
import { SdkSnippet } from "@/components/sdk-snippet";
import { ExecutionHistory } from "@/components/execution-history";
import { SandboxConfig, DEFAULT_CONFIG } from "@/components/sandbox-config";
import type { SandboxConfigValues } from "@/components/sandbox-config";
import { ColdStartTimer } from "@/components/cold-start-timer";
import { Code, Square, Plus, Zap, List } from "lucide-react";

const RUNTIMES = [
  { label: "Node.js 24", runtime: "node24", image: "sandbox-runtime:node24-latest", icon: "N" },
  { label: "Python 3.13", runtime: "python313", image: "sandbox-runtime:python313-latest", icon: "P" },
  { label: "Agent", runtime: "agent", image: "sandbox-runtime:agent-latest", icon: "A" },
  { label: "Desktop", runtime: "desktop", image: "sandbox-runtime:desktop-test", icon: "D" },
] as const;

type ActiveSandbox = {
  id: string;
  runtime: string;
  image: string;
  coldStartMs?: number;
};

export default function Playground() {
  const createSandbox = useCreateSandbox();
  const terminateSandbox = useTerminateSandbox();
  const bindSecrets = useBindSecrets();
  const [active, setActive] = useState<ActiveSandbox | null>(null);
  const [booting, setBooting] = useState(false);
  const [showSnippet, setShowSnippet] = useState(false);
  const [showHistory, setShowHistory] = useState(false);
  const [selectedRuntime, setSelectedRuntime] = useState<string | null>(null);
  const [execHistory, setExecHistory] = useState<ExecEntry[]>([]);
  const [config, setConfig] = useState<SandboxConfigValues>({ ...DEFAULT_CONFIG });
  const lastCommandRef = useRef<string | undefined>(undefined);

  const handleBoot = useCallback(
    (preset: (typeof RUNTIMES)[number]) => {
      setBooting(true);
      setSelectedRuntime(preset.runtime);
      setActive(null);

      const envKeys = Object.keys(config.env);
      createSandbox.mutate(
        {
          image: preset.image,
          runtime: preset.runtime,
          trust: config.trust,
          resources: { vcpus: config.vcpus, memory_mb: config.vcpus * 2048 },
          timeout_ms: config.timeout_ms,
          max_idle_ms: config.max_idle_ms,
          persistent: config.persistent || undefined,
          ...(envKeys.length > 0 ? { env: config.env } : {}),
        },
        {
          onSuccess: (data) => {
            setActive({
              id: data.sandbox.id,
              runtime: preset.runtime,
              image: preset.image,
            });
            if (config.secrets.length > 0) {
              bindSecrets.mutate(
                { sandboxId: data.sandbox.id, secrets: config.secrets },
                {
                  onError: (err) => {
                    console.error("Failed to bind secrets:", err);
                  },
                },
              );
            }
            setBooting(false);
          },
          onError: () => {
            setBooting(false);
            setSelectedRuntime(null);
          },
        },
      );
    },
    [createSandbox, config, bindSecrets],
  );

  const handleStop = useCallback(() => {
    if (active) {
      terminateSandbox.mutate({ id: active.id });
    }
    setActive(null);
    setSelectedRuntime(null);
    setShowSnippet(false);
    setShowHistory(false);
    setExecHistory([]);
    lastCommandRef.current = undefined;
  }, [active, terminateSandbox]);

  const handleColdStartDone = useCallback(
    (ms: number) => {
      if (active) {
        setActive((prev) => (prev ? { ...prev, coldStartMs: ms } : null));
      }
    },
    [active],
  );

  if (!active && !booting) {
    return (
      <div className="flex flex-col items-center justify-center min-h-[calc(100dvh-4rem)] px-4">
        <div className="text-center mb-10">
          <div className="flex items-center justify-center gap-2 mb-3">
            <Zap size={20} className="text-status-amber" />
            <h1 className="text-2xl sm:text-3xl font-semibold tracking-tight">
              Boot a sandbox
            </h1>
          </div>
          <p className="text-text-3 text-sm">
            Pick a runtime. Your sandbox boots in under 2 seconds.
          </p>
        </div>

        <div className="flex flex-wrap justify-center gap-3 mb-8">
          {RUNTIMES.map((rt) => (
            <button
              key={rt.runtime}
              onClick={() => handleBoot(rt)}
              className="group flex items-center gap-3 px-5 py-3 rounded-lg border border-border bg-bg-1 hover:border-border-light hover:bg-bg-2 transition-all"
            >
              <span className="w-8 h-8 rounded-md bg-bg-2 group-hover:bg-bg-hover flex items-center justify-center text-sm font-mono font-bold text-text-2 transition-colors">
                {rt.icon}
              </span>
              <span className="text-sm font-medium">{rt.label}</span>
            </button>
          ))}
        </div>

        <SandboxConfig value={config} onChange={setConfig} />

        {createSandbox.error && (
          <div className="mt-6 text-sm text-status-red bg-status-red/10 border border-status-red/20 rounded-lg px-4 py-3 max-w-md">
            {createSandbox.error.message}
          </div>
        )}
      </div>
    );
  }

  if (booting) {
    return (
      <div className="flex flex-col items-center justify-center min-h-[calc(100dvh-4rem)] px-4">
        <div className="text-center">
          <div className="w-10 h-10 rounded-lg bg-bg-2 border border-border flex items-center justify-center text-lg font-mono font-bold text-text-2 mx-auto mb-4">
            {RUNTIMES.find((r) => r.runtime === selectedRuntime)?.icon ?? "?"}
          </div>
          <h2 className="text-lg font-semibold mb-2">Booting sandbox</h2>
          <ColdStartTimer running={true} />
        </div>
      </div>
    );
  }

  if (!active) return null;

  return (
    <div className="flex flex-col h-[calc(100dvh-4rem)]">
      <div className="flex items-center justify-between px-4 py-2 border-b border-border bg-bg shrink-0">
        <div className="flex items-center gap-3 min-w-0">
          <span className="text-xs px-2 py-0.5 rounded-md bg-bg-2 border border-border font-mono text-text-2">
            {active.runtime}
          </span>
          <span className="text-xs font-mono text-text-3 truncate">
            {active.id}
          </span>
          {active.coldStartMs !== undefined && (
            <span className="text-xs font-mono text-status-green hidden sm:inline">
              {(active.coldStartMs / 1000).toFixed(2)}s cold start
            </span>
          )}
        </div>
        <div className="flex items-center gap-2 shrink-0">
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
          <button
            onClick={handleStop}
            disabled={terminateSandbox.isPending}
            className="flex items-center gap-1.5 text-xs px-2.5 py-1.5 rounded-md border border-border text-text-3 hover:text-status-red hover:border-status-red/30 transition-colors disabled:opacity-50"
          >
            <Square size={10} />
            <span className="hidden sm:inline">{terminateSandbox.isPending ? "Stopping…" : "Stop"}</span>
          </button>
          <button
            onClick={handleStop}
            className="flex items-center gap-1.5 text-xs px-2.5 py-1.5 rounded-md border border-border text-text-3 hover:text-text hover:border-border-light transition-colors"
          >
            <Plus size={12} />
            <span className="hidden sm:inline">New</span>
          </button>
        </div>
      </div>

      <div className="flex flex-1 min-h-0">
        <div className={`${showSnippet || showHistory ? "flex-1 min-w-0" : "w-full"}`}>
          <TerminalView
            sandboxId={active.id}
            onCommand={(cmd) => {
              lastCommandRef.current = cmd;
            }}
            onExecComplete={(entry) => {
              setExecHistory((prev) => [...prev, entry]);
            }}
          />
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
              runtime={active.runtime}
              image={active.image}
              sandboxId={active.id}
              commands={execHistory.map((e) => e.command)}
              lastCommand={lastCommandRef.current}
              config={config}
            />
          </div>
        )}
      </div>
    </div>
  );
}
