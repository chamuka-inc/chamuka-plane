import type { ReactNode } from "react";
import { Info, AlertTriangle, Lightbulb } from "lucide-react";

type CalloutType = "info" | "warning" | "tip";

const styles: Record<CalloutType, { border: string; bg: string; icon: typeof Info }> = {
  info: { border: "border-accent-blue", bg: "bg-accent-blue/5", icon: Info },
  warning: { border: "border-status-amber", bg: "bg-status-amber/5", icon: AlertTriangle },
  tip: { border: "border-status-green", bg: "bg-status-green/5", icon: Lightbulb },
};

export function Callout({ type, children }: { type: CalloutType; children: ReactNode }) {
  const s = styles[type];
  const Icon = s.icon;
  return (
    <div className={`flex gap-3 border-l-2 ${s.border} ${s.bg} rounded-r-md px-4 py-3 my-4`}>
      <Icon size={16} className="shrink-0 mt-0.5 text-text-3" />
      <div className="text-sm text-text-2 leading-relaxed">{children}</div>
    </div>
  );
}
