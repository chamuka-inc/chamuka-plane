import { WebSocketServer, WebSocket } from "ws";
import { createServer } from "http";

const RELAY_PORT = parseInt(process.env.WS_RELAY_PORT ?? "3101", 10);
const CONTROLLER_URL = process.env.CONTROLLER_URL ?? "http://localhost:8080";
const AUTH_TENANT = process.env.AUTH_TENANT ?? "tenant-default";
const AUTH_USER = process.env.AUTH_USER ?? "admin";

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
  } else {
    socket.destroy();
  }
});

async function resolveAgentdUrl(sandboxId: string): Promise<string> {
  const res = await fetch(`${CONTROLLER_URL}/pub/v1/sandboxes/${sandboxId}`, {
    headers: {
      "X-Auth-Tenant": AUTH_TENANT,
      "X-Auth-User": AUTH_USER,
    },
  });

  if (!res.ok) {
    throw new Error(`sandbox ${sandboxId}: controller returned ${res.status}`);
  }

  const sandbox = await res.json();
  const proxyBaseUrl = sandbox.connection?.url ?? sandbox.proxy_url;

  if (!proxyBaseUrl) {
    throw new Error(`sandbox ${sandboxId}: no connection URL (status: ${sandbox.status})`);
  }

  return proxyBaseUrl.replace(/^http/, "ws");
}

async function handleTerminalRelay(clientWs: WebSocket, params: URLSearchParams) {
  const sandboxId = params.get("sandbox");
  const cols = params.get("cols") ?? "80";
  const rows = params.get("rows") ?? "24";

  if (!sandboxId) {
    clientWs.send(JSON.stringify({ type: "error", data: "missing sandbox param" }));
    clientWs.close();
    return;
  }

  let agentdWsUrl: string;
  try {
    const baseWsUrl = await resolveAgentdUrl(sandboxId);
    agentdWsUrl = `${baseWsUrl}v1/terminal?cols=${cols}&rows=${rows}`;
  } catch (err) {
    clientWs.send(JSON.stringify({
      type: "error",
      data: err instanceof Error ? err.message : "failed to resolve sandbox",
    }));
    clientWs.close();
    return;
  }

  const agentdWs = new WebSocket(agentdWsUrl);

  agentdWs.on("open", () => {
    console.log(`[ws-relay] connected to agentd for sandbox=${sandboxId}`);
  });

  agentdWs.on("message", (data) => {
    if (clientWs.readyState === WebSocket.OPEN) {
      clientWs.send(data);
    }
  });

  clientWs.on("message", (data) => {
    if (agentdWs.readyState === WebSocket.OPEN) {
      agentdWs.send(data);
    }
  });

  clientWs.on("close", () => {
    agentdWs.close();
  });

  agentdWs.on("close", () => {
    clientWs.close();
  });

  agentdWs.on("error", (err) => {
    console.error(`[ws-relay] agentd error for sandbox=${sandboxId}:`, err.message);
    if (clientWs.readyState === WebSocket.OPEN) {
      clientWs.send(JSON.stringify({ type: "error", data: `agentd: ${err.message}` }));
    }
    clientWs.close();
  });

  clientWs.on("error", (err) => {
    console.error(`[ws-relay] client error for sandbox=${sandboxId}:`, err.message);
    agentdWs.close();
  });
}

server.listen(RELAY_PORT, () => {
  console.log(`[ws-relay] listening on :${RELAY_PORT}`);
});
