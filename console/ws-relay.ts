import { WebSocketServer, WebSocket } from "ws";
import { createServer } from "http";

const RELAY_PORT = parseInt(process.env.WS_RELAY_PORT ?? "3101", 10);
const BFF_URL = process.env.BFF_URL ?? "http://localhost:3000";

const server = createServer((_req, res) => {
  res.writeHead(200, { "Content-Type": "application/json" });
  res.end(JSON.stringify({ status: "ws-relay", port: RELAY_PORT }));
});

const wss = new WebSocketServer({ noServer: true });

server.on("upgrade", (req, socket, head) => {
  const url = new URL(req.url!, `http://localhost:${RELAY_PORT}`);

  if (url.pathname === "/terminal") {
    wss.handleUpgrade(req, socket, head, (ws) => {
      handleTerminalRelay(ws, url.searchParams);
    });
  } else if (url.pathname === "/vnc") {
    wss.handleUpgrade(req, socket, head, (ws) => {
      handleVncRelay(ws, url.searchParams);
    });
  } else {
    socket.destroy();
  }
});

async function resolveProxyUrl(sandboxId: string): Promise<string> {
  const bffRes = await fetch(
    `${BFF_URL}/api/sandboxes/${encodeURIComponent(sandboxId)}`,
  );

  if (bffRes.ok) {
    const data = await bffRes.json();
    const url = data.connection?.url;
    if (url) return url.replace(/\/$/, "");
  }

  throw new Error(`sandbox ${sandboxId}: cannot resolve proxy URL`);
}

async function handleTerminalRelay(clientWs: WebSocket, params: URLSearchParams) {
  const sandboxId = params.get("sandbox");

  if (!sandboxId) {
    clientWs.send(JSON.stringify({ type: "error", data: "missing sandbox param" }));
    clientWs.close();
    return;
  }

  let proxyUrl: string;
  try {
    proxyUrl = await resolveProxyUrl(sandboxId);
  } catch (err) {
    const msg = err instanceof Error ? err.message : "failed to resolve sandbox";
    clientWs.send(JSON.stringify({ type: "error", data: msg }));
    clientWs.close();
    return;
  }

  let cmdId: string;
  try {
    const execRes = await fetch(`${proxyUrl}/v1/exec`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        cmd: "/bin/bash",
        args: ["-l"],
        detached: true,
        env: { TERM: "xterm-256color" },
      }),
    });

    if (!execRes.ok) {
      const text = await execRes.text().catch(() => "");
      throw new Error(`exec failed: ${execRes.status} ${text}`);
    }

    const execData = await execRes.json();
    cmdId = execData.cmd_id;
    console.log(`[ws-relay] exec started: sandbox=${sandboxId} cmd_id=${cmdId}`);
  } catch (err) {
    const msg = err instanceof Error ? err.message : "exec spawn failed";
    clientWs.send(JSON.stringify({ type: "error", data: msg }));
    clientWs.close();
    return;
  }

  const logsWsUrl = `${proxyUrl.replace(/^http/, "ws")}/v1/exec/${cmdId}/logs`;
  const logsWs = new WebSocket(logsWsUrl);

  logsWs.on("open", () => {
    console.log(`[ws-relay] logs connected: sandbox=${sandboxId} cmd_id=${cmdId}`);
    clientWs.send(JSON.stringify({ type: "output", data: "" }));
  });

  logsWs.on("message", (raw) => {
    if (clientWs.readyState !== WebSocket.OPEN) return;

    try {
      const frame = JSON.parse(raw.toString());

      if (frame.stream === "stdout" || frame.stream === "stderr") {
        const decoded = Buffer.from(frame.data, "base64").toString("utf-8");
        clientWs.send(JSON.stringify({ type: "output", data: decoded }));
      } else if (frame.stream === "exit") {
        clientWs.send(JSON.stringify({ type: "exit", exitCode: frame.code }));
        clientWs.close();
      } else if (frame.stream === "error") {
        clientWs.send(JSON.stringify({ type: "error", data: frame.code }));
      }
    } catch {
      // Non-JSON frame — forward as raw output
      clientWs.send(JSON.stringify({ type: "output", data: raw.toString() }));
    }
  });

  clientWs.on("message", async (raw) => {
    try {
      const msg = JSON.parse(raw.toString());

      if (msg.type === "input" && msg.data) {
        await fetch(`${proxyUrl}/v1/exec/${cmdId}/stdin`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ data: msg.data }),
        });
      }
      // resize is not supported by the current agentd exec API
    } catch (err) {
      console.error(`[ws-relay] stdin error:`, err instanceof Error ? err.message : err);
    }
  });

  clientWs.on("close", () => {
    logsWs.close();
    fetch(`${proxyUrl}/v1/exec/${cmdId}/signal`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ signal: "KILL" }),
    }).catch((err) => {
    // Best-effort cleanup; sandbox TTL will eventually reap the process
    console.warn(`[ws-relay] signal KILL failed for cmd_id=${cmdId}:`, err instanceof Error ? err.message : err);
  });
  });

  logsWs.on("close", () => {
    if (clientWs.readyState === WebSocket.OPEN) {
      clientWs.send(JSON.stringify({ type: "exit", exitCode: 0 }));
      clientWs.close();
    }
  });

  logsWs.on("error", (err) => {
    console.error(`[ws-relay] logs error: sandbox=${sandboxId}:`, err.message);
    if (clientWs.readyState === WebSocket.OPEN) {
      clientWs.send(JSON.stringify({ type: "error", data: `logs: ${err.message}` }));
      clientWs.close();
    }
  });

  clientWs.on("error", (err) => {
    console.error(`[ws-relay] client error: sandbox=${sandboxId}:`, err.message);
    logsWs.close();
  });
}

async function handleVncRelay(clientWs: WebSocket, params: URLSearchParams) {
  const sandboxId = params.get("sandbox");

  if (!sandboxId) {
    clientWs.close(1008, "missing sandbox param");
    return;
  }

  let proxyUrl: string;
  try {
    proxyUrl = await resolveProxyUrl(sandboxId);
  } catch (err) {
    const msg = err instanceof Error ? err.message : "failed to resolve sandbox";
    clientWs.close(1011, msg);
    return;
  }

  const upstreamUrl = `${proxyUrl.replace(/^http/, "ws")}/v1/vnc`;
  const upstreamWs = new WebSocket(upstreamUrl);
  upstreamWs.binaryType = "arraybuffer";

  upstreamWs.on("open", () => {
    console.log(`[ws-relay] vnc connected: sandbox=${sandboxId}`);
  });

  upstreamWs.on("message", (data: Buffer | ArrayBuffer, isBinary: boolean) => {
    if (clientWs.readyState !== WebSocket.OPEN) return;
    clientWs.send(data, { binary: isBinary });
  });

  clientWs.on("message", (data: Buffer | ArrayBuffer, isBinary: boolean) => {
    if (upstreamWs.readyState !== WebSocket.OPEN) return;
    upstreamWs.send(data, { binary: isBinary });
  });

  clientWs.on("close", () => {
    upstreamWs.close();
  });

  upstreamWs.on("close", (code: number, reason: Buffer) => {
    if (clientWs.readyState !== WebSocket.OPEN) return;
    const safeCode = code >= 1000 && code <= 4999 && code !== 1005 && code !== 1006 ? code : 1011;
    clientWs.close(safeCode, reason.toString().slice(0, 123));
  });

  upstreamWs.on("error", (err: Error) => {
    console.error(`[ws-relay] vnc upstream error: sandbox=${sandboxId}:`, err.message);
    if (clientWs.readyState === WebSocket.OPEN) {
      clientWs.close(1011, `upstream: ${err.message}`);
    }
  });

  clientWs.on("error", (err: Error) => {
    console.error(`[ws-relay] vnc client error: sandbox=${sandboxId}:`, err.message);
    upstreamWs.close();
  });
}

server.listen(RELAY_PORT, () => {
  console.log(`[ws-relay] listening on :${RELAY_PORT}`);
});
