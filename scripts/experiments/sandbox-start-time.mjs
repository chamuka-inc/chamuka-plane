#!/usr/bin/env node
// Sandbox start-time experiment.
// Measures: control-plane scheduling latency, loading, starting, waiting, total time-to-ready.
// Subscribes to the SSE status stream for accurate per-transition server timestamps.

import { setTimeout as sleep } from "node:timers/promises";
import { writeFile } from "node:fs/promises";

const CONTROL_URL = process.env.SANDBOX_CONTROL_URL || "http://127.0.0.1:18080";
const TENANT = process.env.SANDBOX_TENANT || "exp-tenant";
const TOKEN = process.env.SANDBOX_TOKEN || "";

const RUNTIMES = (process.env.RUNTIMES || "node24,python313,agent,desktop").split(",");
const ITERATIONS = Number(process.env.ITERATIONS || "5");
const PREWARM = process.env.PREWARM !== "0";

function authHeaders(extra = {}) {
  const h = { "X-Auth-Tenant": TENANT, ...extra };
  if (TOKEN) h.Authorization = `Bearer ${TOKEN}`;
  return h;
}

async function createSandbox(runtime) {
  const body = {
    runtime,
    trust: "trusted",          // dev-fleet: 1 trusted (runc) + 2 untrusted (runc) drones
    resources: { vcpus: 1 },
    timeout_ms: 120_000,
  };
  const t0 = performance.now();
  const res = await fetch(`${CONTROL_URL}/pub/v1/sandboxes`, {
    method: "POST",
    headers: authHeaders({ "Content-Type": "application/json" }),
    body: JSON.stringify(body),
  });
  const t1 = performance.now();
  if (!res.ok) {
    const txt = await res.text();
    throw new Error(`create ${runtime} failed: HTTP ${res.status} ${txt}`);
  }
  const json = await res.json();
  return {
    id: json.sandbox.id,
    backendId: json.sandbox.backend_id,
    createRequestMs: t1 - t0,
    t0,
  };
}

async function waitForReady(sandboxId, t0, timeoutMs = 90_000) {
  // SSE: the endpoint streams status events. Each "data:" line is a
  // BackendStatusStreamEntry with .status and .time (ms epoch).
  const url = `${CONTROL_URL}/pub/v1/sandboxes/${encodeURIComponent(sandboxId)}/status?stream=true`;
  const ac = new AbortController();
  const timer = setTimeout(() => ac.abort(), timeoutMs);
  let res;
  try {
    res = await fetch(url, { headers: authHeaders({ Accept: "text/event-stream" }), signal: ac.signal });
  } catch (e) {
    clearTimeout(timer);
    throw new Error(`status-stream connect failed: ${e?.message ?? e}`);
  }
  if (!res.ok) {
    clearTimeout(timer);
    throw new Error(`status-stream HTTP ${res.status}`);
  }

  const transitions = []; // {status, clientMs, serverMs}
  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  let buf = "";

  try {
    while (true) {
      const { value, done } = await reader.read();
      if (done) break;
      buf += decoder.decode(value, { stream: true });
      let idx;
      while ((idx = buf.indexOf("\n\n")) !== -1) {
        const block = buf.slice(0, idx);
        buf = buf.slice(idx + 2);
        const line = block.split("\n").find((l) => l.startsWith("data:"));
        if (!line) continue;
        let payload;
        try { payload = JSON.parse(line.slice(5).trim()); } catch { continue; }
        transitions.push({
          status: payload.status,
          clientMs: performance.now() - t0,
          serverMs: payload.time,
        });
        if (payload.status === "ready") {
          clearTimeout(timer);
          try { await reader.cancel(); } catch {}
          return transitions;
        }
        if (payload.status === "terminated" || payload.status === "hard-terminated") {
          clearTimeout(timer);
          try { await reader.cancel(); } catch {}
          throw new Error(`sandbox terminated before ready: ${JSON.stringify(payload)}`);
        }
      }
    }
  } finally {
    clearTimeout(timer);
  }
  throw new Error(`status stream ended before ready (saw: ${transitions.map((t) => t.status).join(",")})`);
}

async function deleteSandbox(sandboxId) {
  try {
    await fetch(`${CONTROL_URL}/pub/v1/sandboxes/${encodeURIComponent(sandboxId)}`, {
      method: "DELETE",
      headers: authHeaders(),
    });
  } catch {/* best effort */}
}

async function runOnce(runtime) {
  const c = await createSandbox(runtime);
  try {
    const transitions = await waitForReady(c.id, c.t0);
    const tReady = transitions.find((t) => t.status === "ready");
    const serverMs = Object.fromEntries(transitions.map((t) => [t.status, t.serverMs]));
    const phase = (a, b) =>
      serverMs[a] != null && serverMs[b] != null ? serverMs[b] - serverMs[a] : null;
    return {
      runtime,
      sandboxId: c.id,
      createRequestMs: c.createRequestMs,
      totalMs: tReady.clientMs,
      serverTotalMs: serverMs.ready != null && serverMs.scheduled != null ? serverMs.ready - serverMs.scheduled : null,
      transitions: transitions.map((t) => ({ status: t.status, clientMs: Math.round(t.clientMs) })),
      serverPhases: {
        scheduledToLoading: phase("scheduled", "loading"),
        loadingToStarting:  phase("loading", "starting"),
        startingToWaiting:  phase("starting", "waiting"),
        waitingToReady:     phase("waiting", "ready"),
      },
    };
  } finally {
    await deleteSandbox(c.id);
  }
}

function pct(arr, p) {
  if (arr.length === 0) return null;
  const sorted = [...arr].sort((a, b) => a - b);
  const i = Math.min(sorted.length - 1, Math.floor((p / 100) * sorted.length));
  return sorted[i];
}

async function main() {
  console.log(`# Sandbox start-time experiment`);
  console.log(`# control_url=${CONTROL_URL} tenant=${TENANT}`);
  console.log(`# runtimes=${RUNTIMES.join(",")} iterations=${ITERATIONS} prewarm=${PREWARM}\n`);

  const allRuns = {};

  for (const runtime of RUNTIMES) {
    allRuns[runtime] = [];
    if (PREWARM) {
      process.stdout.write(`[${runtime}] prewarm ... `);
      try {
        const r = await runOnce(runtime);
        console.log(`ok (${r.totalMs.toFixed(0)} ms)`);
      } catch (e) {
        console.log(`SKIP prewarm: ${e.message}`);
      }
    }
    for (let i = 1; i <= ITERATIONS; i++) {
      process.stdout.write(`[${runtime}] iter ${i}/${ITERATIONS} ... `);
      try {
        const r = await runOnce(runtime);
        allRuns[runtime].push(r);
        const sp = r.serverPhases;
        console.log(
          `ready in ${r.totalMs.toFixed(0)} ms (create=${r.createRequestMs.toFixed(0)} ` +
            `srvTotal=${fmt(r.serverTotalMs)} ` +
            `sched→load=${fmt(sp.scheduledToLoading)} ` +
            `load→start=${fmt(sp.loadingToStarting)} ` +
            `start→wait=${fmt(sp.startingToWaiting)} ` +
            `wait→ready=${fmt(sp.waitingToReady)})`,
        );
      } catch (e) {
        console.log(`FAIL: ${e.message}`);
      }
    }
  }

  console.log(`\n## Summary — total time-to-ready (client, ms)\n`);
  const header = `runtime       n     min   p50   p95   max   mean`;
  console.log(header);
  console.log("-".repeat(header.length));
  for (const runtime of RUNTIMES) {
    const totals = allRuns[runtime].map((r) => r.totalMs);
    if (totals.length === 0) { console.log(`${runtime.padEnd(13)}  0   (no successful runs)`); continue; }
    const s = {
      n: totals.length,
      min: Math.min(...totals),
      p50: pct(totals, 50),
      p95: pct(totals, 95),
      max: Math.max(...totals),
      mean: totals.reduce((a, b) => a + b, 0) / totals.length,
    };
    console.log(
      `${runtime.padEnd(13)} ${String(s.n).padStart(2)}   ${pad(s.min)} ${pad(s.p50)} ${pad(s.p95)} ${pad(s.max)} ${pad(s.mean)}`,
    );
  }

  console.log(`\n## Server-side phase medians (ms)\n`);
  console.log(`runtime       create  sched→load  load→start  start→wait  wait→ready  srvTotal`);
  console.log("-".repeat(86));
  for (const runtime of RUNTIMES) {
    const runs = allRuns[runtime];
    if (runs.length === 0) continue;
    const med = (key, type) => {
      const arr = (type === "server" ? runs.map((r) => r.serverPhases[key]) : runs.map((r) => r[key])).filter((v) => v != null);
      return arr.length ? pct(arr, 50) : null;
    };
    console.log(
      `${runtime.padEnd(13)} ${pad(med("createRequestMs", "root"))}   ` +
      `${pad(med("scheduledToLoading", "server"))}       ${pad(med("loadingToStarting", "server"))}       ` +
      `${pad(med("startingToWaiting", "server"))}       ${pad(med("waitingToReady", "server"))}       ` +
      `${pad(med("serverTotalMs", "root"))}`,
    );
  }
  const out = `/tmp/sandbox-start-time-${Date.now()}.json`;
  await writeFile(out, JSON.stringify(allRuns, null, 2));
  console.log(`\nraw data: ${out}`);
}

function fmt(v) { return v == null ? "?" : Math.round(v) + ""; }
function pad(v) { return v == null ? "  -  " : String(Math.round(v)).padStart(5); }

main().catch((e) => { console.error(e); process.exit(1); });
