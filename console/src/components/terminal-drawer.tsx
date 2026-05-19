"use client";

import { useState, useRef, useCallback, useEffect } from "react";
import { ChevronUp, ChevronDown } from "lucide-react";
import { TerminalView } from "@/components/terminal-view";
import type { ExecEntry } from "@/api/types";

const STORAGE_KEY = "terminal-drawer-height";
const DEFAULT_HEIGHT = 200;
const MIN_HEIGHT = 100;
const COLLAPSED_HEIGHT = 32;

type TerminalDrawerProps = {
  sandboxId: string;
  onCommand?: (cmd: string) => void;
  onExecComplete?: (entry: ExecEntry) => void;
};

export function TerminalDrawer({ sandboxId, onCommand, onExecComplete }: TerminalDrawerProps) {
  const [expanded, setExpanded] = useState(false);
  const [height, setHeight] = useState(() => {
    if (typeof window === "undefined") return DEFAULT_HEIGHT;
    const stored = sessionStorage.getItem(STORAGE_KEY);
    return stored ? parseInt(stored, 10) : DEFAULT_HEIGHT;
  });
  const dragging = useRef(false);
  const startY = useRef(0);
  const startHeight = useRef(0);
  const heightRef = useRef(height);

  const onDragStart = useCallback(
    (e: React.MouseEvent) => {
      if (!expanded) return;
      e.preventDefault();
      dragging.current = true;
      startY.current = e.clientY;
      startHeight.current = heightRef.current;

      const onMove = (ev: MouseEvent) => {
        if (!dragging.current) return;
        const delta = startY.current - ev.clientY;
        const maxHeight = window.innerHeight * 0.6;
        const next = Math.max(MIN_HEIGHT, Math.min(maxHeight, startHeight.current + delta));
        heightRef.current = next;
        setHeight(next);
      };

      const onUp = () => {
        dragging.current = false;
        document.removeEventListener("mousemove", onMove);
        document.removeEventListener("mouseup", onUp);
        sessionStorage.setItem(STORAGE_KEY, String(heightRef.current));
      };

      document.addEventListener("mousemove", onMove);
      document.addEventListener("mouseup", onUp);
    },
    [expanded],
  );

  const toggle = useCallback(() => setExpanded((prev) => !prev), []);

  useEffect(() => {
    if (!expanded) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") setExpanded(false);
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [expanded]);

  return (
    <div style={{ height: expanded ? height + COLLAPSED_HEIGHT : COLLAPSED_HEIGHT }}
         className="shrink-0 border-t-2 border-border-light flex flex-col">
      <div
        className={`flex items-center justify-between px-4 shrink-0 select-none ${
          expanded ? "cursor-row-resize" : "cursor-pointer"
        } bg-bg-1 hover:bg-bg-hover transition-colors`}
        style={{ height: COLLAPSED_HEIGHT }}
        onClick={expanded ? undefined : toggle}
        onMouseDown={expanded ? onDragStart : undefined}
        role="separator"
        aria-orientation="horizontal"
      >
        <div className="flex items-center gap-2">
          {expanded ? <ChevronDown size={12} className="text-text-3" /> : <ChevronUp size={12} className="text-text-3" />}
          <span className="text-xs text-text-3 font-mono">Terminal</span>
        </div>
        {expanded ? (
          <button
            type="button"
            onClick={(e) => { e.stopPropagation(); toggle(); }}
            className="text-xs text-text-3 hover:text-text transition-colors"
            aria-label="Collapse terminal"
          >
            collapse
          </button>
        ) : (
          <span className="text-xs text-text-4">click to expand</span>
        )}
      </div>
      {expanded && (
        <div className="flex-1 min-h-0 overflow-hidden">
          <TerminalView sandboxId={sandboxId} onCommand={onCommand} onExecComplete={onExecComplete} />
        </div>
      )}
    </div>
  );
}
