import type { ReactNode } from "react";

type QuickstartStepProps = {
  number?: number;
  /** Replaces the step number in the circle (e.g. play icon for “Try it live”). */
  marker?: ReactNode;
  title: string;
  description?: string;
  children: ReactNode;
  isLast?: boolean;
  id?: string;
};

export function QuickstartStep({
  number,
  marker,
  title,
  description,
  children,
  isLast = false,
  id,
}: QuickstartStepProps) {
  const isHighlighted = Boolean(marker);

  return (
    <div id={id} className={`flex gap-4 ${id ? "scroll-mt-24" : ""}`}>
      <div className="flex flex-col items-center shrink-0">
        <span
          className={`w-8 h-8 rounded-full border flex items-center justify-center text-sm font-mono font-semibold shrink-0 ${
            isHighlighted
              ? "border-accent-blue/40 bg-accent-blue/10 text-accent-blue"
              : "border-border bg-bg-2 text-text-2"
          }`}
        >
          {marker ?? number}
        </span>
        {!isLast && <div className="w-px flex-1 min-h-8 bg-border mt-2" />}
      </div>
      <div className="flex-1 min-w-0 pb-10 *:w-full *:min-w-0">
        <h2 className="text-base font-semibold mb-1 w-auto">{title}</h2>
        {description && (
          <p className="text-sm text-text-3 mb-4 w-auto">{description}</p>
        )}
        {children}
      </div>
    </div>
  );
}
