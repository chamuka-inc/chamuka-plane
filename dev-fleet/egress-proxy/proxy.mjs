import http from "node:http";
import https from "node:https";
import net from "node:net";
import tls from "node:tls";
import { URL } from "node:url";
import fs from "node:fs";
import crypto from "node:crypto";
import { execSync } from "node:child_process";

const PORT = parseInt(process.env.PROXY_PORT ?? "3128", 10);
const TRANSPARENT_PORT = parseInt(process.env.TRANSPARENT_PORT ?? "3130", 10);
const TRANSPARENT_HTTPS_PORT = parseInt(process.env.TRANSPARENT_HTTPS_PORT ?? "3131", 10);
const ADMIN_PORT = parseInt(process.env.ADMIN_PORT ?? "3129", 10);
const CA_DIR = process.env.CA_DIR ?? "/app/ca";
const MAX_HEADER_SIZE = 65536;

const credentials = new Map();

function formatHeader(format, value) {
  return format.replace("{value}", value);
}

function findCredential(hostname) {
  if (credentials.has(hostname)) return credentials.get(hostname);
  const bare = hostname.replace(/:\d+$/, "");
  if (credentials.has(bare)) return credentials.get(bare);
  return null;
}

// --- CA certificate for HTTPS interception ---

let caCert, caKey;
const certCache = new Map();

function ensureCA() {
  const certPath = `${CA_DIR}/ca.crt`;
  const keyPath = `${CA_DIR}/ca.key`;

  if (fs.existsSync(certPath) && fs.existsSync(keyPath)) {
    caCert = fs.readFileSync(certPath, "utf8");
    caKey = fs.readFileSync(keyPath, "utf8");
    console.log("[ca] loaded existing CA certificate");
    return;
  }

  fs.mkdirSync(CA_DIR, { recursive: true });

  execSync(
    `openssl req -new -x509 -days 3650 -nodes ` +
      `-keyout "${keyPath}" -out "${certPath}" ` +
      `-subj "/CN=Sandbox Egress Proxy CA" ` +
      `-addext "basicConstraints=critical,CA:TRUE" ` +
      `-addext "keyUsage=critical,keyCertSign,cRLSign"`,
    { stdio: "pipe" },
  );

  caCert = fs.readFileSync(certPath, "utf8");
  caKey = fs.readFileSync(keyPath, "utf8");
  console.log("[ca] generated new CA certificate");
}

function generateCertForHost(hostname) {
  if (certCache.has(hostname)) return certCache.get(hostname);

  const keyPair = crypto.generateKeyPairSync("rsa", { modulusLength: 2048 });
  const privKey = keyPair.privateKey.export({ type: "pkcs8", format: "pem" });

  const tmpDir = fs.mkdtempSync("/tmp/cert-");
  const csrPath = `${tmpDir}/host.csr`;
  const certPath = `${tmpDir}/host.crt`;
  const keyPath = `${tmpDir}/host.key`;
  const extPath = `${tmpDir}/ext.cnf`;

  fs.writeFileSync(keyPath, privKey);
  fs.writeFileSync(
    extPath,
    `subjectAltName=DNS:${hostname}\nbasicConstraints=CA:FALSE\nkeyUsage=digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth`,
  );

  execSync(
    `openssl req -new -key "${keyPath}" -out "${csrPath}" -subj "/CN=${hostname}"`,
    { stdio: "pipe" },
  );

  execSync(
    `openssl x509 -req -in "${csrPath}" -CA "${CA_DIR}/ca.crt" -CAkey "${CA_DIR}/ca.key" ` +
      `-CAcreateserial -out "${certPath}" -days 365 -extfile "${extPath}"`,
    { stdio: "pipe" },
  );

  const cert = fs.readFileSync(certPath, "utf8");
  const result = { cert, key: privKey };
  certCache.set(hostname, result);

  fs.rmSync(tmpDir, { recursive: true, force: true });
  return result;
}

ensureCA();

// --- Shared TLS interception: parse HTTP from decrypted stream, inject, forward ---

function handleInterceptedRequest(tlsSocket, hostname, port, cred, logTag) {
  let requestData = "";
  let headersDone = false;

  tlsSocket.on("data", (chunk) => {
    if (headersDone) return;

    requestData += chunk.toString();
    if (requestData.length > MAX_HEADER_SIZE) {
      console.error(`[${logTag}] headers exceed ${MAX_HEADER_SIZE} bytes for ${hostname}, closing`);
      tlsSocket.destroy();
      return;
    }

    const headerEnd = requestData.indexOf("\r\n\r\n");
    if (headerEnd === -1) return;

    headersDone = true;
    const headerSection = requestData.slice(0, headerEnd);
    const bodyStart = requestData.slice(headerEnd + 4);
    const lines = headerSection.split("\r\n");
    const [method, path, httpVersion] = lines[0].split(" ");

    const headers = {};
    for (let i = 1; i < lines.length; i++) {
      const colon = lines[i].indexOf(":");
      if (colon > 0) {
        headers[lines[i].slice(0, colon).trim().toLowerCase()] = lines[i].slice(colon + 1).trim();
      }
    }

    const headerValue = formatHeader(cred.header_format, cred.value);
    headers[cred.header_name.toLowerCase()] = headerValue;
    console.log(
      `[${logTag}] ${cred.header_name}: ${cred.header_format.replace("{value}", "***")} -> ${hostname}:${port}${path}`,
    );

    const contentLength = parseInt(headers["content-length"] || "0", 10);
    const hasBody = method !== "GET" && method !== "HEAD" && contentLength > 0;

    const outReq = https.request(
      { hostname, port, path, method, headers, rejectUnauthorized: true },
      (outRes) => {
        let statusLine = `${httpVersion} ${outRes.statusCode} ${outRes.statusMessage}\r\n`;
        let respHeaders = "";
        for (const [k, v] of Object.entries(outRes.headers)) {
          if (Array.isArray(v)) {
            for (const val of v) respHeaders += `${k}: ${val}\r\n`;
          } else {
            respHeaders += `${k}: ${v}\r\n`;
          }
        }
        tlsSocket.write(statusLine + respHeaders + "\r\n");
        outRes.pipe(tlsSocket);
      },
    );

    outReq.on("error", (err) => {
      console.error(`[${logTag}] upstream error for ${hostname}: ${err.message}`);
      tlsSocket.write(`${httpVersion} 502 Bad Gateway\r\nContent-Length: 11\r\n\r\nBad gateway`);
      tlsSocket.end();
    });

    if (hasBody) {
      if (bodyStart.length > 0) outReq.write(bodyStart);
      let remaining = contentLength - bodyStart.length;
      if (remaining > 0) {
        tlsSocket.on("data", function bodyReader(chunk) {
          outReq.write(chunk);
          remaining -= chunk.length;
          if (remaining <= 0) {
            tlsSocket.removeListener("data", bodyReader);
            outReq.end();
          }
        });
      } else {
        outReq.end();
      }
    } else {
      outReq.end();
    }
  });
}

// --- Forward HTTP proxy (explicit mode via HTTP_PROXY) ---

const proxy = http.createServer((clientReq, clientRes) => {
  let targetUrl;
  try {
    targetUrl = new URL(clientReq.url);
  } catch {
    clientRes.writeHead(400);
    clientRes.end("Bad request URL");
    return;
  }

  proxyHttpRequest(targetUrl, clientReq, clientRes);
});

function proxyHttpRequest(targetUrl, clientReq, clientRes) {
  const hostname = targetUrl.hostname;
  const port = parseInt(targetUrl.port || "80", 10);
  const path = targetUrl.pathname + targetUrl.search;

  const headers = { ...clientReq.headers };
  delete headers["proxy-authorization"];
  delete headers["proxy-connection"];
  headers.host = targetUrl.host;

  const cred = findCredential(targetUrl.host) ?? findCredential(hostname);
  if (cred) {
    const headerValue = formatHeader(cred.header_format, cred.value);
    headers[cred.header_name.toLowerCase()] = headerValue;
    console.log(
      `[inject] ${cred.header_name}: ${cred.header_format.replace("{value}", "***")} -> ${targetUrl.host}${path}`,
    );
  }

  const proxyReq = http.request(
    { hostname, port, path, method: clientReq.method, headers },
    (proxyRes) => {
      clientRes.writeHead(proxyRes.statusCode, proxyRes.headers);
      proxyRes.pipe(clientRes, { end: true });
    },
  );

  proxyReq.on("error", (err) => {
    console.error(`[proxy] error connecting to ${hostname}:${port}: ${err.message}`);
    clientRes.writeHead(502);
    clientRes.end("Bad gateway");
  });

  clientReq.pipe(proxyReq, { end: true });
}

// CONNECT handling — intercept if credential exists, otherwise tunnel
proxy.on("connect", (req, clientSocket, head) => {
  const [hostname, portStr] = req.url.split(":");
  const port = parseInt(portStr || "443", 10);

  const cred = findCredential(`${hostname}:${port}`) ?? findCredential(hostname);

  if (cred) {
    interceptTls(hostname, port, clientSocket, cred);
  } else {
    tunnelTls(hostname, port, clientSocket, head);
  }
});

function tunnelTls(hostname, port, clientSocket, head) {
  const serverSocket = net.connect(port, hostname, () => {
    clientSocket.write("HTTP/1.1 200 Connection Established\r\n\r\n");
    serverSocket.write(head);
    serverSocket.pipe(clientSocket);
    clientSocket.pipe(serverSocket);
  });
  serverSocket.on("error", (err) => {
    console.error(`[tunnel] error: ${err.message}`);
    clientSocket.end();
  });
}

function interceptTls(hostname, port, clientSocket, cred) {
  const hostCert = generateCertForHost(hostname);

  clientSocket.write("HTTP/1.1 200 Connection Established\r\n\r\n");

  const tlsSocket = new tls.TLSSocket(clientSocket, {
    isServer: true,
    cert: hostCert.cert,
    key: hostCert.key,
    ca: caCert,
  });

  tlsSocket.on("error", (err) => {
    console.error(`[intercept-tls] client error for ${hostname}: ${err.message}`);
    tlsSocket.destroy();
  });

  handleInterceptedRequest(tlsSocket, hostname, port, cred, "inject-tls");
}

proxy.listen(PORT, "0.0.0.0", () => {
  console.log(`[egress-proxy] forward proxy on :${PORT}`);
});

// --- Transparent proxy (for iptables DNAT redirect) ---

const transparentProxy = http.createServer((clientReq, clientRes) => {
  const host = clientReq.headers.host;
  if (!host) {
    clientRes.writeHead(400);
    clientRes.end("Missing Host header");
    return;
  }

  const [hostname, portStr] = host.split(":");
  const port = parseInt(portStr || "80", 10);
  const targetUrl = new URL(`http://${hostname}:${port}${clientReq.url}`);

  proxyHttpRequest(targetUrl, clientReq, clientRes);
});

transparentProxy.listen(TRANSPARENT_PORT, "0.0.0.0", () => {
  console.log(`[egress-proxy] transparent proxy on :${TRANSPARENT_PORT}`);
});

// --- Transparent HTTPS proxy (for iptables DNAT redirect of port 443) ---

function extractSNI(data) {
  if (data[0] !== 0x16) return null;
  let pos = 5;
  if (data[pos] !== 0x01) return null;
  pos += 4;
  pos += 2 + 32;
  const sessionIdLen = data[pos]; pos += 1 + sessionIdLen;
  const cipherSuitesLen = (data[pos] << 8) | data[pos + 1]; pos += 2 + cipherSuitesLen;
  const compressionLen = data[pos]; pos += 1 + compressionLen;
  if (pos + 2 > data.length) return null;
  const extensionsLen = (data[pos] << 8) | data[pos + 1]; pos += 2;
  const extensionsEnd = pos + extensionsLen;
  while (pos + 4 <= extensionsEnd) {
    const extType = (data[pos] << 8) | data[pos + 1]; pos += 2;
    const extLen = (data[pos] << 8) | data[pos + 1]; pos += 2;
    if (extType === 0x0000) {
      let snPos = pos + 2;
      while (snPos + 3 < pos + extLen) {
        const nameType = data[snPos]; snPos += 1;
        const nameLen = (data[snPos] << 8) | data[snPos + 1]; snPos += 2;
        if (nameType === 0) return data.subarray(snPos, snPos + nameLen).toString("ascii");
        snPos += nameLen;
      }
    }
    pos += extLen;
  }
  return null;
}

const transparentTlsServer = net.createServer((clientSocket) => {
  clientSocket.once("data", (initialData) => {
    const sni = extractSNI(initialData);
    if (!sni) {
      console.error("[transparent-tls] no SNI found, closing connection");
      clientSocket.end();
      return;
    }

    const cred = findCredential(`${sni}:443`) ?? findCredential(sni);
    if (!cred) {
      const upstream = net.connect(443, sni, () => {
        upstream.write(initialData);
        upstream.pipe(clientSocket);
        clientSocket.pipe(upstream);
      });
      upstream.on("error", (err) => {
        console.error(`[transparent-tls] passthrough error for ${sni}: ${err.message}`);
        clientSocket.end();
      });
      return;
    }

    const hostCert = generateCertForHost(sni);
    const tlsSocket = new tls.TLSSocket(clientSocket, {
      isServer: true,
      cert: hostCert.cert,
      key: hostCert.key,
      ca: caCert,
    });

    clientSocket.unshift(initialData);

    tlsSocket.on("error", (err) => {
      console.error(`[transparent-tls] TLS error for ${sni}: ${err.message}`);
      tlsSocket.destroy();
    });

    handleInterceptedRequest(tlsSocket, sni, 443, cred, "inject-transparent-tls");
  });
});

transparentTlsServer.listen(TRANSPARENT_HTTPS_PORT, "0.0.0.0", () => {
  console.log(`[egress-proxy] transparent HTTPS proxy on :${TRANSPARENT_HTTPS_PORT}`);
});

// --- Admin API for credential management ---

const admin = http.createServer((req, res) => {
  const url = new URL(req.url, `http://localhost:${ADMIN_PORT}`);

  if (req.method === "GET" && url.pathname === "/admin/credentials") {
    const entries = Object.fromEntries(credentials);
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify(entries, null, 2));
    return;
  }

  if (req.method === "POST" && url.pathname === "/admin/credentials") {
    let body = "";
    req.on("data", (chunk) => (body += chunk));
    req.on("end", () => {
      try {
        const { destination, header_name, header_format, value } = JSON.parse(body);
        if (!destination || !header_name || !value) {
          res.writeHead(400, { "Content-Type": "application/json" });
          res.end(
            JSON.stringify({
              error: "destination, header_name, and value are required",
            }),
          );
          return;
        }
        credentials.set(destination, {
          header_name,
          header_format: header_format || "{value}",
          value,
        });
        console.log(`[admin] registered credential for ${destination} (${header_name})`);
        res.writeHead(200, { "Content-Type": "application/json" });
        res.end(JSON.stringify({ ok: true, destination }));
      } catch {
        res.writeHead(400, { "Content-Type": "application/json" });
        res.end(JSON.stringify({ error: "Invalid JSON" }));
      }
    });
    return;
  }

  if (req.method === "DELETE" && url.pathname.startsWith("/admin/credentials/")) {
    const destination = decodeURIComponent(
      url.pathname.slice("/admin/credentials/".length),
    );
    const deleted = credentials.delete(destination);
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ ok: true, deleted }));
    return;
  }

  if (req.method === "GET" && url.pathname === "/admin/health") {
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(
      JSON.stringify({ status: "ok", credentials_count: credentials.size }),
    );
    return;
  }

  if (req.method === "GET" && url.pathname === "/admin/ca.crt") {
    res.writeHead(200, { "Content-Type": "application/x-pem-file" });
    res.end(caCert);
    return;
  }

  res.writeHead(404);
  res.end("Not found");
});

admin.listen(ADMIN_PORT, "0.0.0.0", () => {
  console.log(`[egress-proxy] admin API on :${ADMIN_PORT}`);
});
