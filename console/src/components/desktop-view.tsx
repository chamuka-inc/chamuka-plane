"use client";

import { useEffect, useRef, useState, useCallback } from "react";
import { Maximize, Minimize } from "lucide-react";

type ConnectionState = "connecting" | "connected" | "disconnected";

type DesktopViewProps = {
  sandboxId: string;
  onConnectionChange?: (state: ConnectionState) => void;
};

function getWsRelayUrl() {
  if (process.env.NEXT_PUBLIC_WS_RELAY_URL) {
    return process.env.NEXT_PUBLIC_WS_RELAY_URL;
  }
  if (typeof window === "undefined") return "ws://localhost:3101";
  const isSecure = window.location.protocol === "https:";
  const proto = isSecure ? "wss" : "ws";
  const portSuffix = isSecure
    ? (window.location.port ? `:${window.location.port}` : "")
    : ":3101";
  return `${proto}://${window.location.hostname}${portSuffix}`;
}

export function DesktopView({ sandboxId, onConnectionChange }: DesktopViewProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const rfbRef = useRef<InstanceType<typeof import("@novnc/novnc").default> | null>(null);
  const [connState, setConnState] = useState<ConnectionState>("connecting");
  const [resolution, setResolution] = useState<string>("");
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [retryIn, setRetryIn] = useState(0);
  const retryTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const retryTickRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const retryDelayRef = useRef(1000);
  const generationRef = useRef(0);
  const rfbModuleRef = useRef<typeof import("@novnc/novnc").default | null>(null);
  const connectRef = useRef<(() => Promise<void>) | null>(null);
  const onConnectionChangeRef = useRef(onConnectionChange);
  onConnectionChangeRef.current = onConnectionChange;

  const updateState = useCallback((state: ConnectionState) => {
    setConnState(state);
    onConnectionChangeRef.current?.(state);
  }, []);

  const scheduleRetry = useCallback(() => {
    const delay = retryDelayRef.current;
    retryDelayRef.current = Math.min(delay * 2, 10000);

    let remaining = Math.ceil(delay / 1000);
    setRetryIn(remaining);

    if (retryTimerRef.current) clearTimeout(retryTimerRef.current);
    if (retryTickRef.current) clearInterval(retryTickRef.current);
    retryTickRef.current = setInterval(() => {
      remaining -= 1;
      setRetryIn(remaining);
      if (remaining <= 0) {
        clearInterval(retryTickRef.current!);
        retryTickRef.current = null;
      }
    }, 1000);

    retryTimerRef.current = setTimeout(() => {
      if (retryTickRef.current) { clearInterval(retryTickRef.current); retryTickRef.current = null; }
      setRetryIn(0);
      void connectRef.current?.();
    }, delay);
  }, []);

  const connect = useCallback(async () => {
    if (!containerRef.current) return;

    if (rfbRef.current) {
      rfbRef.current.disconnect();
      rfbRef.current = null;
    }
    containerRef.current.replaceChildren();

    const gen = ++generationRef.current;

    updateState("connecting");
    retryDelayRef.current = 1000;

    if (!rfbModuleRef.current) {
      rfbModuleRef.current = (await import("@novnc/novnc")).default;
    }
    const RFB = rfbModuleRef.current;

    if (gen !== generationRef.current || !containerRef.current) return;

    const url = `${getWsRelayUrl()}/vnc?sandbox=${encodeURIComponent(sandboxId)}`;
    const rfb = new RFB(containerRef.current, url);
    rfb.scaleViewport = true;
    rfb.resizeSession = false;
    rfb.focusOnClick = true;
    rfbRef.current = rfb;

    rfb.addEventListener("connect", () => {
      if (gen !== generationRef.current) return;
      updateState("connected");
      const canvas = containerRef.current?.querySelector("canvas");
      if (canvas) {
        setResolution(`${canvas.width}×${canvas.height}`);
      }
    });

    rfb.addEventListener("disconnect", (e: Event) => {
      if (gen !== generationRef.current) return;
      rfbRef.current = null;
      updateState("disconnected");
      const detail = (e as CustomEvent<{ clean?: boolean }>).detail;
      if (!detail?.clean) {
        scheduleRetry();
      }
    });
  }, [sandboxId, updateState, scheduleRetry]);

  connectRef.current = connect;

  useEffect(() => {
    connect();
    return () => {
      generationRef.current++;
      if (retryTimerRef.current) clearTimeout(retryTimerRef.current);
      if (retryTickRef.current) clearInterval(retryTickRef.current);
      rfbRef.current?.disconnect();
      rfbRef.current = null;
      if (containerRef.current) containerRef.current.replaceChildren();
    };
  }, [connect]);

  useEffect(() => {
    const handler = () => setIsFullscreen(!!document.fullscreenElement);
    document.addEventListener("fullscreenchange", handler);
    return () => document.removeEventListener("fullscreenchange", handler);
  }, []);

  const toggleFullscreen = useCallback(() => {
    const el = containerRef.current?.parentElement;
    if (!el) return;
    if (document.fullscreenElement) {
      document.exitFullscreen();
    } else {
      el.requestFullscreen();
    }
  }, []);

  const statusDot = {
    connecting: "bg-status-amber",
    connected: "bg-status-green",
    disconnected: "bg-status-red",
  }[connState];

  const statusText = connState === "connected"
    ? `vnc · ${resolution}`
    : connState === "connecting"
      ? "connecting..."
      : retryIn > 0
        ? `disconnected — retry in ${retryIn}s`
        : "disconnected";

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between px-4 py-1.5 border-b border-border bg-bg-1">
        <div className="flex items-center gap-2">
          <span className={`w-1.5 h-1.5 rounded-full ${statusDot}`} />
          <span className="text-xs text-text-3 font-mono">{statusText}</span>
        </div>
        <button
          type="button"
          onClick={toggleFullscreen}
          className="flex items-center gap-1.5 text-xs text-text-3 hover:text-text transition-colors"
          aria-label={isFullscreen ? "Exit fullscreen" : "Fullscreen"}
        >
          {isFullscreen ? <Minimize size={12} /> : <Maximize size={12} />}
          <span className="hidden sm:inline">{isFullscreen ? "Exit" : "Fullscreen"}</span>
        </button>
      </div>
      <div ref={containerRef} className="flex-1 min-h-0 bg-black" />
    </div>
  );
}
