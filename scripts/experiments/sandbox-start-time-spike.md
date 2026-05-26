# Sandbox start-time — optimization spike

Source data: `scripts/experiments/sandbox-start-time.mjs` against local Docker fleet, 4 runtimes × 5 iters + 1 prewarm, runc pool, warm images.

## Baseline

| runtime | p50 total | server-side total | dominant phase (median) |
|---|---:|---:|---:|
| node24    |  868 ms | 854 ms | start→wait 632 ms (74%) |
| python313 |  959 ms | 948 ms | start→wait 693 ms (73%) |
| agent     |  966 ms | 954 ms | start→wait 779 ms (82%) |
| desktop   | 1009 ms | 997 ms | start→wait 712 ms (71%) |

`create POST + sched→load + load→start` is consistently 40–80 ms — the platform's own control-plane work is already cheap. **Everything else is downstream of Docker `start_container`.**

## Where the 600–800 ms in start→wait actually goes

Isolated `docker start` (no Plane in the loop, fresh container each time, same image):

| run | start API returns | port listenable |
|----:|------------------:|----------------:|
| 1 | 449 ms | 558 ms |
| 2 | 500 ms | 613 ms |
| 3 | 401 ms | 525 ms |
| 4 | 805 ms | 917 ms |
| 5 | 945 ms | 1078 ms |

So on Docker Desktop / macOS the **container-start floor is ~500 ms p50, with a long tail past 1 s**. That's a Docker Desktop VM cost, not anything Plane can fix on this host. Once the start API returns, agentd binds the port within ~50–150 ms.

Implication: on Linux drones (Vagrant, AWS) the same workload would be ~300–500 ms total instead of ~900 ms. Most of what we measured on macOS is the dev environment, not the platform.

## Opportunities, ranked by likely ROI

### Tier 1 — structural, large impact

**1. Container pool / pre-warmed sandboxes** (potential: -500 to -800 ms)
- Keep N warm containers per `(runtime, trust)` tier; on spawn, claim one and reset workspace
- Brings time-to-ready to "checkout a connection" latency (~tens of ms)
- Cost: idle memory + reset semantics (Plane currently treats containers as one-shot)
- New code paths in drone (pool manager, reset hook) + new platform-api option (`pool_target_warm: N`)
- This is what Modal / E2B / Vercel Sandbox do under the hood

**2. Bake egress-proxy setup into agentd** (potential: -50 to -200 ms on Linux; -30 ms on macOS measured)
- Today: `tini -> sandbox-entrypoint.sh -> getent + curl + 2× iptables -> exec agentd`
- 4 forked subprocesses + 1 admin-API call before agentd even starts
- agentd already runs as PID 2 — move iptables/CA setup into agentd's `main()` pre-`bind` and drop the wrapper entirely. Drops a process layer and parallelizes nothing useful right now.
- On macOS the measured overhead is ~30 ms; on Linux drones with iptables xtables-lock contention this commonly spikes higher.

**3. Surface Plane key-reuse through the SDK** (potential: -700+ ms when applicable)
- `ConnectResponse.spawned: bool` exists. `KeyConfig` is already plumbed.
- Today `create_sandbox` always calls `KeyConfig::new_random`, so reuse never fires.
- Add `key` (or `idempotency_key` / `pool_key`) to `CreateSandboxRequest`; pass through to Plane.
- For agentic workloads that hit the same tenant repeatedly, ready-time becomes ~0 for cache hits.
- No new infra; just contract + SDK affordance. Cheapest big win.

### Tier 2 — local code, medium impact

**4. agentd posts "ready" to the drone instead of TCP polling** (potential: -50 to -100 ms)
- Today: `wait_for_backend` connects every 100 ms, does HEAD, reads 5 bytes. Floor = 100 ms.
- Cleaner: agentd writes to a known unix socket / shared volume / sends a Docker label update once it binds; drone watches that.
- Removes both the 100 ms polling slop *and* the per-poll inspect-container roundtrip.
- Real fix; not a tweak. New protocol bit between agentd and drone.

**5. Tune `WAIT_TIMEOUT_MS = 100` → 25 ms** (potential: -25 to -75 ms)
- Trivial. Cost: more idle TCP connects on localhost — negligible.
- Doesn't fix the structural issue but ships in a one-line change.

**6. Trust TCP connect as "ready", drop the HEAD/read** (potential: -5 to -20 ms)
- `bind() → listen()` ordering in tokio means a successful connect after listen ≈ accept-ready
- Saves one RTT per probe. Marginal but free.
- Slight risk during reverse-proxy edge cases; verify with `wait_for_backend` test.

### Tier 3 — infra / longer horizon

**7. Skip Docker; talk to containerd directly** (potential: -50 to -150 ms on Linux; nothing on macOS)
- bollard → dockerd → containerd → runc has 2 extra hops vs. crun via the containerd client
- Real engineering project — replaces the `DockerRuntime` trait impl
- Only worth doing once Tiers 1 and 2 are exhausted

**8. Persist drone `image_cache` to disk** (potential: -5 to -10 ms first-spawn-after-restart only)
- Currently a `HashSet<String>` in memory; first spawn after drone restart pays one `inspect_image` call (~5–10 ms)
- Tiny, only matters under high churn. Defer.

**9. Smaller runtime images / fewer layers** (potential: marginal on warm-image path, large on cold pulls)
- node24 image is 249 MB, python313 is 136 MB
- Layer count affects overlay-mount setup but not by much when image is local
- Real win only when we exercise cold pulls — out of scope for this baseline

### Not worth pursuing right now

- Parallelizing the controller's create-time DB writes — sched→load is already 10–30 ms
- Skipping the controller WS round-trip — same reason
- Resource-limit tuning — affects cgroup setup by maybe single-digit ms

## Recommended sequence if we wanted to act

1. **Surface key-reuse in the SDK** (Tier 1 #3) — cheapest, opens up a whole pattern
2. **Tune `WAIT_TIMEOUT_MS` + trust TCP connect** (Tier 2 #5 + #6) — half a day, predictable
3. **Inline egress-proxy setup into agentd** (Tier 1 #2) — clean structural win
4. **Container pool design doc** (Tier 1 #1) — biggest payoff but biggest change; needs design before code

Steps 1–3 plausibly knock 100–200 ms off the steady-state path on Linux and noticeably more under iptables-lock contention. Step 4 is the order-of-magnitude move when you want to compete with hosted sandbox vendors on cold-start.

## Caveats

- All measurements are local Docker Desktop on macOS. Re-run on a Linux drone before deciding anything — the floor is very different there.
- One drone-1 hang observed during setup (Spawn action received, no container ever created, no error logged). Resolved by restarting the drone container. Worth keeping an eye on — if it recurs it'd dwarf any of the above optimizations in user-perceived reliability.

---

## Measurement update (pass 2) — entrypoint shell wrapper is much more expensive than estimated

The first pass measured `which iptables; which curl` inside the container at 29 ms and used that to estimate "~30 ms overhead from the entrypoint script". That was wrong — `which` lookups don't exercise the actual cost path. Re-ran with the real environment.

Time from `docker start` API call to port-listenable, 3–5 runs each, same node24 image:

| Config | Range | Median |
|---|---:|---:|
| `SANDBOX_EGRESS_PROXY` unset (entrypoint short-circuits) | 450–728 ms | ~632 ms |
| `SANDBOX_EGRESS_PROXY=192.0.2.1:3128` (unreachable; curl fails fast) | 565–1276 ms | ~984 ms |
| `SANDBOX_EGRESS_PROXY=host.docker.internal:3128` (real, reachable) | 733–4035 ms | ~1046 ms |

Two findings:

1. **The dev-fleet baseline ran with egress active.** Drone command line includes `--egress-proxy-addr host.docker.internal:3130`; `crates/plane/src/drone/runtime/docker/commands.rs:112` injects `SANDBOX_EGRESS_PROXY` into every spawned container plus `NET_ADMIN`. So the original 700–1100 ms total time-to-ready numbers already pay this cost.
2. **The egress entrypoint adds ~350 ms median, ~3 s p99.** It's not "30 ms of forks" — it's the `curl http://proxy:3129/admin/ca.crt -o /tmp/egress-proxy-ca.crt` call. Failure modes (unreachable, slow admin endpoint) add seconds. The iptables calls are small in comparison.

This **promotes Tier 1 #2** ("Bake egress-proxy setup into agentd") from "structural cleanup" to "the single biggest local-code win." Doing it in agentd lets us:
- Skip the cert curl on cold path entirely if we cache the CA cert in the container image at build time
- Issue the cert fetch lazily (first network call) instead of pre-`bind`
- Eliminate the tini → sh → 4 forks layer

Revised recommended sequence:

1. **Bake egress-proxy setup into agentd, with CA cert baked into the runtime image at build time** (was Tier 1 #2) — measured 200–400 ms median win, much more under proxy slowness
2. **Surface Plane key-reuse through the SDK** (Tier 1 #3) — still cheap; opens the ~0 ms reuse path
3. **`WAIT_TIMEOUT_MS` 100→25 ms + drop HEAD/read** (Tier 2 #5 + #6) — 30–95 ms steady
4. **Container pool design doc** (Tier 1 #1) — the order-of-magnitude move

Open: validate #1's CA-cert-bake-at-build approach works with cert rotation. If the CA changes per drone deployment, the build-time bake won't fly and the cert curl stays but moves into agentd (still wins by removing the shell wrapper and parallelizing with bind).

---

## Measurement update (pass 3) — what the 350 ms in the entrypoint actually is, and a key-reuse blocker

Decomposing the egress overhead by measuring each step inside a fresh container:

| Step (inside container, fresh boot) | Cost (3 runs) |
|---|---:|
| `getent ahostsv4 host.docker.internal` | 37 / 83 / 268 ms |
| `curl http://proxy:3129/admin/ca.crt` | 110 / 235 / 634 ms |
| `iptables -t nat -A OUTPUT …` (each call) | 4 / 8 / 35 / 67 ms |

Same curl from the host: **0.02 ms**. So almost all of the egress overhead is the **first network-call cost from a brand-new container** — DNS bootstrap + first TCP connect across the container bridge. iptables itself is small noise compared to these.

This refines the Tier 1 #2 framing: the win isn't from removing tini+sh+forks (a direct-agentd interleaved A/B at sample-of-8 showed entrypoint ≈ direct within noise on Docker Desktop). The win is from removing the **synchronous getent + curl** from the pre-bind critical path. Two ways:

- **Bake the CA cert into the runtime image at build time.** Removes the cert-fetch entirely. Requires CA stability per fleet deployment (probably fine; the dev-fleet currently regenerates per egress-proxy startup, but a prod deployment would pin it).
- **Move egress setup to a background task that starts after agentd binds.** Agentd is ready immediately; first outbound HTTP/HTTPS in the sandbox may briefly bypass interception for ~200 ms. Acceptable for most sandbox workloads.

Either gets you most of the way to "no egress" timing (~632 ms p50 vs ~1046 ms p50 — measured pass 2).

### Validated: Plane key-reuse exists but the platform-api breaks it

Ran an A/B: random-key vs same-key creation. First random-key call works (~1.2 s). First same-key call works. **Second same-key call fails with `HTTP 500: Failed to persist sandbox`.**

Root cause:
- Plane's `connect()` reuses the same `backend_id` when a key matches → returns `spawned: false`
- Platform-api's `create_sandbox` then unconditionally calls `controller.db.sandbox().insert()` with the reused backend_id
- `schema/derived_schema.sql:1146`: `sandbox_backend_id_key UNIQUE (backend_id)` — second insert violates uniqueness

So Tier 1 #3 isn't a pure "expose the field" change. The platform-api needs to handle `spawned == false` by returning the existing sandbox row (lookup by backend_id) instead of inserting a new one. Still small, still cheap — but a real change, not just SDK plumbing.

### Net effect on rankings

Order stays the same; the wording sharpens:

1. **Egress: bake CA cert in image OR background the setup after bind** — biggest measured local win, ~400 ms p50, scales harder under proxy slowness
2. **Key-reuse end-to-end** — needs server-side handling of `spawned == false`, then SDK affordance. Still cheap, still the ~0 ms hot path
3. **`WAIT_TIMEOUT_MS` + drop HEAD/read** — one-line ship
4. **Container pool design doc** — order-of-magnitude play

### Noise warning for future runs

Docker Desktop on macOS throttles measurably under sustained container churn. Fresh state: ~500 ms `docker start` floor. After ~30 containers in a session: ~1200–2500 ms. **Always cool down for 30+ seconds between batches, and prefer interleaved A/B over sequential batches** if comparing two configs. Single-batch absolute numbers from a hot Docker daemon are unreliable.

---

## Pass 4 — key-reuse line item shipped (commits a87efcf + 74adfac)

The "Plane reuse exists but the platform-api breaks it" finding from Pass 3 is fixed and verified on the live fleet.

Changes:
- `api_v1::create_sandbox` now tenant-scopes the Plane key namespace as `t:{tenant}/{user_namespace}` so keys can't collide across tenants. Without this, two tenants using `key.name = "shared"` would share a backend (and the UNIQUE constraint that was masking the issue is not a security boundary).
- When `connect_response.spawned == false`, the handler now looks up the existing sandbox row by `(backend_id, tenant)` and returns it with `200 OK` instead of attempting a duplicate `INSERT` that violated `sandbox_backend_id_key UNIQUE`.

Live measurement against the dev fleet (4 calls, same key, same tenant):

| iteration | time-to-ready | initial status |
|---|---:|---|
| 1 (cold) | 838 ms | scheduled |
| 2 | 17 ms | ready |
| 3 | 16 ms | ready |
| 4 | 13 ms | ready |

Cross-tenant test (`tenant-a` and `tenant-b`, both with `key.name = "shared-name"`) returned **distinct** backend IDs as expected.

Coverage:
- Layer 2 (live DB + mock drone): two new `plane-tests` cases — `create_sandbox_with_key_reuses_backend` (asserts 200 + same sandbox id + same backend id on second call) and `create_sandbox_key_is_tenant_scoped` (asserts distinct backend ids across tenants for same key.name). Both pass.
- Layer 4: ad-hoc node script against the dev fleet (results above).

Open follow-ups:
- TypeScript SDK exposes `KeySpec` (generated), but `Sandbox.create()` doesn't surface a sugar like `Sandbox.getOrCreate({ key, ... })`. A small SDK helper would make the pattern discoverable. Not urgent — the contract is honored.
- The reuse path currently does not touch `expires_at`. If a user calls with `timeout_ms` larger than the row's remaining lifetime, the sandbox will still expire on the original schedule. Document as "reuse returns existing sandbox as-is; call `extend()` to refresh." Could be promoted to "extend forward only" in a follow-up if real usage shows people expect that.
- The egress entrypoint cost (Tier 1 #1) is still untouched — biggest remaining local win.
