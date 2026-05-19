"use client";

import { useState, useCallback } from "react";
import { ChevronRight, Plus, X, Shield, ShieldAlert, ShieldCheck } from "lucide-react";

export type SecretEntry = {
  destination: string;
  header_name: string;
  header_format: string;
  value: string;
};

export type SandboxConfigValues = {
  trust: "trusted" | "untrusted" | "paranoid";
  vcpus: number;
  timeout_ms: number;
  max_idle_ms: number;
  env: Record<string, string>;
  secrets: SecretEntry[];
  persistent: boolean;
};

export const DEFAULT_CONFIG: SandboxConfigValues = {
  trust: "untrusted",
  vcpus: 2,
  timeout_ms: 300_000,
  max_idle_ms: 60_000,
  env: {},
  secrets: [],
  persistent: false,
};

type SandboxConfigProps = {
  value: SandboxConfigValues;
  onChange: (value: SandboxConfigValues) => void;
};

const TRUST_OPTIONS = [
  { value: "trusted" as const, label: "Trusted", desc: "runc · fastest cold start", Icon: ShieldCheck },
  { value: "untrusted" as const, label: "Untrusted", desc: "gVisor · default isolation", Icon: Shield },
  { value: "paranoid" as const, label: "Paranoid", desc: "Kata + Firecracker · full VM", Icon: ShieldAlert },
];

const VCPU_OPTIONS = [1, 2, 4, 8];

const TIMEOUT_PRESETS = [
  { label: "1 min", ms: 60_000 },
  { label: "5 min", ms: 300_000 },
  { label: "30 min", ms: 1_800_000 },
  { label: "1 hr", ms: 3_600_000 },
];

function EnvEditor({
  env,
  onChange,
}: {
  env: Record<string, string>;
  onChange: (env: Record<string, string>) => void;
}) {
  const entries = Object.entries(env);
  const [draftKey, setDraftKey] = useState("");
  const [draftValue, setDraftValue] = useState("");

  const handleAdd = useCallback(() => {
    const k = draftKey.trim();
    if (!k) return;
    onChange({ ...env, [k]: draftValue });
    setDraftKey("");
    setDraftValue("");
  }, [draftKey, draftValue, env, onChange]);

  const handleRemove = useCallback(
    (key: string) => {
      const next = { ...env };
      delete next[key];
      onChange(next);
    },
    [env, onChange],
  );

  return (
    <div className="space-y-1.5">
      {entries.map(([k, v]) => (
        <div key={k} className="flex items-center gap-1.5">
          <span className="text-xs font-mono text-accent-cyan truncate min-w-0">{k}</span>
          <span className="text-xs text-text-4">=</span>
          <span className="text-xs font-mono text-text-2 truncate flex-1 min-w-0">{v}</span>
          <button
            onClick={() => handleRemove(k)}
            className="text-text-4 hover:text-status-red transition-colors shrink-0 p-0.5"
          >
            <X size={10} />
          </button>
        </div>
      ))}
      <div className="flex items-center gap-1.5">
        <input
          type="text"
          value={draftKey}
          onChange={(e) => setDraftKey(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") handleAdd(); }}
          placeholder="KEY"
          className="w-24 text-xs font-mono bg-bg-1 border border-border rounded px-1.5 py-1 text-text placeholder:text-text-4 focus:outline-none focus:border-border-light"
        />
        <span className="text-xs text-text-4">=</span>
        <input
          type="text"
          value={draftValue}
          onChange={(e) => setDraftValue(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") handleAdd(); }}
          placeholder="value"
          className="flex-1 min-w-0 text-xs font-mono bg-bg-1 border border-border rounded px-1.5 py-1 text-text placeholder:text-text-4 focus:outline-none focus:border-border-light"
        />
        <button
          onClick={handleAdd}
          disabled={!draftKey.trim()}
          className="text-text-3 hover:text-text disabled:text-text-4 transition-colors shrink-0 p-0.5"
        >
          <Plus size={12} />
        </button>
      </div>
    </div>
  );
}

function SecretEditor({
  secrets,
  onChange,
}: {
  secrets: SecretEntry[];
  onChange: (secrets: SecretEntry[]) => void;
}) {
  const [draftDest, setDraftDest] = useState("");
  const [draftHeader, setDraftHeader] = useState("");
  const [draftValue, setDraftValue] = useState("");

  const handleAdd = useCallback(() => {
    const dest = draftDest.trim();
    const val = draftValue;
    if (!dest || !val) return;

    let headerFormat = "{value}";
    let storedValue = val;
    if (val.startsWith("Bearer ")) {
      headerFormat = "Bearer {value}";
      storedValue = val.slice(7);
    }

    onChange([
      ...secrets,
      {
        destination: dest,
        header_name: draftHeader.trim() || "Authorization",
        header_format: headerFormat,
        value: storedValue,
      },
    ]);
    setDraftDest("");
    setDraftHeader("");
    setDraftValue("");
  }, [draftDest, draftHeader, draftValue, secrets, onChange]);

  const handleRemove = useCallback(
    (index: number) => {
      onChange(secrets.filter((_, i) => i !== index));
    },
    [secrets, onChange],
  );

  return (
    <div className="space-y-1.5">
      {secrets.map((s, i) => (
        <div key={`${s.destination}-${s.header_name}-${i}`} className="flex items-center gap-1.5">
          <span className="text-xs font-mono text-accent-cyan truncate min-w-0">{s.destination}</span>
          <span className="text-xs text-text-4">/</span>
          <span className="text-xs text-text-2 truncate">{s.header_name}</span>
          <span className="text-xs text-text-4">=</span>
          <span className="text-xs font-mono text-text-3 truncate flex-1 min-w-0">{"********"}</span>
          <button
            onClick={() => handleRemove(i)}
            className="text-text-4 hover:text-status-red transition-colors shrink-0 p-0.5"
          >
            <X size={10} />
          </button>
        </div>
      ))}
      <div className="flex items-center gap-1.5">
        <input
          type="text"
          value={draftDest}
          onChange={(e) => setDraftDest(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") handleAdd(); }}
          placeholder="api.example.com"
          className="w-28 text-xs font-mono bg-bg-1 border border-border rounded px-1.5 py-1 text-text placeholder:text-text-4 focus:outline-none focus:border-border-light"
        />
        <input
          type="text"
          value={draftHeader}
          onChange={(e) => setDraftHeader(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") handleAdd(); }}
          placeholder="Authorization"
          className="w-20 text-xs font-mono bg-bg-1 border border-border rounded px-1.5 py-1 text-text placeholder:text-text-4 focus:outline-none focus:border-border-light"
        />
        <input
          type="password"
          value={draftValue}
          onChange={(e) => setDraftValue(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") handleAdd(); }}
          placeholder="secret value"
          className="flex-1 min-w-0 text-xs font-mono bg-bg-1 border border-border rounded px-1.5 py-1 text-text placeholder:text-text-4 focus:outline-none focus:border-border-light"
        />
        <button
          onClick={handleAdd}
          disabled={!draftDest.trim() || !draftValue}
          className="text-text-3 hover:text-text disabled:text-text-4 transition-colors shrink-0 p-0.5"
        >
          <Plus size={12} />
        </button>
      </div>
    </div>
  );
}

function isDefault(config: SandboxConfigValues): boolean {
  return (
    config.trust === DEFAULT_CONFIG.trust &&
    config.vcpus === DEFAULT_CONFIG.vcpus &&
    config.timeout_ms === DEFAULT_CONFIG.timeout_ms &&
    config.max_idle_ms === DEFAULT_CONFIG.max_idle_ms &&
    Object.keys(config.env).length === 0 &&
    config.secrets.length === 0 &&
    !config.persistent
  );
}

function countOverrides(config: SandboxConfigValues): number {
  let n = 0;
  if (config.trust !== DEFAULT_CONFIG.trust) n++;
  if (config.vcpus !== DEFAULT_CONFIG.vcpus) n++;
  if (config.timeout_ms !== DEFAULT_CONFIG.timeout_ms) n++;
  if (config.max_idle_ms !== DEFAULT_CONFIG.max_idle_ms) n++;
  if (Object.keys(config.env).length > 0) n++;
  if (config.secrets.length > 0) n++;
  if (config.persistent) n++;
  return n;
}

export function SandboxConfig({ value, onChange }: SandboxConfigProps) {
  const [open, setOpen] = useState(false);
  const overrides = countOverrides(value);

  const update = useCallback(
    (patch: Partial<SandboxConfigValues>) => onChange({ ...value, ...patch }),
    [value, onChange],
  );

  return (
    <div className="w-full max-w-md">
      <button
        onClick={() => setOpen(!open)}
        className="flex items-center gap-2 text-xs text-text-3 hover:text-text transition-colors mx-auto"
      >
        <ChevronRight
          size={12}
          className={`transition-transform ${open ? "rotate-90" : ""}`}
        />
        <span>Configure</span>
        {overrides > 0 && (
          <span className="text-xs px-1.5 py-0.5 rounded bg-accent-purple/15 text-accent-purple font-mono">
            {overrides}
          </span>
        )}
      </button>

      {open && (
        <div className="mt-4 border border-border rounded-lg bg-bg-1 divide-y divide-border">
          {/* Trust tier */}
          <div className="p-3 space-y-2">
            <label className="text-xs text-text-3 font-medium">Isolation</label>
            <div className="flex gap-1.5">
              {TRUST_OPTIONS.map(({ value: t, label, desc, Icon }) => (
                <button
                  key={t}
                  onClick={() => update({ trust: t })}
                  className={`flex-1 flex flex-col items-center gap-1 px-2 py-2 rounded-md border text-center transition-colors ${
                    value.trust === t
                      ? "border-accent-purple/40 bg-accent-purple/10 text-text"
                      : "border-border text-text-3 hover:border-border-light hover:text-text"
                  }`}
                >
                  <Icon size={14} />
                  <span className="text-xs font-medium">{label}</span>
                  <span className="text-[10px] text-text-4 leading-tight">{desc}</span>
                </button>
              ))}
            </div>
          </div>

          {/* Resources */}
          <div className="p-3 space-y-2">
            <label className="text-xs text-text-3 font-medium">Resources</label>
            <div className="flex items-center gap-2">
              <span className="text-xs text-text-4 w-12 shrink-0">vCPU</span>
              <div className="flex gap-1">
                {VCPU_OPTIONS.map((n) => (
                  <button
                    key={n}
                    onClick={() => update({ vcpus: n })}
                    className={`text-xs font-mono px-2.5 py-1 rounded-md border transition-colors ${
                      value.vcpus === n
                        ? "border-accent-blue/40 bg-accent-blue/10 text-accent-blue"
                        : "border-border text-text-3 hover:border-border-light hover:text-text"
                    }`}
                  >
                    {n}
                  </button>
                ))}
              </div>
              <span className="text-xs text-text-4 ml-auto font-mono">
                {value.vcpus * 2} GB
              </span>
            </div>
          </div>

          {/* Timeout */}
          <div className="p-3 space-y-2">
            <label className="text-xs text-text-3 font-medium">Timeout</label>
            <div className="flex gap-1">
              {TIMEOUT_PRESETS.map(({ label, ms }) => (
                <button
                  key={ms}
                  onClick={() => update({ timeout_ms: ms })}
                  className={`text-xs px-2.5 py-1 rounded-md border transition-colors ${
                    value.timeout_ms === ms
                      ? "border-accent-blue/40 bg-accent-blue/10 text-accent-blue"
                      : "border-border text-text-3 hover:border-border-light hover:text-text"
                  }`}
                >
                  {label}
                </button>
              ))}
            </div>
          </div>

          {/* Env vars */}
          <div className="p-3 space-y-2">
            <label className="text-xs text-text-3 font-medium">Environment</label>
            <EnvEditor env={value.env} onChange={(env) => update({ env })} />
          </div>

          {/* Secrets */}
          <div className="p-3 space-y-2">
            <div>
              <label className="text-xs text-text-3 font-medium">Secrets</label>
              <span className="text-[10px] text-text-4 ml-2">Injected by proxy — sandbox never sees these</span>
            </div>
            <SecretEditor secrets={value.secrets} onChange={(secrets) => update({ secrets })} />
          </div>

          {/* Persistent toggle */}
          <div className="p-3 flex items-center justify-between">
            <div>
              <span className="text-xs text-text-3 font-medium">Persistent</span>
              <span className="text-[10px] text-text-4 ml-2">Survives idle timeout</span>
            </div>
            <button
              onClick={() => update({ persistent: !value.persistent })}
              className={`w-8 h-[18px] rounded-full transition-colors relative ${
                value.persistent ? "bg-accent-blue" : "bg-bg-2 border border-border"
              }`}
            >
              <span
                className={`absolute top-[2px] w-[14px] h-[14px] rounded-full bg-white transition-transform ${
                  value.persistent ? "translate-x-[16px]" : "translate-x-[2px]"
                }`}
              />
            </button>
          </div>

          {/* Reset */}
          {!isDefault(value) && (
            <div className="p-3">
              <button
                onClick={() => onChange({ ...DEFAULT_CONFIG })}
                className="text-xs text-text-4 hover:text-text transition-colors"
              >
                Reset to defaults
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
