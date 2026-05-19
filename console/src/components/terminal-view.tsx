"use client";

import { useEffect, useRef, useState, useCallback } from "react";
import { tokens } from "@/lib/theme";
import type { ExecEntry } from "@/api/types";
import "@xterm/xterm/css/xterm.css";

type TerminalViewProps = {
  sandboxId: string;
  onCommand?: (cmd: string) => void;
  onExecComplete?: (entry: ExecEntry) => void;
};

export function TerminalView({ sandboxId, onCommand, onExecComplete }: TerminalViewProps) {
  const terminalRef = useRef<HTMLDivElement>(null);
  const [status, setStatus] = useState<"connecting" | "connected" | "error">("connecting");
  const cleanupRef = useRef<(() => void) | null>(null);
  const onCommandRef = useRef(onCommand);
  const onExecCompleteRef = useRef(onExecComplete);

  onCommandRef.current = onCommand;
  onExecCompleteRef.current = onExecComplete;

  const connect = useCallback(async () => {
    cleanupRef.current?.();
    cleanupRef.current = null;

    const container = terminalRef.current;
    if (!container) return;

    setStatus("connecting");

    const execUrl = `/api/sandboxes/${encodeURIComponent(sandboxId)}/exec`;

    let cancelled = false;

    try {
      const [{ Terminal }, { FitAddon }] = await Promise.all([
        import("@xterm/xterm"),
        import("@xterm/addon-fit"),
      ]);

      if (cancelled || !terminalRef.current) return;

      const terminal = new Terminal({
        fontFamily: "'SF Mono', 'Menlo', monospace",
        fontSize: 13,
        lineHeight: 1.4,
        theme: {
          background: tokens.bg1,
          foreground: tokens.text,
          cursor: tokens.text,
          selectionBackground: `${tokens.accentBlue}4d`,
          black: tokens.bg,
          red: tokens.statusRed,
          green: tokens.statusGreen,
          yellow: tokens.statusAmber,
          blue: tokens.accentBlue,
          magenta: tokens.accentPink,
          cyan: tokens.accentCyan,
          white: tokens.text,
        },
        cursorBlink: true,
        scrollback: 5000,
        convertEol: true,
      });

      const fitAddon = new FitAddon();
      terminal.loadAddon(fitAddon);
      terminal.open(terminalRef.current);

      // Canvas renderer only — WebGL addon can crash tabs/GPU on remount.
      requestAnimationFrame(() => {
        if (cancelled) return;
        try {
          fitAddon.fit();
        } catch {
          // container may have zero size briefly
        }
      });

      if (cancelled) {
        terminal.dispose();
        return;
      }

      setStatus("connected");

      let inputBuffer = "";
      let busy = false;
      let cwd = "/";
      let execSeq = 0;

      async function execCommand(cmd: string) {
        busy = true;
        const t0 = performance.now();
        const timestamp = new Date().toISOString();
        let stdout = "";
        let stderr = "";
        let exitCode = 0;

        try {
          const res = await fetch(execUrl, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              cmd: "/bin/bash",
              args: ["-c", `${cmd}; echo __CWD__; pwd`],
              cwd,
              timeout_ms: 30000,
              env: { TERM: "xterm-256color" },
            }),
          });

          if (cancelled) return;

          if (!res.ok) {
            stderr = `exec error: ${res.status}`;
            exitCode = res.status;
            terminal.write(`\x1b[31m${stderr}\x1b[0m\r\n`);
            const durationMs = Math.round(performance.now() - t0);
            terminal.write(`\x1b[33m${durationMs}ms · exit ${exitCode}\x1b[0m\r\n`);
            return;
          }

          const data = await res.json();
          exitCode = data.exit_code ?? 0;

          if (data.stdout) {
            const parts = data.stdout.split("__CWD__\n");
            const output = parts[0];
            stdout = output ?? "";
            if (output) terminal.write(output);
            if (parts[1]) cwd = parts[1].trim();
          }
          if (data.stderr) {
            stderr = data.stderr;
            terminal.write(`\x1b[31m${data.stderr}\x1b[0m`);
          }

          const durationMs = Math.round(performance.now() - t0);
          const exitLabel = exitCode === 0
            ? `\x1b[90m${durationMs}ms\x1b[0m`
            : `\x1b[33m${durationMs}ms · exit ${exitCode}\x1b[0m`;
          terminal.write(`\r\n${exitLabel}\r\n`);
        } catch (err) {
          if (cancelled) return;
          const msg = err instanceof Error ? err.message : "request failed";
          terminal.write(`\x1b[31m${msg}\x1b[0m\r\n`);
          stderr = msg;
          exitCode = 1;
        } finally {
          const durationMs = Math.round(performance.now() - t0);
          onExecCompleteRef.current?.({
            id: ++execSeq,
            command: cmd,
            stdout,
            stderr,
            exitCode,
            durationMs,
            timestamp,
          });
          busy = false;
        }
      }

      function writePrompt() {
        const dir = cwd === "/" ? "/" : cwd.split("/").pop() ?? cwd;
        terminal.write(`\x1b[36m${dir}\x1b[0m \x1b[32m$\x1b[0m `);
      }

      terminal.write(`\x1b[90mConnected to sandbox ${sandboxId}\x1b[0m\r\n`);
      writePrompt();

      terminal.onData((data) => {
        if (busy || cancelled) return;

        for (const ch of data) {
          if (ch === "\r") {
            terminal.write("\r\n");
            const cmd = inputBuffer.trim();
            inputBuffer = "";

            if (!cmd) {
              writePrompt();
              return;
            }

            execCommand(cmd).then(() => {
              onCommandRef.current?.(cmd);
              writePrompt();
            }).catch(() => writePrompt());
          } else if (ch === "\x7f") {
            if (inputBuffer.length > 0) {
              inputBuffer = inputBuffer.slice(0, -1);
              terminal.write("\b \b");
            }
          } else if (ch === "\x03") {
            inputBuffer = "";
            terminal.write("^C\r\n");
            writePrompt();
          } else if (ch >= " ") {
            inputBuffer += ch;
            terminal.write(ch);
          }
        }
      });

      let resizeTimer: ReturnType<typeof setTimeout>;
      let lastWidth = 0;
      let lastHeight = 0;
      const resizeObserver = new ResizeObserver(() => {
        clearTimeout(resizeTimer);
        resizeTimer = setTimeout(() => {
          if (cancelled) return;
          const { clientWidth, clientHeight } = container;
          if (clientWidth === lastWidth && clientHeight === lastHeight) return;
          lastWidth = clientWidth;
          lastHeight = clientHeight;
          if (clientWidth > 0 && clientHeight > 0) {
            try {
              fitAddon.fit();
            } catch {
              // ignore fit errors during layout transitions
            }
          }
        }, 150);
      });
      resizeObserver.observe(container);

      cleanupRef.current = () => {
        cancelled = true;
        clearTimeout(resizeTimer);
        resizeObserver.disconnect();
        terminal.dispose();
      };
    } catch {
      if (!cancelled) setStatus("error");
    }
  }, [sandboxId]);

  useEffect(() => {
    let disposed = false;

    const frame = requestAnimationFrame(() => {
      if (!disposed) void connect();
    });

    return () => {
      disposed = true;
      cancelAnimationFrame(frame);
      cleanupRef.current?.();
      cleanupRef.current = null;
    };
  }, [connect]);

  const statusColor = {
    connecting: "text-status-amber",
    connected: "text-status-green",
    error: "text-status-red",
  }[status];

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between px-4 py-2 border-b border-border">
        <span className="text-xs text-text-3 font-mono">
          agentd /v1/exec
        </span>
        <span className={`text-xs ${statusColor}`}>
          {status}
        </span>
      </div>
      <div ref={terminalRef} className="flex-1 min-h-0 bg-bg-1" />
    </div>
  );
}
