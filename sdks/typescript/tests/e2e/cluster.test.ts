import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { Sandbox } from "../../src/index.js";
import { SandboxClient } from "../../src/client.js";

const CONTROL_URL = process.env.CONTROL_URL ?? "http://localhost:8080";
const TIMEOUT_MS = 120_000;
const RUN_ID = process.env.RUN_ID ?? String(Date.now());
const seed = parseInt(RUN_ID.replace(/\D/g, "").slice(-6), 10) || 123456;

let client: SandboxClient;
let sandbox: Sandbox;

beforeAll(async () => {
  client = new SandboxClient({ controlUrl: CONTROL_URL, tenant: "tenant-default" });

  sandbox = await Sandbox.create(
    {
      runtime: "node24",
      trust: "untrusted",
      resources: { vcpus: 1, memory_mb: 512 },
      timeout_ms: TIMEOUT_MS,
      env: { NODE_ENV: "test", RUN_ID },
      tags: { purpose: "e2e-cluster", run_id: RUN_ID },
    },
    { controlUrl: CONTROL_URL, tenant: "tenant-default" },
  );

  await waitForReady(sandbox.url, 30_000);
}, 60_000);

afterAll(async () => {
  if (sandbox) {
    await sandbox.stop().catch(() => {});
  }
});

describe("e2e/cluster: sandbox lifecycle", () => {
  it("sandbox was created with expected fields", () => {
    expect(sandbox.id).toMatch(/^sb-/);
    expect(sandbox.runtime).toBe("node24");
    expect(sandbox.status).toBe("scheduled");
    expect(sandbox.url).toContain("localhost:9090");
  });

  it("sandbox is listed and retrievable", async () => {
    const list = await client.list({ limit: 50 });
    const found = list.items.find((s) => s.id === sandbox.id);
    expect(found).toBeDefined();
    expect(found!.tags.run_id).toBe(RUN_ID);

    const details = await client.get(sandbox.id);
    expect(details.backend_id).toBeDefined();
  });
});

describe(`e2e/cluster: data processing pipeline (run=${RUN_ID})`, () => {
  const ws = `/tmp/pipeline-${RUN_ID}`;

  it("generates a CSV dataset and computes statistics", async () => {
    await sandbox.mkdir(ws, { recursive: true });

    const rows = 50 + (seed % 50);
    const csvLines = ["id,value,category"];
    const values: number[] = [];
    for (let i = 0; i < rows; i++) {
      const v = Math.round(((seed * (i + 1) * 7 + 13) % 10000) / 100);
      values.push(v);
      csvLines.push(`${i},${v},${["alpha", "beta", "gamma"][i % 3]}`);
    }
    await sandbox.writeFile(`${ws}/data.csv`, csvLines.join("\n"));

    await sandbox.writeFile(
      `${ws}/analyze.js`,
      `
const fs = require("fs");
const lines = fs.readFileSync("${ws}/data.csv", "utf-8").trim().split("\\n");
const header = lines[0].split(",");
const records = lines.slice(1).map(l => {
  const cols = l.split(",");
  return { id: +cols[0], value: +cols[1], category: cols[2] };
});

const vals = records.map(r => r.value);
const sum = vals.reduce((a, b) => a + b, 0);
const mean = sum / vals.length;
const sorted = [...vals].sort((a, b) => a - b);
const median = vals.length % 2 === 0
  ? (sorted[vals.length / 2 - 1] + sorted[vals.length / 2]) / 2
  : sorted[Math.floor(vals.length / 2)];
const variance = vals.reduce((a, v) => a + (v - mean) ** 2, 0) / vals.length;
const stddev = Math.sqrt(variance);

const byCat = {};
for (const r of records) {
  if (!byCat[r.category]) byCat[r.category] = [];
  byCat[r.category].push(r.value);
}
const catMeans = {};
for (const [cat, vs] of Object.entries(byCat)) {
  catMeans[cat] = vs.reduce((a, b) => a + b, 0) / vs.length;
}

const result = {
  count: records.length,
  sum: Math.round(sum * 100) / 100,
  mean: Math.round(mean * 100) / 100,
  median,
  stddev: Math.round(stddev * 100) / 100,
  min: sorted[0],
  max: sorted[sorted.length - 1],
  categories: Object.keys(catMeans).sort(),
  category_means: catMeans,
};

fs.writeFileSync("${ws}/stats.json", JSON.stringify(result));
console.log(JSON.stringify(result));
`,
    );

    const result = await sandbox.runCommand("node", { args: [`${ws}/analyze.js`] });
    expect(result.exitCode).toBe(0);

    const stats = JSON.parse(result.stdout.trim());
    expect(stats.count).toBe(rows);
    expect(stats.categories).toEqual(["alpha", "beta", "gamma"]);
    expect(stats.min).toBeLessThanOrEqual(stats.median);
    expect(stats.median).toBeLessThanOrEqual(stats.max);
    expect(stats.stddev).toBeGreaterThan(0);

    const expectedSum = values.reduce((a, b) => a + b, 0);
    expect(stats.sum).toBe(Math.round(expectedSum * 100) / 100);
    const expectedMean = expectedSum / values.length;
    expect(stats.mean).toBe(Math.round(expectedMean * 100) / 100);

    const persisted = JSON.parse(await sandbox.readTextFile(`${ws}/stats.json`));
    expect(persisted).toEqual(stats);
  });

  it("filters, sorts, and re-exports the dataset", async () => {
    await sandbox.writeFile(
      `${ws}/transform.js`,
      `
const fs = require("fs");
const lines = fs.readFileSync("${ws}/data.csv", "utf-8").trim().split("\\n");
const records = lines.slice(1).map(l => {
  const cols = l.split(",");
  return { id: +cols[0], value: +cols[1], category: cols[2] };
});

const filtered = records.filter(r => r.value >= 50);
filtered.sort((a, b) => b.value - a.value);

const topN = Math.min(10, filtered.length);
const top = filtered.slice(0, topN);

const output = ["rank,id,value,category"];
top.forEach((r, i) => output.push(\`\${i + 1},\${r.id},\${r.value},\${r.category}\`));
fs.writeFileSync("${ws}/top.csv", output.join("\\n"));

const summary = {
  total_records: records.length,
  above_threshold: filtered.length,
  top_count: topN,
  highest: top[0]?.value ?? null,
};
console.log(JSON.stringify(summary));
`,
    );

    const result = await sandbox.runCommand("node", { args: [`${ws}/transform.js`] });
    expect(result.exitCode).toBe(0);

    const summary = JSON.parse(result.stdout.trim());
    expect(summary.above_threshold).toBeLessThanOrEqual(summary.total_records);
    expect(summary.top_count).toBeLessThanOrEqual(10);
    if (summary.highest !== null) {
      expect(summary.highest).toBeGreaterThanOrEqual(50);
    }

    const topCsv = await sandbox.readTextFile(`${ws}/top.csv`);
    const topLines = topCsv.trim().split("\n");
    expect(topLines[0]).toBe("rank,id,value,category");
    expect(topLines.length - 1).toBe(summary.top_count);

    if (topLines.length > 2) {
      const firstVal = parseInt(topLines[1].split(",")[2], 10);
      const secondVal = parseInt(topLines[2].split(",")[2], 10);
      expect(firstVal).toBeGreaterThanOrEqual(secondVal);
    }
  });
});

describe(`e2e/cluster: multi-file project build (run=${RUN_ID})`, () => {
  const proj = `/tmp/project-${RUN_ID}`;

  it("creates a Node.js project, builds, and runs tests", async () => {
    await sandbox.mkdir(`${proj}/src`, { recursive: true });
    await sandbox.mkdir(`${proj}/test`, { recursive: true });

    const scaleFactor = 1 + (seed % 10);

    await sandbox.writeFile(
      `${proj}/src/math.js`,
      `
function fibonacci(n) {
  if (n <= 1) return n;
  let a = 0, b = 1;
  for (let i = 2; i <= n; i++) { [a, b] = [b, a + b]; }
  return b;
}

function isPrime(n) {
  if (n < 2) return false;
  if (n < 4) return true;
  if (n % 2 === 0 || n % 3 === 0) return false;
  for (let i = 5; i * i <= n; i += 6) {
    if (n % i === 0 || n % (i + 2) === 0) return false;
  }
  return true;
}

function sieve(limit) {
  const flags = new Uint8Array(limit + 1).fill(1);
  flags[0] = flags[1] = 0;
  for (let i = 2; i * i <= limit; i++) {
    if (flags[i]) for (let j = i * i; j <= limit; j += i) flags[j] = 0;
  }
  const primes = [];
  for (let i = 2; i <= limit; i++) if (flags[i]) primes.push(i);
  return primes;
}

function matMul(a, b) {
  const rows = a.length, cols = b[0].length, inner = b.length;
  const c = Array.from({ length: rows }, () => new Array(cols).fill(0));
  for (let i = 0; i < rows; i++)
    for (let k = 0; k < inner; k++)
      for (let j = 0; j < cols; j++)
        c[i][j] += a[i][k] * b[k][j];
  return c;
}

module.exports = { fibonacci, isPrime, sieve, matMul };
`,
    );

    await sandbox.writeFile(
      `${proj}/src/transform.js`,
      `
const { fibonacci, sieve, matMul } = require("./math");

function generateDataset(n, scaleFactor) {
  const primes = sieve(n * scaleFactor + 200);
  return Array.from({ length: n }, (_, i) => ({
    index: i,
    fib: fibonacci(i % 30),
    nearestPrime: primes.find(p => p >= i * scaleFactor) ?? primes[primes.length - 1],
    category: i % 2 === 0 ? "even" : "odd",
  }));
}

function aggregate(dataset) {
  const groups = {};
  for (const row of dataset) {
    if (!groups[row.category]) groups[row.category] = { count: 0, fibSum: 0, primeSum: 0 };
    groups[row.category].count++;
    groups[row.category].fibSum += row.fib;
    groups[row.category].primeSum += row.nearestPrime;
  }
  return groups;
}

module.exports = { generateDataset, aggregate };
`,
    );

    await sandbox.writeFile(
      `${proj}/test/run.js`,
      `
const { fibonacci, isPrime, sieve, matMul } = require("../src/math");
const { generateDataset, aggregate } = require("../src/transform");

const SCALE = ${scaleFactor};
let passed = 0;
let failed = 0;

function assert(cond, msg) {
  if (cond) { passed++; }
  else { failed++; console.error("FAIL:", msg); }
}

assert(fibonacci(0) === 0, "fib(0)");
assert(fibonacci(1) === 1, "fib(1)");
assert(fibonacci(10) === 55, "fib(10)");
assert(fibonacci(20) === 6765, "fib(20)");

assert(isPrime(2), "2 is prime");
assert(isPrime(13), "13 is prime");
assert(!isPrime(1), "1 is not prime");
assert(!isPrime(15), "15 is not prime");

const primes = sieve(100 * SCALE);
assert(primes[0] === 2, "sieve starts at 2");
assert(primes.every(p => isPrime(p)), "all sieve results are prime");
assert(primes.length > 10, "sieve found enough primes");

const A = [[1, 2], [3, 4]];
const B = [[5, 6], [7, 8]];
const C = matMul(A, B);
assert(C[0][0] === 19 && C[0][1] === 22, "matmul row 0");
assert(C[1][0] === 43 && C[1][1] === 50, "matmul row 1");

const identity = [[1, 0], [0, 1]];
const AI = matMul(A, identity);
assert(AI[0][0] === 1 && AI[0][1] === 2, "matmul identity row 0");

const dataset = generateDataset(100, SCALE);
assert(dataset.length === 100, "dataset size");
assert(dataset[0].fib === 0, "first fib is 0");
assert(dataset.every(r => r.nearestPrime >= r.index * SCALE || r.index === 0), "primes >= index*scale");

const groups = aggregate(dataset);
assert(groups.even.count === 50, "50 evens");
assert(groups.odd.count === 50, "50 odds");
assert(groups.even.fibSum + groups.odd.fibSum === dataset.reduce((s, r) => s + r.fib, 0), "fib sums match");

const result = { passed, failed, scale: SCALE, primeCount: primes.length, datasetSize: dataset.length };
console.log(JSON.stringify(result));
process.exit(failed > 0 ? 1 : 0);
`,
    );

    const result = await sandbox.runCommand("node", { args: [`${proj}/test/run.js`] });
    expect(result.exitCode).toBe(0);

    const report = JSON.parse(result.stdout.trim());
    expect(report.failed).toBe(0);
    expect(report.passed).toBeGreaterThan(15);
    expect(report.scale).toBe(scaleFactor);
    expect(report.primeCount).toBeGreaterThan(10);
  });

  it("builds a concatenated bundle and verifies it runs", async () => {
    await sandbox.writeFile(
      `${proj}/build.js`,
      `
const fs = require("fs");
const path = require("path");

const srcDir = "${proj}/src";
const files = fs.readdirSync(srcDir).filter(f => f.endsWith(".js")).sort();
const banner = "// Bundle generated at " + new Date().toISOString() + "\\n";
const modules = files.map(f => {
  const content = fs.readFileSync(path.join(srcDir, f), "utf-8");
  return \`// --- \${f} ---\\n\${content}\`;
}).join("\\n\\n");

const bundle = banner + modules + \`
\\n// --- entry ---
const { sieve } = require("./src/math");
const primes = sieve(1000);
console.log(JSON.stringify({ files: ${JSON.stringify("files")}, primeCount: primes.length, largest: primes[primes.length - 1] }));
\`;

fs.writeFileSync("${proj}/bundle.js", bundle);
console.log(JSON.stringify({ fileCount: files.length, bundleSize: bundle.length }));
`,
    );

    const buildResult = await sandbox.runCommand("node", { args: [`${proj}/build.js`] });
    expect(buildResult.exitCode).toBe(0);

    const buildInfo = JSON.parse(buildResult.stdout.trim());
    expect(buildInfo.fileCount).toBe(2);
    expect(buildInfo.bundleSize).toBeGreaterThan(100);

    const bundleContent = await sandbox.readTextFile(`${proj}/bundle.js`);
    expect(bundleContent).toContain("// Bundle generated at");
    expect(bundleContent).toContain("fibonacci");
    expect(bundleContent).toContain("generateDataset");
  });
});

describe(`e2e/cluster: parallel worker fan-out (run=${RUN_ID})`, () => {
  const ws = `/tmp/workers-${RUN_ID}`;
  const workerCount = 3 + (seed % 3);

  it("fans out work to N parallel processes, merges results", async () => {
    await sandbox.mkdir(`${ws}/results`, { recursive: true });

    await sandbox.writeFile(
      `${ws}/worker.js`,
      `
const fs = require("fs");
const workerId = process.argv[2];
const rangeStart = parseInt(process.argv[3], 10);
const rangeEnd = parseInt(process.argv[4], 10);

let primeCount = 0;
let sum = 0;
for (let n = rangeStart; n < rangeEnd; n++) {
  let isPrime = n >= 2;
  for (let i = 2; i * i <= n && isPrime; i++) {
    if (n % i === 0) isPrime = false;
  }
  if (isPrime) { primeCount++; sum += n; }
}

const result = { workerId, rangeStart, rangeEnd, primeCount, primeSum: sum };
fs.writeFileSync(\`${ws}/results/worker-\${workerId}.json\`, JSON.stringify(result));
console.log("done:" + workerId);
`,
    );

    const rangeSize = Math.ceil(5000 / workerCount);
    const spawns = [];
    for (let i = 0; i < workerCount; i++) {
      const start = i * rangeSize;
      const end = Math.min((i + 1) * rangeSize, 5000);
      spawns.push(
        sandbox.runCommand("node", {
          args: [`${ws}/worker.js`, String(i), String(start), String(end)],
          detached: true,
        }),
      );
    }
    const workers = await Promise.all(spawns);

    await waitFor(async () => {
      const allDone = await Promise.all(
        workers.map((w) => sandbox.getProcessStatus(w.cmdId)),
      );
      return allDone.every((s) => s.status === "exited");
    }, 15_000);

    const merge = await sandbox.runCommand("node", {
      args: [
        "-e",
        `
const fs = require("fs");
const dir = "${ws}/results";
const files = fs.readdirSync(dir).filter(f => f.endsWith(".json"));
const results = files.map(f => JSON.parse(fs.readFileSync(dir + "/" + f, "utf-8")));
results.sort((a, b) => a.rangeStart - b.rangeStart);

let totalPrimes = 0, totalSum = 0;
for (const r of results) { totalPrimes += r.primeCount; totalSum += r.primeSum; }

console.log(JSON.stringify({
  workerCount: results.length,
  totalPrimes,
  totalSum,
  ranges: results.map(r => [r.rangeStart, r.rangeEnd]),
}));
`,
      ],
    });
    expect(merge.exitCode).toBe(0);

    const merged = JSON.parse(merge.stdout.trim());
    expect(merged.workerCount).toBe(workerCount);
    expect(merged.totalPrimes).toBeGreaterThan(600);
    expect(merged.totalPrimes).toBeLessThan(700);
    expect(merged.totalSum).toBeGreaterThan(1_000_000);
    expect(merged.ranges[0][0]).toBe(0);
    expect(merged.ranges[merged.ranges.length - 1][1]).toBe(5000);

    const verify = await sandbox.runCommand("node", {
      args: [
        "-e",
        `
let count = 0, sum = 0;
for (let n = 2; n < 5000; n++) {
  let ok = true;
  for (let i = 2; i * i <= n && ok; i++) if (n % i === 0) ok = false;
  if (ok) { count++; sum += n; }
}
console.log(JSON.stringify({ count, sum }));
`,
      ],
    });
    expect(verify.exitCode).toBe(0);
    const truth = JSON.parse(verify.stdout.trim());
    expect(merged.totalPrimes).toBe(truth.count);
    expect(merged.totalSum).toBe(truth.sum);
  });

  it("stdin-driven processing pipeline", async () => {
    const msg = `task-${RUN_ID}`;
    const outPath = `${ws}/stdin-result.json`;

    const spawned = await sandbox.runCommand("node", {
      args: [
        "-e",
        `
process.stdin.setEncoding("utf-8");
let buf = "";
process.stdin.on("data", chunk => {
  buf += chunk;
  if (buf.includes("__END__")) {
    const lines = buf.split("\\n").filter(l => l && l !== "__END__");
    const result = {
      lineCount: lines.length,
      reversed: lines.map(l => l.split("").reverse().join("")),
      lengths: lines.map(l => l.length),
      checksum: lines.reduce((s, l) => s + l.length, 0),
    };
    require("fs").writeFileSync("${outPath}", JSON.stringify(result));
    process.exit(0);
  }
});
`,
      ],
      detached: true,
    });

    const lines = [`${msg}-alpha`, `${msg}-beta-longer`, `${msg}-gamma`];
    await sandbox.writeStdin(spawned.cmdId, lines.join("\n") + "\n__END__\n");

    await waitFor(async () => {
      const s = await sandbox.getProcessStatus(spawned.cmdId);
      return s.status === "exited";
    }, 5_000);

    const result = JSON.parse(await sandbox.readTextFile(outPath));
    expect(result.lineCount).toBe(3);
    expect(result.reversed[0]).toBe(lines[0].split("").reverse().join(""));
    expect(result.lengths).toEqual(lines.map((l) => l.length));
    expect(result.checksum).toBe(lines.reduce((s, l) => s + l.length, 0));
  });
});

describe(`e2e/cluster: agentic code generation (run=${RUN_ID})`, () => {
  const ws = `/tmp/codegen-${RUN_ID}`;

  it("generates code from a spec, runs it, verifies output", async () => {
    await sandbox.mkdir(ws, { recursive: true });

    const dimensions = 2 + (seed % 4);
    const size = 3 + (seed % 3);

    await sandbox.writeFile(
      `${ws}/spec.json`,
      JSON.stringify({
        task: "matrix_operations",
        dimensions,
        size,
        operations: ["create", "transpose", "multiply", "determinant"],
      }),
    );

    await sandbox.writeFile(
      `${ws}/generate.js`,
      `
const fs = require("fs");
const spec = JSON.parse(fs.readFileSync("${ws}/spec.json", "utf-8"));
const { size } = spec;

const code = \`
function createMatrix(size, seedVal) {
  return Array.from({ length: size }, (_, i) =>
    Array.from({ length: size }, (_, j) => ((i * size + j + seedVal) % 97) - 48)
  );
}

function transpose(m) {
  return m[0].map((_, j) => m.map(row => row[j]));
}

function multiply(a, b) {
  const n = a.length;
  const result = Array.from({ length: n }, () => new Array(n).fill(0));
  for (let i = 0; i < n; i++)
    for (let k = 0; k < n; k++)
      for (let j = 0; j < n; j++)
        result[i][j] += a[i][k] * b[k][j];
  return result;
}

function determinant(m) {
  const n = m.length;
  if (n === 1) return m[0][0];
  if (n === 2) return m[0][0] * m[1][1] - m[0][1] * m[1][0];
  let det = 0;
  for (let j = 0; j < n; j++) {
    const minor = m.slice(1).map(row => [...row.slice(0, j), ...row.slice(j + 1)]);
    det += (j % 2 === 0 ? 1 : -1) * m[0][j] * determinant(minor);
  }
  return det;
}

const A = createMatrix(\${size}, 7);
const B = createMatrix(\${size}, 31);
const At = transpose(A);
const AB = multiply(A, B);
const detA = determinant(A);

// verify transpose: A[i][j] === At[j][i]
let transposeOk = true;
for (let i = 0; i < \${size}; i++)
  for (let j = 0; j < \${size}; j++)
    if (A[i][j] !== At[j][i]) transposeOk = false;

// verify multiply: (AB)^T === B^T * A^T
const ABt = transpose(AB);
const BtAt = multiply(transpose(B), At);
let multiplyOk = true;
for (let i = 0; i < \${size}; i++)
  for (let j = 0; j < \${size}; j++)
    if (ABt[i][j] !== BtAt[i][j]) multiplyOk = false;

console.log(JSON.stringify({
  size: \${size},
  detA,
  transposeOk,
  multiplyOk,
  A_sample: A[0],
  AB_sample: AB[0],
}));
\`;

fs.writeFileSync("${ws}/matrix_ops.js", code);
console.log("generated");
`,
    );

    const gen = await sandbox.runCommand("node", { args: [`${ws}/generate.js`] });
    expect(gen.exitCode).toBe(0);

    const run = await sandbox.runCommand("node", { args: [`${ws}/matrix_ops.js`] });
    expect(run.exitCode).toBe(0);

    const output = JSON.parse(run.stdout.trim());
    expect(output.size).toBe(size);
    expect(output.transposeOk).toBe(true);
    expect(output.multiplyOk).toBe(true);
    expect(typeof output.detA).toBe("number");
    expect(output.A_sample.length).toBe(size);
  });

  it("detects and recovers from a runtime error", async () => {
    await sandbox.writeFile(
      `${ws}/buggy.js`,
      `
const data = [${Array.from({ length: 5 }, (_, i) => ((seed + i * 7) % 100)).join(",")}];
const result = data.map(x => x.toString().split("").reverse().join(""));
// Bug: treating string result as number
const sum = result.reduce((a, b) => a + b, 0);
console.log(JSON.stringify({ sum, type: typeof sum }));
`,
    );

    const firstRun = await sandbox.runCommand("node", { args: [`${ws}/buggy.js`] });
    expect(firstRun.exitCode).toBe(0);
    const firstOutput = JSON.parse(firstRun.stdout.trim());
    expect(firstOutput.type).toBe("string");

    await sandbox.writeFile(
      `${ws}/fixed.js`,
      `
const data = [${Array.from({ length: 5 }, (_, i) => ((seed + i * 7) % 100)).join(",")}];
const reversed = data.map(x => parseInt(x.toString().split("").reverse().join(""), 10) || 0);
const sum = reversed.reduce((a, b) => a + b, 0);
const original_sum = data.reduce((a, b) => a + b, 0);
console.log(JSON.stringify({ reversed, sum, original_sum, type: typeof sum, fixed: true }));
`,
    );

    const fixedRun = await sandbox.runCommand("node", { args: [`${ws}/fixed.js`] });
    expect(fixedRun.exitCode).toBe(0);

    const fixedOutput = JSON.parse(fixedRun.stdout.trim());
    expect(fixedOutput.type).toBe("number");
    expect(fixedOutput.fixed).toBe(true);
    expect(fixedOutput.sum).toBeGreaterThan(0);
    expect(fixedOutput.reversed.length).toBe(5);
  });
});

describe(`e2e/cluster: HTTP microservice (run=${RUN_ID})`, () => {
  const ws = `/tmp/svc-${RUN_ID}`;
  const port = 4000 + (parseInt(RUN_ID.replace(/\D/g, "").slice(-3), 10) % 1000);

  it("builds and queries an in-sandbox REST API", async () => {
    await sandbox.mkdir(ws, { recursive: true });

    await sandbox.writeFile(
      `${ws}/server.js`,
      `
const http = require("http");
const url = require("url");

const store = new Map();

const server = http.createServer((req, res) => {
  const parsed = url.parse(req.url, true);
  const path = parsed.pathname;

  res.setHeader("Content-Type", "application/json");

  if (req.method === "GET" && path === "/health") {
    res.end(JSON.stringify({ status: "ok", run: "${RUN_ID}", items: store.size }));
    return;
  }

  if (req.method === "POST" && path === "/items") {
    let body = "";
    req.on("data", c => body += c);
    req.on("end", () => {
      const item = JSON.parse(body);
      const id = "item-" + (store.size + 1);
      store.set(id, { ...item, id, created_at: Date.now() });
      res.writeHead(201);
      res.end(JSON.stringify(store.get(id)));
    });
    return;
  }

  if (req.method === "GET" && path === "/items") {
    const items = Array.from(store.values());
    const category = parsed.query.category;
    const filtered = category ? items.filter(i => i.category === category) : items;
    res.end(JSON.stringify({ items: filtered, total: filtered.length }));
    return;
  }

  if (req.method === "GET" && path === "/stats") {
    const items = Array.from(store.values());
    const byCategory = {};
    for (const item of items) {
      if (!byCategory[item.category]) byCategory[item.category] = { count: 0, totalValue: 0 };
      byCategory[item.category].count++;
      byCategory[item.category].totalValue += item.value;
    }
    res.end(JSON.stringify({ itemCount: items.length, byCategory }));
    return;
  }

  res.writeHead(404);
  res.end(JSON.stringify({ error: "not found" }));
});

server.listen(${port}, () => console.log("up"));
`,
    );

    const spawned = await sandbox.runCommand("node", { args: [`${ws}/server.js`], detached: true });

    await waitFor(async () => {
      const r = await sandbox.runCommand("sh", {
        args: ["-c", `curl -sf http://localhost:${port}/health`],
      });
      return r.exitCode === 0;
    }, 10_000);

    const items = [
      { name: `widget-${RUN_ID}`, value: 10 + (seed % 90), category: "tools" },
      { name: `gadget-${RUN_ID}`, value: 20 + (seed % 80), category: "electronics" },
      { name: `wrench-${RUN_ID}`, value: 5 + (seed % 50), category: "tools" },
      { name: `phone-${RUN_ID}`, value: 50 + (seed % 200), category: "electronics" },
      { name: `hammer-${RUN_ID}`, value: 8 + (seed % 30), category: "tools" },
    ];

    for (const item of items) {
      const post = await sandbox.runCommand("sh", {
        args: [
          "-c",
          `curl -sf -X POST -H "Content-Type: application/json" -d '${JSON.stringify(item)}' http://localhost:${port}/items`,
        ],
      });
      expect(post.exitCode).toBe(0);
      const created = JSON.parse(post.stdout.trim());
      expect(created.id).toMatch(/^item-/);
      expect(created.name).toBe(item.name);
    }

    const listAll = await sandbox.runCommand("sh", {
      args: ["-c", `curl -sf http://localhost:${port}/items`],
    });
    expect(listAll.exitCode).toBe(0);
    const allItems = JSON.parse(listAll.stdout.trim());
    expect(allItems.total).toBe(5);

    const listTools = await sandbox.runCommand("sh", {
      args: ["-c", `curl -sf 'http://localhost:${port}/items?category=tools'`],
    });
    expect(listTools.exitCode).toBe(0);
    const toolItems = JSON.parse(listTools.stdout.trim());
    expect(toolItems.total).toBe(3);
    expect(toolItems.items.every((i: { category: string }) => i.category === "tools")).toBe(true);

    const stats = await sandbox.runCommand("sh", {
      args: ["-c", `curl -sf http://localhost:${port}/stats`],
    });
    expect(stats.exitCode).toBe(0);
    const statsData = JSON.parse(stats.stdout.trim());
    expect(statsData.itemCount).toBe(5);
    expect(statsData.byCategory.tools.count).toBe(3);
    expect(statsData.byCategory.electronics.count).toBe(2);

    const expectedToolsTotal = items
      .filter((i) => i.category === "tools")
      .reduce((s, i) => s + i.value, 0);
    expect(statsData.byCategory.tools.totalValue).toBe(expectedToolsTotal);

    await sandbox.signal(spawned.cmdId, "KILL");
  });
});

describe(`e2e/cluster: npm package workflow (run=${RUN_ID})`, () => {
  it("installs a package and uses it for real computation", async () => {
    const install = await sandbox.runCommand("npm", {
      args: ["install", "--no-save", "lodash"],
      cwd: "/tmp",
      timeout_ms: 30_000,
    });
    expect(install.exitCode).toBe(0);

    const dataset = Array.from({ length: 20 }, (_, i) => ({
      id: i,
      group: ["A", "B", "C"][i % 3],
      score: ((seed + i * 13) % 100),
    }));

    const result = await sandbox.runCommand("node", {
      args: [
        "-e",
        `
const _ = require("lodash");
const data = ${JSON.stringify(dataset)};

const grouped = _.groupBy(data, "group");
const summary = _.mapValues(grouped, items => ({
  count: items.length,
  mean: _.round(_.meanBy(items, "score"), 2),
  max: _.maxBy(items, "score").score,
  min: _.minBy(items, "score").score,
  sorted: _.sortBy(items, "score").map(i => i.score),
}));

const flattened = _.chain(data)
  .filter(d => d.score > 50)
  .sortBy("score")
  .reverse()
  .take(5)
  .map(d => _.pick(d, ["id", "group", "score"]))
  .value();

console.log(JSON.stringify({ summary, top5: flattened }));
`,
      ],
      cwd: "/tmp",
    });
    expect(result.exitCode).toBe(0);

    const output = JSON.parse(result.stdout.trim());
    expect(Object.keys(output.summary)).toEqual(expect.arrayContaining(["A", "B", "C"]));

    for (const group of Object.values(output.summary) as Array<{
      count: number;
      mean: number;
      max: number;
      min: number;
      sorted: number[];
    }>) {
      expect(group.count).toBeGreaterThan(0);
      expect(group.max).toBeGreaterThanOrEqual(group.min);
      expect(group.mean).toBeGreaterThanOrEqual(group.min);
      expect(group.mean).toBeLessThanOrEqual(group.max);
      for (let i = 1; i < group.sorted.length; i++) {
        expect(group.sorted[i]).toBeGreaterThanOrEqual(group.sorted[i - 1]);
      }
    }

    expect(output.top5.length).toBeLessThanOrEqual(5);
    for (let i = 1; i < output.top5.length; i++) {
      expect(output.top5[i - 1].score).toBeGreaterThanOrEqual(output.top5[i].score);
    }
  });
});

describe("e2e/cluster: cleanup", () => {
  it("stops the sandbox", async () => {
    await sandbox.stop();

    await waitFor(async () => {
      const details = await client.get(sandbox.id);
      return ["terminating", "hard-terminating", "terminated"].includes(details.status);
    }, 10_000);
  });
});

async function waitForReady(proxyUrl: string, timeoutMs: number): Promise<void> {
  const healthUrl = `${proxyUrl.replace(/\/$/, "")}/health`;
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const res = await fetch(healthUrl);
      if (res.ok) return;
    } catch {
      // not ready yet
    }
    await new Promise((r) => setTimeout(r, 500));
  }
  throw new Error(`sandbox did not become ready at ${healthUrl} within ${timeoutMs}ms`);
}

async function waitFor(
  predicate: () => Promise<boolean>,
  timeoutMs: number,
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (await predicate()) return;
    await new Promise((r) => setTimeout(r, 300));
  }
  throw new Error(`waitFor timed out after ${timeoutMs}ms`);
}
