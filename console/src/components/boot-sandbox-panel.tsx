"use client";

import { useState, useCallback } from "react";
import { useCreateSandbox, useBindSecrets } from "@/api/hooks";
import { SandboxConfig, DEFAULT_CONFIG } from "@/components/sandbox-config";
import type { SandboxConfigValues } from "@/components/sandbox-config";

export const BOOT_RUNTIMES = [
  { label: "Node.js 24", runtime: "node24", image: "sandbox-runtime:node24-latest", icon: "N" },
  { label: "Python 3.13", runtime: "python313", image: "sandbox-runtime:python313-latest", icon: "P" },
  { label: "Agent", runtime: "agent", image: "sandbox-runtime:agent-latest", icon: "A" },
  { label: "Desktop", runtime: "desktop", image: "sandbox-runtime:desktop-latest", icon: "D" },
] as const;

export type BootedSandbox = {
  id: string;
  runtime: string;
  image: string;
};

type BootSandboxPanelProps = {
  onBooted: (sandbox: BootedSandbox) => void;
};

export function BootSandboxPanel({ onBooted }: BootSandboxPanelProps) {
  const createSandbox = useCreateSandbox();
  const bindSecrets = useBindSecrets();
  const [config, setConfig] = useState<SandboxConfigValues>({ ...DEFAULT_CONFIG });

  const handleBoot = useCallback(
    (preset: (typeof BOOT_RUNTIMES)[number]) => {
      createSandbox.mutate(
        {
          image: preset.image,
          runtime: preset.runtime,
          trust: config.trust,
          resources: { vcpus: config.vcpus, memory_mb: config.vcpus * 2048 },
          timeout_ms: config.timeout_ms,
          max_idle_ms: config.max_idle_ms,
          persistent: config.persistent || undefined,
          ...(() => {
            const env = { ...config.env };
            if (preset.runtime === "desktop") {
              env.VNC_ENABLED = "1";
              env.VNC_VIEWONLY = "0";
            }
            return Object.keys(env).length > 0 ? { env } : {};
          })(),
        },
        {
          onSuccess: (data) => {
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
            onBooted({
              id: data.sandbox.id,
              runtime: preset.runtime,
              image: preset.image,
            });
          },
        },
      );
    },
    [createSandbox, config, bindSecrets, onBooted],
  );

  return (
    <div className="w-full min-w-0 border border-border rounded-lg bg-bg-1 p-4 sm:p-6 flex flex-col gap-6">
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
        {BOOT_RUNTIMES.map((rt) => (
          <button
            key={rt.runtime}
            type="button"
            onClick={() => handleBoot(rt)}
            disabled={createSandbox.isPending}
            className="group flex w-full min-w-0 items-center gap-2 sm:gap-3 px-3 py-2.5 rounded-lg border border-border bg-bg-2 hover:border-border-light hover:bg-bg-hover transition-all disabled:opacity-50"
          >
            <span className="w-8 h-8 shrink-0 rounded-md bg-bg-1 group-hover:bg-bg-hover flex items-center justify-center text-sm font-mono font-bold text-text-2 transition-colors">
              {rt.icon}
            </span>
            <span className="text-sm font-medium truncate min-w-0">{rt.label}</span>
          </button>
        ))}
      </div>

      <SandboxConfig value={config} onChange={setConfig} />

      {createSandbox.error && (
        <div className="text-sm text-status-red bg-status-red/10 border border-status-red/20 rounded-lg px-4 py-3">
          {createSandbox.error.message}
        </div>
      )}
    </div>
  );
}
