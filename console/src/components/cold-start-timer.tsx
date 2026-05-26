"use client";

import { useState, useEffect, useRef } from "react";

type ColdStartTimerProps = {
  running: boolean;
  onElapsed?: (ms: number) => void;
};

export function ColdStartTimer({ running, onElapsed }: ColdStartTimerProps) {
  const [elapsed, setElapsed] = useState(0);
  const startRef = useRef(0);
  const rafRef = useRef(0);

  useEffect(() => {
    if (!running) {
      if (elapsed > 0) onElapsed?.(elapsed);
      return;
    }

    startRef.current = performance.now();
    setElapsed(0);

    function tick() {
      const ms = performance.now() - startRef.current;
      setElapsed(ms);
      rafRef.current = requestAnimationFrame(tick);
    }

    rafRef.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(rafRef.current);
  }, [running]); // eslint-disable-line react-hooks/exhaustive-deps

  if (elapsed === 0) return null;

  const seconds = (elapsed / 1000).toFixed(2);
  const done = !running;

  return (
    <span
      className={`text-xs font-mono tabular-nums ${
        done ? "text-status-green" : "text-status-amber"
      }`}
    >
      {done ? `${seconds}s` : `${seconds}s...`}
    </span>
  );
}
