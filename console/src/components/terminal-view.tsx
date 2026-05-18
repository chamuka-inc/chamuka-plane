"use client";

import { useEffect, useRef, useState, useCallback } from "react";
import "@xterm/xterm/css/xterm.css";

const WS_RELAY_URL = process.env.NEXT_PUBLIC_WS_RELAY_URL ?? "ws://localhost:3101";

type TerminalViewProps = {
  sandboxId: string;
};

export function TerminalView({ sandboxId }: TerminalViewProps) {
  const terminalRef = useRef<HTMLDivElement>(null);
  const [status, setStatus] = useState<"connecting" | "connected" | "error">("connecting");
  const [errorMsg, setErrorMsg] = useState<string>("");
  const cleanupRef = useRef<(() => void) | null>(null);

  const connect = useCallback(async () => {
    if (!terminalRef.current) return;

    setStatus("connecting");

    const [{ Terminal }, { FitAddon }] = await Promise.all([
      import("@xterm/xterm"),
      import("@xterm/addon-fit"),
    ]);

    const terminal = new Terminal({
      fontFamily: "'SF Mono', 'Menlo', monospace",
      fontSize: 13,
      lineHeight: 1.4,
      theme: {
        background: "#0a0a0a",
        foreground: "#ededed",
        cursor: "#ededed",
        selectionBackground: "rgba(0,112,243,0.3)",
        black: "#000000",
        red: "#ee0000",
        green: "#0cce6b",
        yellow: "#f5a623",
        blue: "#0070f3",
        magenta: "#f81ce5",
        cyan: "#50e3c2",
        white: "#ededed",
      },
      cursorBlink: true,
      scrollback: 5000,
    });

    const fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(terminalRef.current);
    fitAddon.fit();

    try {
      const { WebglAddon } = await import("@xterm/addon-webgl");
      terminal.loadAddon(new WebglAddon());
    } catch {
      // WebGL not available, canvas renderer is fine
    }

    const wsUrl = `${WS_RELAY_URL}/terminal?sandbox=${encodeURIComponent(sandboxId)}&cols=${terminal.cols}&rows=${terminal.rows}`;
    const ws = new WebSocket(wsUrl);

    ws.onopen = () => {
      setStatus("connected");
    };

    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data);
        switch (msg.type) {
          case "output":
            terminal.write(msg.data);
            break;
          case "exit":
            terminal.write(`\r\n[Process exited: code=${msg.exitCode}]\r\n`);
            break;
          case "error":
            setStatus("error");
            setErrorMsg(msg.data);
            terminal.write(`\r\n[Error: ${msg.data}]\r\n`);
            break;
        }
      } catch {
        terminal.write(event.data);
      }
    };

    ws.onerror = () => {
      setStatus("error");
      setErrorMsg("WebSocket connection failed");
    };

    ws.onclose = () => {
      terminal.write("\r\n[Connection closed]\r\n");
    };

    terminal.onData((data) => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: "input", data }));
      }
    });

    terminal.onResize(({ cols, rows }) => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: "resize", cols, rows }));
      }
    });

    const resizeObserver = new ResizeObserver(() => {
      fitAddon.fit();
    });
    resizeObserver.observe(terminalRef.current!);

    cleanupRef.current = () => {
      resizeObserver.disconnect();
      ws.close();
      terminal.dispose();
    };
  }, [sandboxId]);

  useEffect(() => {
    connect();
    return () => {
      cleanupRef.current?.();
    };
  }, [connect]);

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between px-4 py-2 border-b border-border">
        <span className="text-xs text-text-3 font-mono">
          ws-relay → agentd /v1/terminal
        </span>
        <span
          className={`text-xs ${
            status === "connected"
              ? "text-status-green"
              : status === "error"
                ? "text-status-red"
                : "text-status-amber"
          }`}
        >
          {status}
        </span>
      </div>
      {status === "error" && errorMsg && (
        <div className="px-4 py-3 bg-status-red/10 border-b border-status-red/20 text-sm text-status-red">
          {errorMsg}
        </div>
      )}
      <div ref={terminalRef} className="flex-1 min-h-0 bg-bg-1" />
    </div>
  );
}
