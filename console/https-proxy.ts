import { createServer } from "https";
import { readFileSync } from "fs";
import { join } from "path";
import { request as httpRequest } from "http";
import type { IncomingMessage, ServerResponse } from "http";

const HTTPS_PORT = parseInt(process.env.HTTPS_PORT ?? "3443", 10);
const NEXT_PORT = parseInt(process.env.NEXT_PORT ?? "3000", 10);
const RELAY_PORT = parseInt(process.env.WS_RELAY_PORT ?? "3101", 10);

const certsDir = join(import.meta.dirname, "certificates");
const cert = readFileSync(join(certsDir, "localhost.pem"));
const key = readFileSync(join(certsDir, "localhost-key.pem"));

const RELAY_PATHS = new Set(["/vnc", "/terminal"]);

function upstreamPort(path: string) {
  const segment = path.split("?")[0];
  return RELAY_PATHS.has(segment) ? RELAY_PORT : NEXT_PORT;
}

function proxyHttp(clientReq: IncomingMessage, clientRes: ServerResponse) {
  const port = upstreamPort(clientReq.url ?? "/");
  const proxyReq = httpRequest(
    { hostname: "127.0.0.1", port, path: clientReq.url, method: clientReq.method, headers: clientReq.headers },
    (proxyRes) => {
      clientRes.writeHead(proxyRes.statusCode!, proxyRes.headers);
      proxyRes.pipe(clientRes, { end: true });
    },
  );
  proxyReq.on("error", (err) => {
    clientRes.writeHead(502);
    clientRes.end(`upstream error: ${err.message}`);
  });
  clientReq.pipe(proxyReq, { end: true });
}

const server = createServer({ cert, key }, proxyHttp);

server.on("upgrade", (req, socket, head) => {
  const port = upstreamPort(req.url ?? "/");
  const proxyReq = httpRequest({
    hostname: "127.0.0.1",
    port,
    path: req.url,
    method: "GET",
    headers: req.headers,
  });

  proxyReq.on("upgrade", (proxyRes, proxySocket, proxyHead) => {
    const headers = ["HTTP/1.1 101 Switching Protocols"];
    for (const [k, v] of Object.entries(proxyRes.headers)) {
      if (v) headers.push(`${k}: ${Array.isArray(v) ? v.join(", ") : v}`);
    }
    socket.write(headers.join("\r\n") + "\r\n\r\n");
    if (proxyHead.length > 0) socket.write(proxyHead);
    if (head.length > 0) proxySocket.write(head);

    proxySocket.pipe(socket);
    socket.pipe(proxySocket);

    socket.on("error", () => proxySocket.destroy());
    socket.on("close", () => proxySocket.destroy());
    proxySocket.on("error", () => socket.destroy());
    proxySocket.on("close", () => socket.destroy());
  });

  proxyReq.on("error", () => socket.end());
  proxyReq.end();
});

server.listen(HTTPS_PORT, "0.0.0.0", () => {
  console.log(`[https-proxy] https://0.0.0.0:${HTTPS_PORT} -> next::${NEXT_PORT} + relay::${RELAY_PORT}`);
});
