"use client";

import { ENV_OPTIONS, type QuickstartEnv } from "@/lib/quickstart-snippets";

type RuntimeSwitcherProps = {
  selected: QuickstartEnv;
  onChange: (env: QuickstartEnv) => void;
};

export function RuntimeSwitcher({ selected, onChange }: RuntimeSwitcherProps) {
  return (
    <div
      role="group"
      aria-label="Environment"
      className="flex flex-wrap gap-2"
    >
      {ENV_OPTIONS.map((opt) => (
        <button
          key={opt.id}
          type="button"
          onClick={() => onChange(opt.id)}
          aria-pressed={selected === opt.id}
          className={`flex items-center gap-2 px-3 sm:px-4 py-2 rounded-lg border text-sm transition-colors ${
            selected === opt.id
              ? "border-accent-blue/40 text-text bg-accent-blue/10"
              : "border-border text-text-3 hover:text-text hover:border-border-light bg-bg-1"
          }`}
        >
          <span className="w-7 h-7 rounded-md bg-bg-2 flex items-center justify-center text-xs font-mono font-bold">
            {opt.icon}
          </span>
          <span className="hidden sm:inline">{opt.label}</span>
        </button>
      ))}
    </div>
  );
}
