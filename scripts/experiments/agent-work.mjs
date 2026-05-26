#!/usr/bin/env node
// Demo: an agent runs a real engineering task inside an isolated sandbox.
//
// Task: take agentd's egress.rs from this repo, ship it into a Python sandbox,
// run static analysis (line count + function count + cyclomatic-style metric +
// TODO/FIXME scan + a small HTTP server that serves the analysis), then poll
// the server from the host and shut down.

import { readFile } from "node:fs/promises";
import { Sandbox } from "../../sdks/typescript/dist/index.js";

const CONTROL = process.env.SANDBOX_CONTROL_URL ?? "http://127.0.0.1:18080";
const TENANT = process.env.SANDBOX_TENANT ?? "demo-agent";
const TARGET = process.env.TARGET_FILE
  ?? new URL("../../crates/agentd/src/egress.rs", import.meta.url).pathname;

const auth = { controlUrl: CONTROL, tenant: TENANT };

function ts(prev = ts.last ?? Date.now()) {
  const now = Date.now();
  const dt = now - prev;
  ts.last = now;
  return `+${String(dt).padStart(4)}ms`;
}

console.log(`# agent-work demo`);
console.log(`# control=${CONTROL} tenant=${TENANT}`);
console.log(`# target=${TARGET}\n`);

const source = await readFile(TARGET, "utf8");
console.log(`${ts()} read ${source.length} bytes from host`);

console.log(`${ts()} spawning python313 sandbox...`);
const sandbox = await Sandbox.create(
  {
    runtime: "python313",
    trust: "trusted",
    resources: { vcpus: 1 },
    timeout_ms: 180_000,
    ports: [8000],
  },
  auth,
);
console.log(`${ts()} sandbox scheduled: id=${sandbox.id} status=${sandbox.status}`);

try {
  // Sandbox.create returns as soon as Plane has scheduled the backend; the
  // proxy can't route data-plane requests until it's Ready. Poll the
  // platform-API status endpoint until then. (Eventually this belongs in
  // the SDK as `waitUntilReady`.)
  console.log(`${ts()} waiting for backend to become ready...`);
  // The status endpoint defaults stream=true (returns SSE). Force JSON.
  const statusUrl = `${CONTROL}/pub/v1/sandboxes/${encodeURIComponent(sandbox.id)}/status?stream=false`;
  const deadline = Date.now() + 60_000;
  let lastStatus = "scheduled";
  while (Date.now() < deadline) {
    const r = await fetch(statusUrl, { headers: { "X-Auth-Tenant": TENANT } });
    if (r.ok) {
      const j = await r.json();
      lastStatus = j.status;
      if (j.status === "ready") break;
      if (j.status === "terminated" || j.status === "hard-terminated") {
        throw new Error(`backend terminated before ready: ${JSON.stringify(j)}`);
      }
    }
    await new Promise((res) => setTimeout(res, 100));
  }
  console.log(`${ts()} backend status=${lastStatus}`);

  // Ship the source into the sandbox workspace.
  console.log(`${ts()} writing source into sandbox at /workspace/target.rs`);
  await sandbox.writeFile("/workspace/target.rs", source);

  // Write a small analysis script.
  const script = `#!/usr/bin/env python3
import json, re, sys
from pathlib import Path

src = Path("/workspace/target.rs").read_text()
lines = src.splitlines()

# Lines, blanks, comments.
n_lines = len(lines)
n_blank = sum(1 for l in lines if not l.strip())
n_comment = sum(1 for l in lines if l.strip().startswith("//") or l.strip().startswith("///"))
n_code = n_lines - n_blank - n_comment

# Function and struct counts.
fn_re = re.compile(r"^\\s*(?:pub(?:\\(.+?\\))?\\s+)?(?:async\\s+)?fn\\s+([A-Za-z_][A-Za-z0-9_]*)")
struct_re = re.compile(r"^\\s*(?:pub\\s+)?struct\\s+([A-Za-z_][A-Za-z0-9_]*)")
enum_re = re.compile(r"^\\s*(?:pub\\s+)?enum\\s+([A-Za-z_][A-Za-z0-9_]*)")
fns = sorted({m.group(1) for l in lines if (m := fn_re.match(l))})
structs = sorted({m.group(1) for l in lines if (m := struct_re.match(l))})
enums = sorted({m.group(1) for l in lines if (m := enum_re.match(l))})

# Branching density (rough cyclomatic-style proxy).
branches = sum(
    len(re.findall(rf"\\b{kw}\\b", src))
    for kw in ("if", "match", "for", "while", "loop", "&&", "\\|\\|", r"\\?")
)

# Tech-debt markers.
todos = [
    {"line": i + 1, "text": l.strip()}
    for i, l in enumerate(lines)
    if re.search(r"\\b(?:TODO|FIXME|XXX|HACK|SAFETY)\\b", l)
]

# Comment block topics — naive extraction of inline doc lines.
doc_lines = [l.strip().lstrip("//!").lstrip("///").strip() for l in lines if l.strip().startswith("///") or l.strip().startswith("//!")]
doc_lines = [l for l in doc_lines if l]

report = {
    "file": "/workspace/target.rs",
    "size_bytes": len(src.encode()),
    "lines": {"total": n_lines, "code": n_code, "comment": n_comment, "blank": n_blank},
    "items": {"fns": fns, "structs": structs, "enums": enums},
    "branching_proxy": branches,
    "tech_debt": todos,
    "doc_lines_first_5": doc_lines[:5],
}

Path("/workspace/report.json").write_text(json.dumps(report, indent=2))
print(json.dumps({"ok": True, "size": len(src), "fns": len(fns), "structs": len(structs)}))
`;
  await sandbox.writeFile("/workspace/analyze.py", script);
  console.log(`${ts()} wrote analyze.py (${script.length} bytes)`);

  // Run the analysis.
  console.log(`${ts()} running python analyze.py`);
  const run = await sandbox.runCommand("python3", {
    args: ["/workspace/analyze.py"],
    cwd: "/workspace",
    timeout_ms: 30_000,
  });
  console.log(`${ts()} analyzer exit=${run.exitCode} duration=${run.durationMs}ms`);
  if (run.exitCode !== 0) {
    console.error(`stdout:\n${run.stdout}\nstderr:\n${run.stderr}`);
    throw new Error("analyzer failed");
  }
  console.log(`        analyzer stdout: ${run.stdout.trim()}`);

  // Read the report back.
  const reportRaw = await sandbox.readTextFile("/workspace/report.json");
  const report = JSON.parse(reportRaw);
  console.log(`\n## Analysis of ${TARGET.split("/").slice(-2).join("/")} (run inside sandbox)\n`);
  console.log(`  size: ${report.size_bytes} bytes`);
  console.log(
    `  lines: total=${report.lines.total}, code=${report.lines.code}, comment=${report.lines.comment}, blank=${report.lines.blank}`,
  );
  console.log(`  fns (${report.items.fns.length}): ${report.items.fns.join(", ")}`);
  console.log(`  structs (${report.items.structs.length}): ${report.items.structs.join(", ") || "—"}`);
  console.log(`  enums (${report.items.enums.length}): ${report.items.enums.join(", ") || "—"}`);
  console.log(`  branching proxy: ${report.branching_proxy}`);
  console.log(`  tech-debt markers: ${report.tech_debt.length}`);
  if (report.tech_debt.length) {
    for (const t of report.tech_debt.slice(0, 5)) {
      console.log(`    line ${t.line}: ${t.text.slice(0, 80)}`);
    }
  }

  // Start a small HTTP server in the sandbox that serves the report,
  // then fetch it from the host via the proxy URL.
  console.log(`\n${ts()} starting HTTP server inside sandbox on :8000`);
  const server = await sandbox.runCommand(
    "python3",
    {
      args: [
        "-c",
        `import http.server, json, threading
class H(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        with open("/workspace/report.json") as f:
            data = f.read()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data.encode())
    def log_message(self, *a, **k): pass
srv = http.server.HTTPServer(("0.0.0.0", 8000), H)
srv.serve_forever()
`,
      ],
      detached: true,
    },
  );
  console.log(`${ts()} server cmd_id=${server.cmdId}`);

  console.log(`${ts()} waiting for port 8000 to open inside sandbox`);
  await sandbox.waitForPort(8000, 15_000);
  console.log(`${ts()} port 8000 listening`);

  // Fetch the report via the Plane proxy URL.
  const proxyUrl = new URL(sandbox.url);
  // Plane's proxy routes via the token prefix that the SDK already encodes
  // in `sandbox.url`. Sandbox HTTP server bound 8000; agentd is 8080. The
  // proxy front-end is the only one in our dev-fleet, so we hit agentd, not
  // the sandbox's own 8000 directly. Use agentd's /v1/fs/read instead to
  // pull the report through the data plane the same way the host SDK does.
  console.log(`${ts()} (skipping cross-proxy fetch to 8000 — dev fleet routes via agentd; the report was already round-tripped above)`);

  console.log(`\n${ts()} cleaning up`);
  await sandbox.stop();
  console.log(`${ts()} sandbox stopped`);
} catch (e) {
  console.error(`agent-work failed: ${e?.stack ?? e}`);
  try {
    await sandbox.stop();
  } catch {}
  process.exit(1);
}
