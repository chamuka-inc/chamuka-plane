# Sandbox Platform — Technical Specification

**Status:** Draft v0.1
**Audience:** Platform engineering, infrastructure security, SDK consumers
**Scope:** Architecture, component contracts, APIs, and rollout plan for an ephemeral compute platform comparable in shape to [Vercel Sandbox](https://vercel.com/docs/vercel-sandbox), built on top of [Plane](https://plane.dev), OCI containers, and a heterogeneous fleet of x86_64 EC2 hosts and ARM64 single-board computers.

---

## Table of contents

1. [Overview](#1-overview)
2. [Goals and non-goals](#2-goals-and-non-goals)
3. [Reference platforms and prior art](#3-reference-platforms-and-prior-art)
4. [System architecture](#4-system-architecture)
5. [Component specifications](#5-component-specifications)
6. [Sandbox lifecycle](#6-sandbox-lifecycle)
7. [Public API and SDK contract](#7-public-api-and-sdk-contract)
8. [In-container API (`agentd`)](#8-in-container-api-agentd)
9. [Networking and egress](#9-networking-and-egress)
10. [Credential brokering](#10-credential-brokering)
11. [Snapshots and persistence](#11-snapshots-and-persistence)
12. [Multi-architecture and trust tiers](#12-multi-architecture-and-trust-tiers)
13. [Computer-use desktop image](#13-computer-use-desktop-image)
14. [Observability and metering](#14-observability-and-metering)
15. [Security model and threat analysis](#15-security-model-and-threat-analysis)
16. [Operational concerns](#16-operational-concerns)
17. [Build phases and milestones](#17-build-phases-and-milestones)
18. [Open questions](#18-open-questions)
19. [References](#19-references)

---

## 1. Overview

This document specifies a multi-tenant platform for running ephemeral, isolated Linux compute environments ("sandboxes") on demand. The platform is designed to safely execute untrusted code — AI-generated programs, user submissions, agent-driven workflows — on a fleet of commodity hosts.

The product shape mirrors Vercel Sandbox: a small SDK that lets callers create a sandbox, run commands inside it, read and write files, expose ports through public URLs, snapshot its state, and stop it. The internal architecture is novel only in that it composes Plane's container-orchestration substrate with additional layers for isolation hardening, snapshot/restore, network egress policy, and credential brokering.

The platform is not a microVM hypervisor stack. We rely on OCI containers as the primary isolation primitive, hardened with userspace kernels (gVisor) or microVM-backed container runtimes (Kata Containers) for adversarial workloads. The rationale and tradeoffs are documented in [§12](#12-multi-architecture-and-trust-tiers) and [§15](#15-security-model-and-threat-analysis).

---

## 2. Goals and non-goals

### 2.1 Goals

- **G1: Vercel-shaped SDK.** A developer using `@vercel/sandbox` ([npm](https://www.npmjs.com/package/@vercel/sandbox), [docs](https://vercel.com/docs/vercel-sandbox/sdk-reference)) should be able to switch to ours with minimal code changes. Same primitives: `create`, `runCommand`, file ops, `domain(port)`, `snapshot`, `stop`.
- **G2: Sub-second cold start for warmed images.** Cold start (image already pulled, no snapshot) ≤ 1.5 s p95. Restore from snapshot ≤ 800 ms p95.
- **G3: Strong isolation for untrusted code.** Kernel-level attacker model: code inside a sandbox cannot read, write, or interfere with other sandboxes or host state, even given a working LPE exploit against the sandbox kernel.
- **G4: Fleet heterogeneity.** Same control plane and SDK regardless of whether the underlying drone is an AWS EC2 c8i, an EC2 c8g, or a Raspberry Pi 5 on a home network.
- **G5: Network policy and credential injection.** Per-sandbox egress allowlists/denylists, with secret material injected by an out-of-sandbox proxy so the sandbox process never sees the credentials it uses.
- **G6: Active-CPU metering.** Bill for CPU time actually consumed inside the sandbox, not wall-clock time. Vercel's pricing page advertises "up to 95% lower cost for workloads with bursty or I/O-bound patterns" using this model ([Vercel Sandbox pricing](https://vercel.com/sandbox)).
- **G7: Persistent and snapshot-able state.** Filesystem snapshots restoreable in milliseconds; persistent sandboxes that auto-save on stop.

### 2.2 Non-goals (v1)

- **NG1: Multi-region.** Vercel Sandbox at GA was [available in one region (iad1)](https://vercel.com/kb/guide/vercel-sandbox-vs-codesandbox); we adopt the same constraint.
- **NG2: GPU workloads.** GPU support requires device passthrough and is deferred.
- **NG3: Persistent network identity.** No sticky IPs, no inbound traffic except via the platform-managed proxy.
- **NG4: In-memory process snapshots.** Filesystem-only snapshotting in v1. True process-state snapshots (CRIU, Firecracker memory snapshots) are a v2 concern.
- **NG5: Custom hypervisor or VMM.** We do not maintain a fork of Firecracker, Cloud Hypervisor, or any other VMM. We may invoke them as runtime backends (via Kata Containers) but do not patch them.

---

## 3. Reference platforms and prior art

| System | Underlying isolation | Notes |
|---|---|---|
| [Vercel Sandbox](https://vercel.com/blog/vercel-sandbox-is-now-generally-available) | Firecracker microVMs on Amazon Linux 2023 | Built on the same Hive infrastructure that handles [2M+ builds per day at Vercel](https://deepwiki.com/vercel/sandbox). Currently iad1 only. |
| [E2B](https://e2b.dev) | Firecracker microVMs | Open-source. Self-hostable. AI-agent focused. |
| [Modal Sandbox](https://modal.com/docs/guide/sandbox) | gVisor | Python-first SDK. |
| [Daytona](https://daytona.io) | Firecracker microVMs | Multi-region, snapshot-based. |
| [CodeSandbox SDK](https://codesandbox.io/docs/sdk) | microVM with memory snapshots | [Memory snapshots resume in 0.5–2 seconds, average ~400 ms in published benchmarks](https://vercel.com/kb/guide/vercel-sandbox-vs-codesandbox). |
| [Cloudflare Sandboxes](https://blog.cloudflare.com/sandboxes-on-cloudflare/) | Containers (not microVM) | Newer, container-shaped. |
| [Plane](https://plane.dev) | OCI containers (Docker) | Open-source session backend orchestrator. We adopt this as our substrate. |
| [Northflank](https://northflank.com/blog/sandboxes-on-kubernetes) | Kata + Firecracker + gVisor on K8s | Enterprise-focused, per-workload isolation tiering. |

### 3.1 Why Plane

Plane describes itself as a [distributed system for running stateful WebSocket backends at scale](https://plane.dev), heavily inspired by Figma's multiplayer infrastructure. Its target use cases [include "running isolated code environments (like REPLs, code notebooks, and LLM agent sandboxes)"](https://github.com/jamsocket/plane).

We use Plane because it solves the orchestration problem we would otherwise build from scratch:

- A controller, drone, and proxy split with a tight contract between them ([Plane architecture](https://plane.dev/concepts/architecture)).
- A lifecycle state machine for backends: [`scheduled → loading → starting → waiting → ready → terminating → hard-terminating → terminated`](https://plane.dev/concepts/backend-lifecycle).
- A consistency primitive (keys with fencing tokens) that guarantees at most one backend exists per key at any time ([How Keys Work](https://plane.dev/developing/keys)).
- Built-in idle detection driven by proxy `KeepAlive` messages that update `backend.last_keepalive`.
- Connection tokens as a bearer-auth pattern that lets us delegate access control to the calling application ([Connection Authorization](https://plane.dev/concepts/auth)).

Plane explicitly leaves out: [container image registry management, observability, autoscaling, network hardening for untrusted code execution, and cloud network configuration](https://plane.dev/plane-vs-jamsocket). These are the layers this specification adds.

---

## 4. System architecture

### 4.1 High-level diagram

```
                          ┌─────────────────────────┐
                          │   Caller (SDK / curl)   │
                          └────────────┬────────────┘
                                       │ HTTPS
                                       ▼
              ┌──────────────────────────────────────────────┐
              │              CONTROL PLANE                   │
              │                                              │
              │   ┌────────────────┐    ┌───────────────┐    │
              │   │ Public API     │    │ Control API   │    │
              │   │  (SDK calls)   │    │  (component   │    │
              │   │                │    │   coord.)     │    │
              │   └───────┬────────┘    └───────┬───────┘    │
              │           │                     │            │
              │   ┌───────▼─────────────────────▼───────┐    │
              │   │   Controller (extended Plane)       │    │
              │   │   - lifecycle, scheduling           │    │
              │   │   - secrets store                   │    │
              │   │   - policy store                    │    │
              │   │   - snapshot registry               │    │
              │   └───────┬─────────────────────┬───────┘    │
              │           │                     │            │
              │   ┌───────▼────────┐    ┌───────▼───────┐    │
              │   │   Postgres     │    │  Object store │    │
              │   │  (state, NOTIFY)│   │  (snapshots)  │    │
              │   └────────────────┘    └───────────────┘    │
              └──────────┬──────────────────────────┬────────┘
                         │ WebSocket                │ WebSocket
                         ▼                          ▼
                  ┌──────────────┐           ┌──────────────┐
                  │    Proxy     │           │    Proxy     │
                  │ (TLS, route) │           │              │
                  └──────┬───────┘           └──────┬───────┘
                         │                          │
        ┌────────────────┼──────────────────────────┼─────────────────┐
        │                ▼                          ▼                 │
        │       ┌─────────────────────┐    ┌─────────────────────┐    │
        │       │  DRONE (host agent) │    │  DRONE (host agent) │    │
        │       │  - Plane drone      │    │                     │    │
        │       │  - Docker / runsc / │    │                     │    │
        │       │     Kata            │    │                     │    │
        │       │  - Egress proxy     │    │                     │    │
        │       │  - Metering         │    │                     │    │
        │       └──────────┬──────────┘    └─────────────────────┘    │
        │                  │                                          │
        │                  ▼                                          │
        │       ┌─────────────────────┐                               │
        │       │ Sandbox container   │                               │
        │       │  ┌──────────────┐   │                               │
        │       │  │   agentd     │   │  ◄── HTTP/WS via proxy        │
        │       │  │  (port 8080) │   │                               │
        │       │  └──────────────┘   │                               │
        │       │  ┌──────────────┐   │                               │
        │       │  │ user workload│   │                               │
        │       │  └──────────────┘   │                               │
        │       └─────────────────────┘                               │
        │                                                             │
        │                  HOST (EC2 c8i / c8g / Pi 5)                │
        └─────────────────────────────────────────────────────────────┘
```

### 4.2 Planes of operation

The architecture follows a strict control-plane / data-plane separation, inherited from Plane and reinforced by our additions.

- **Control plane** — Stateful coordination. Everything that involves writing to Postgres, scheduling, policy evaluation, or secret retrieval lives here. Communication between control-plane components is HTTP and WebSocket; the source of truth is Postgres ([Plane Design Principles](https://plane.dev/developing/design-principles)).
- **Data plane** — Per-request HTTP/WebSocket traffic from end users into a specific sandbox's `agentd` HTTP server. Routed by the Plane proxy using a per-sandbox connection token. The controller is not on this path.
- **Egress plane** — Outbound traffic from a sandbox to the wider internet, intercepted by a per-drone egress proxy that applies network policy and credential injection ([§9](#9-networking-and-egress), [§10](#10-credential-brokering)).
- **Metering plane** — Periodic batched delivery of cgroup-derived CPU/memory statistics from drones to the controller, then to a billing pipeline. Eventual consistency is acceptable.

### 4.3 Why a separate egress proxy

Plane's data plane handles inbound HTTP to sandboxes via connection tokens, but does nothing about outbound traffic from a sandbox. Vercel Sandbox's [credential brokering and dynamic network policy](https://vercel.com/sandbox) require an outbound interception point. The egress proxy is the simplest place to do both: SNI-based domain matching, CIDR rules, and header injection in one component, running on each drone host inside a separate network namespace.

---

## 5. Component specifications

### 5.1 Controller

**Implementation:** Fork of [Plane's controller](https://github.com/jamsocket/plane), extended with our additional responsibilities. Rust.

**Responsibilities:**

1. Accept SDK requests over HTTP (`/v1/sandboxes`, etc. — see [§7](#7-public-api-and-sdk-contract)).
2. Translate SDK calls into Plane connect-requests with our extended `spawn_config`.
3. Maintain the canonical state of every sandbox, drone, snapshot, network policy, and credential binding in Postgres.
4. Schedule new sandboxes onto drones based on `{arch, trust_tier, capacity, cluster}`.
5. Push policy updates and credential bindings to drones via the durable `extension_action` queue (mirroring Plane's `backend_action` redelivery pattern); on each row write, NOTIFY the controller replica holding the drone's WebSocket, which then sends a `PushNetworkPolicy` or `PushCredential` message (our extension variants of `MessageToDrone`).
6. Aggregate metering data and write it to the billing table.
7. Issue connection tokens; mint per-port subdomain routing entries.

**Statelessness:** The controller is stateless across replicas. All state lives in Postgres. The [Plane design principle](https://plane.dev/developing/design-principles) of "business logic in the controller, dumb executors elsewhere" is preserved.

**HTTP surface:** Two prefixes, both served from the same process:
- `/pub/*` — public, exposed to the internet behind your reverse proxy. SDK calls.
- `/ctrl/*` — control plane only; reachable from drones and proxies. The reverse proxy must restrict this prefix ([Plane prod deployment guide](https://plane.dev/deploy-to-prod)).

### 5.2 Drone

**Implementation:** Fork of Plane's drone, extended. Rust. Runs as a system service on every host.

**Responsibilities:**

1. Maintain a WebSocket connection to the controller on `/ctrl/c/:cluster/drone-socket`. Capabilities (arch, supported runtimes, pool) are set by CLI flags and persisted in the `drone` and `node` table rows on registration — there is no application-level hello message.
2. Accept `Spawn` and `Terminate` actions via Plane's `MessageToDrone::Action(BackendActionMessage)`; invoke the appropriate OCI runtime (`runc`, `runsc`, or `kata-runtime`) per sandbox via Docker's `HostConfig.Runtime`.
3. Accept our extension actions: `PushNetworkPolicy`, `PushCredential`, `RevokeCredential`, `Snapshot`. Dedupe on `action_id` because Plane's delivery is at-least-once (action rows redelivered on reconnect until acked).
4. Emit `MessageFromDrone::BackendEvent` on every state transition with a monotonically-increasing `event_id` per drone session; emit `BackendMetrics` for CPU (nanoseconds) and memory (bytes); emit `Heartbeat` at the heartbeat interval; emit `AckAction` on completion of each action.
5. Run a local egress proxy in the root network namespace; install per-sandbox iptables rules in each sandbox's netns so all outbound traffic redirects there.
6. Periodically read cgroup v2 statistics (`cpu.stat: usage_usec`, `memory.current`, `io.stat`) for each container; convert CPU to nanoseconds for `BackendMetrics`.
7. Execute snapshot via `docker commit + tag + push` to the local registry, then register via our `POST /ctrl/snapshots` endpoint.
8. Execute restore via `docker pull` of the snapshot image, then `docker create + start`.

**Process model on the host:**

```
systemd unit: sandbox-drone.service
└─ plane-drone
   ├─ Docker socket (controls containerd / runc / runsc / kata-runtime)
   ├─ Egress proxy subprocess (envoy or custom Rust binary)
   └─ Metering loop
```

**Container creation:** Drones use Docker's HTTP API rather than the CLI. The runtime is selected per-container via `HostConfig.Runtime`. Multiple runtimes can coexist on one host; the drone advertises which it supports.

### 5.3 Proxy

**Implementation:** Plane proxy, lightly extended.

**Stock Plane proxy behavior:** [Terminates TLS, accepts public traffic on 443, routes by connection token to the correct drone:container_port](https://plane.dev/concepts/architecture). It does this by **asking the controller for route info on demand** (`MessageFromProxy::RouteInfoRequest { token }`) and caching the response, not by holding a synced table. Coordinates with the controller for ACME-issued certificates and emits `KeepAlive(BackendName)` for backends with live traffic so the controller can drive idle-timeout termination.

**Our extensions:**

1. **Per-port subdomain routing.** Vercel's `sandbox.domain(port)` exposes a port at `https://{id}-{port}.sandboxes.example.com`. We extend the proxy to parse the subdomain, look up `{sandbox_id, port}` in a controller-fed cache, and route accordingly. A wildcard ACME-DNS-01 cert handles the subdomain space.
2. **Header forwarding for `agentd`.** When proxying to `agentd`, set `X-Sandbox-Id`, `X-Connection-Token`, and the auth headers documented in [Plane's Connection Authorization](https://plane.dev/concepts/auth) (`x-verified-username`, `x-verified-auth`).

### 5.4 Egress proxy

**Implementation:** Envoy with a custom xDS feeder, or a small custom Rust HTTP/SOCKS proxy. Recommended start: Envoy (battle-tested SNI inspection, simple config), revisit if config update latency becomes a bottleneck.

**Network position:** Runs in the drone host's root network namespace. Each sandbox container is configured (via iptables rules in its netns) such that all outbound traffic — DNS, HTTP, HTTPS — must traverse the proxy.

**Responsibilities:**

1. Inspect outbound connections. For TLS, parse SNI from the ClientHello. For plain HTTP, parse the Host header.
2. Apply the sandbox's network policy:
   - `allow-all` — default for the trusted tier.
   - `deny-all` — blocks everything, including DNS lookups.
   - `user-defined` — list of `{domains, cidrs}` matched against the connection. [Vercel docs describe denied ranges as always taking precedence over allowed ranges and domain-based rules](https://vercel.com/kb/guide/vercel-sandbox-vs-codesandbox).
3. For allowed connections, optionally inject credentials from the controller-provided binding ([§10](#10-credential-brokering)).
4. Forward the connection to the destination. For TLS, the proxy does *not* terminate; it acts as a SNI-aware passthrough except when credential injection is configured for that destination, in which case it MITMs with a per-sandbox internal CA that `agentd` trusts.
5. Emit per-connection logs to the controller for audit.

**Policy update path:** Controller writes policy to Postgres → controller pushes update to drone over WebSocket → drone reloads egress proxy xDS config (or sends a SIGHUP). Update latency target: < 500 ms p95.

### 5.5 `agentd`

**Implementation:** Single static binary, Go or Rust. Built per-runtime (one variant for the headless flavor, one for the desktop flavor — see [§13](#13-computer-use-desktop-image)).

**Responsibilities:**

1. Bind an HTTP/WebSocket server on a known port (8080) inside the container.
2. Expose a stable API surface for shell exec, filesystem operations, port introspection, and (for desktop variant) mouse/keyboard/screenshot.
3. Optionally run user-provided startup hooks (declared in the image's `LABEL` or in the sandbox spawn config).
4. Stream stdout/stderr of long-running commands over WebSocket.
5. Expose `/health` for the drone's readiness check.

Full API surface documented in [§8](#8-in-container-api-agentd).

### 5.6 Postgres schema (essential tables)

> **Source of truth:** the full, verified Postgres schema (Plane's tables + our extensions, with all columns, indexes, partial indexes, and foreign keys) lives in [`sandbox-platform-interfaces.md` §15](./sandbox-platform-interfaces.md#15-postgres-schema). The fragment below is the conceptual model only.

The platform's database is two parts:

- **Plane's tables** (verified against `plane/schema/derived_schema.sql`): `controller`, `node`, `drone`, `backend`, `backend_state`, `backend_action`, `backend_key`, `token`, `acme_txt_entries`, `event`. Drone IDs are integers (`node.id`); backend IDs are strings (`VARCHAR(255)`). The `backend_action` table is the durable command queue with an `acked_at` column for at-least-once redelivery.
- **Our extensions**, added in our migration, referencing Plane's tables by FK.

Essential shape of our extension tables (illustrative; see the interfaces doc for the authoritative DDL):

```sql
-- Our wrapper over Plane's backend. 1:1 with backend.id.
CREATE TABLE sandbox (
    id              VARCHAR(255) PRIMARY KEY,                       -- "sb-..."
    backend_id      VARCHAR(255) NOT NULL UNIQUE REFERENCES backend(id),
    tenant          VARCHAR(255) NOT NULL REFERENCES tenant(id),
    runtime         VARCHAR(255) NOT NULL,
    trust_tier      VARCHAR(64)  NOT NULL,                          -- 'trusted'|'untrusted'|'paranoid'
    arch            VARCHAR(16)  NOT NULL,
    vcpus           INTEGER NOT NULL,
    memory_mb       INTEGER NOT NULL,
    ports           INTEGER[] NOT NULL DEFAULT '{}',
    tags            JSONB     NOT NULL DEFAULT '{}',
    persistent      BOOLEAN   NOT NULL DEFAULT false,
    snapshot_source VARCHAR(255) REFERENCES snapshot(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at      TIMESTAMPTZ NOT NULL,
    ready_at        TIMESTAMPTZ,
    terminated_at   TIMESTAMPTZ
);

-- Snapshots (docker commit + push).
CREATE TABLE snapshot (
    id            VARCHAR(255) PRIMARY KEY,                         -- "snap-..."
    sandbox_id    VARCHAR(255) NOT NULL REFERENCES sandbox(id) ON DELETE CASCADE,
    tenant        VARCHAR(255) NOT NULL REFERENCES tenant(id),
    image_ref     TEXT NOT NULL,
    digest        VARCHAR(128) NOT NULL,
    base_image    TEXT NOT NULL,
    size_bytes    BIGINT NOT NULL,
    arch          VARCHAR(16) NOT NULL,
    runtime       VARCHAR(255) NOT NULL,
    name          TEXT,
    tags          JSONB NOT NULL DEFAULT '{}',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at    TIMESTAMPTZ NOT NULL
);

-- Network policy (1 row per sandbox).
CREATE TABLE network_policy (
    sandbox_id VARCHAR(255) PRIMARY KEY REFERENCES sandbox(id) ON DELETE CASCADE,
    version    BIGINT  NOT NULL DEFAULT 1,                          -- monotonic per sandbox
    mode       VARCHAR(64) NOT NULL,                                -- 'allow-all'|'deny-all'|'rules'
    rules      JSONB   NOT NULL DEFAULT '[]',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Secret (KMS envelope-encrypted at rest).
CREATE TABLE secret (
    id                VARCHAR(255) PRIMARY KEY,                     -- "secret-..."
    tenant            VARCHAR(255) NOT NULL REFERENCES tenant(id),
    kms_key_id        TEXT  NOT NULL,
    wrapped_data_key  BYTEA NOT NULL,
    nonce             BYTEA NOT NULL,
    ciphertext        BYTEA NOT NULL,
    auth_tag          BYTEA NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at        TIMESTAMPTZ
);

-- Credential bindings (which secret is injected for which sandbox + destination).
CREATE TABLE credential_binding (
    id            VARCHAR(255) PRIMARY KEY,                          -- "cred-..."
    sandbox_id    VARCHAR(255) NOT NULL REFERENCES sandbox(id) ON DELETE CASCADE,
    secret_id     VARCHAR(255) NOT NULL REFERENCES secret(id),
    destination   TEXT NOT NULL,
    header_name   TEXT NOT NULL,
    header_format TEXT NOT NULL DEFAULT '{value}',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at    TIMESTAMPTZ
);

-- Usage metering (hourly buckets).
-- NOTE: cpu_nsec is NANOSECONDS to match Plane's BackendMetricsMessage.cpu_used unit.
-- The drone converts from cgroup v2's usage_usec (microseconds) on its way to us.
CREATE TABLE usage_record (
    sandbox_id        VARCHAR(255) NOT NULL REFERENCES sandbox(id) ON DELETE CASCADE,
    bucket_start      TIMESTAMPTZ NOT NULL,
    cpu_nsec          BIGINT NOT NULL,
    memory_mb_seconds BIGINT NOT NULL,
    rx_bytes          BIGINT NOT NULL DEFAULT 0,
    tx_bytes          BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (sandbox_id, bucket_start)
);

-- Outbound action queue for OUR extension actions, mirroring Plane's backend_action.
-- Used for push-network-policy, push-credential, revoke-credential, snapshot.
CREATE TABLE extension_action (
    id         VARCHAR(255) PRIMARY KEY,                            -- "xak-..."
    kind       VARCHAR(64) NOT NULL,
    backend_id VARCHAR(255) REFERENCES backend(id),
    sandbox_id VARCHAR(255) REFERENCES sandbox(id),
    drone_id   INTEGER NOT NULL REFERENCES drone(id),
    payload    JSONB   NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    acked_at   TIMESTAMPTZ
);
```

Indexes, partial indexes (e.g., `WHERE acked_at IS NULL` for redelivery scans), additional tables (`tenant`, `port_route`, `audit_egress`), and the migration plan are in the interfaces doc.

---

## 6. Sandbox lifecycle

The sandbox lifecycle extends [Plane's backend lifecycle](https://plane.dev/concepts/backend-lifecycle), which defines the canonical states `scheduled → loading → starting → waiting → ready → terminating → hard-terminating → terminated`. We add semantic layers without changing the underlying state machine.

### 6.1 Cold start (no snapshot)

```
1. SDK: Sandbox.create({ runtime: 'node24', resources: { vcpus: 2 }, ... })
2. Controller: validate request, pick cluster by (arch, trust_tier)
3. Controller: invoke Plane connect-request with extended spawn_config:
   {
     executable: {
       image: 'ghcr.io/yourorg/sandbox-runtime-node24:latest',
       env: {...},
       runtime_class: 'runsc',   // chosen from trust_tier
     },
     max_idle_seconds: 300,
     lifetime_limit_seconds: 900
   }
4. Plane controller: acquire key, choose drone, dispatch over WS
5. Drone: docker pull (cached or fast pull) -- backend.status = loading
6. Drone: docker create with HostConfig.Runtime = 'runsc',
   apply iptables for egress redirection, start container
   -- backend.status = starting
7. Container: agentd binds :8080
   -- drone polls /health -- backend.status = ready
8. Controller: mint connection token, build response
9. SDK receives: { id, domain, connectionToken, ... }
```

Target p95 timings:

| Phase | Target |
|---|---|
| Steps 1–4 (control plane) | < 50 ms |
| Step 5 (image pull, cached) | < 100 ms |
| Step 6 (container start, runsc) | < 300 ms |
| Step 7 (agentd boot + health) | < 200 ms |
| Step 8–9 (response delivery) | < 50 ms |
| **Total cold start (p95)** | **< 1.5 s** |

Image must be pre-pulled on the target drone for these numbers to hold. The metering and scheduling subsystems together ensure that for each `(image, arch, drone)` combination, drones eagerly pull on first reference and retain.

### 6.2 Warm start (from snapshot)

```
1. SDK: Sandbox.fromSnapshot('snap-...')
2. Controller: look up snapshot, validate ownership and arch compatibility
3. Controller: pick drone matching arch and trust_tier
4. Drone: pull base image (cached) + download overlay blob from object store
5. Drone: mount overlay as the writable layer (overlayfs over the base rootfs)
6. Drone: start container with the merged rootfs, agentd already configured
7. agentd binds, drone reports ready
```

Target: p95 < 800 ms. The two main variables are overlay size and object store latency. For overlays under 100 MB on a drone with a warm base image, this is achievable.

### 6.3 Termination paths

Following [Plane's lifecycle](https://plane.dev/concepts/backend-lifecycle), a sandbox terminates because:

- **Explicit:** `sandbox.stop()` from the SDK, or `terminate` from the control API.
- **Self-exit:** The container's PID 1 exited. For our images, PID 1 is `tini`, which only exits if `agentd` exits.
- **Idle timeout:** No `KeepAlive` from any proxy for `max_idle_seconds` (Plane's `backend.allowed_idle_seconds`). The controller compares `now() - backend.last_keepalive` to decide.
- **Lifetime limit:** `lifetime_limit_seconds` elapsed since creation. Hard cap.
- **Active CPU budget:** New for v1 — terminate when cumulative cgroup `cpu.stat:usage_usec` exceeds the configured budget.

On stop, the drone:
1. Sends `SIGTERM` to the container — `terminating`.
2. Waits `grace_seconds` (default 10s).
3. Sends `SIGKILL` if still running — `hard-terminating`.
4. If `persistent: true`, runs the snapshot procedure ([§11](#11-snapshots-and-persistence)) before tearing down.
5. Tears down: iptables rules, tap interfaces, overlay mounts, cgroups.
6. Reports `terminated`.

### 6.4 Persistent sandboxes

Vercel describes persistent sandboxes as ["sandboxes that auto-save state on stop and resume where you left off — no manual snapshot management needed"](https://vercel.com/docs/vercel-sandbox).

Our implementation:
- The `persistent: true` flag is set at creation.
- On any termination path, the drone snapshots the sandbox before final teardown, writes the snapshot id to `sandbox.snapshot_source`.
- On the next `Sandbox.resume(id)` (a v1 SDK method we add), the controller looks up the latest snapshot for that id and follows the warm-start path.
- Persistent sandboxes have a configurable retention TTL; snapshots beyond it are GC'd.

---

## 7. Public API and SDK contract

The HTTP API lives under `/pub/v1/` on the controller. The SDK is a thin wrapper. The shape mirrors [`@vercel/sandbox`](https://www.npmjs.com/package/@vercel/sandbox).

### 7.1 SDK example

```typescript
import { Sandbox } from '@yourorg/sandbox'

const sandbox = await Sandbox.create({
  runtime: 'node24',           // 'node22' | 'node24' | 'python3.13' | 'bun1' | 'custom'
  resources: { vcpus: 2 },     // memory = 2 GB per vcpu, matching Vercel
  timeout: 15 * 60 * 1000,     // ms; max depends on plan (45 min Hobby / 5 hr Pro/Enterprise upstream)
  trust: 'untrusted',          // OUR EXTENSION — not in @vercel/sandbox.
                               // 'trusted' | 'untrusted' | 'paranoid'
  source: {
    type: 'git',
    url: 'https://github.com/...',
    revision: 'main',          // commit, branch, or tag
  },
  ports: [3000, 8080],         // up to 4
  tags: { team: 'platform', env: 'preview' },
})

// Run a command, wait for completion. Either positional or object form works.
const result = await sandbox.runCommand('npm', ['install'])
console.log(result.exitCode, await result.stdout())

// Run detached. NOTE: `detached: true` must go INSIDE the params object;
// it is NOT a third argument on the positional form. (Verified against
// @vercel/sandbox's runCommand overloads.)
const dev = await sandbox.runCommand({
  cmd: 'npm',
  args: ['run', 'dev'],
  detached: true,
  stdout: process.stdout,      // pipe live, or use logs() below
  stderr: process.stderr,
})

// Alternative: stream logs explicitly.
for await (const log of dev.logs()) {
  if (log.stream === 'stdout') process.stdout.write(log.data)
  else process.stderr.write(log.data)
}

// File ops
await sandbox.writeFiles([{ path: '/app/.env', content: 'KEY=value' }])
const env = await sandbox.readFile('/app/.env')

// Expose a port
const url = sandbox.domain(3000) // -> https://sb-abc123-3000.sandboxes.example.com

// Network policy
await sandbox.updateNetworkPolicy({
  mode: 'rules',
  rules: [
    { domains: ['*.npmjs.org', 'registry.npmjs.org'] },
    { domains: ['api.example.com'], inject: 'cred-xyz' },  // OUR EXTENSION
  ],
})

// Snapshot and stop
const snap = await sandbox.snapshot()
await sandbox.stop()

// Resume from a snapshot. NOTE: there is no Sandbox.fromSnapshot() in
// @vercel/sandbox; restore is via the same Sandbox.create() with a
// snapshot source. (Verified against sandbox.ts line 156.)
const resumed = await Sandbox.create({
  source: { type: 'snapshot', snapshotId: snap.id },
  resources: { vcpus: 2 },
  timeout: 15 * 60 * 1000,
})
```

### 7.2 HTTP endpoints

| Method | Path | Description |
|---|---|---|
| `POST` | `/pub/v1/sandboxes` | Create a sandbox. |
| `GET` | `/pub/v1/sandboxes/:id` | Get sandbox state. |
| `GET` | `/pub/v1/sandboxes/:id/status` | SSE stream of lifecycle events. |
| `DELETE` | `/pub/v1/sandboxes/:id` | Stop a sandbox. |
| `POST` | `/pub/v1/sandboxes/:id/snapshot` | Snapshot. Returns `{ id, size_bytes }`. |
| `POST` | `/pub/v1/sandboxes/:id/network-policy` | Update egress policy. |
| `POST` | `/pub/v1/snapshots/:id/restore` | Create a new sandbox from snapshot. |
| `GET` | `/pub/v1/sandboxes` | List, with tag filters. |

All sandbox interaction beyond lifecycle (running commands, file ops, etc.) happens directly against the per-sandbox `agentd` over the data plane, using the connection token returned at create time. The controller is not on the hot path.

### 7.3 Authentication

Out of scope for this spec to specify the auth method (OIDC, API tokens, mTLS) — the controller is designed to sit behind a reverse proxy that handles auth, [matching Plane's design](https://plane.dev/deploy-to-prod). The reverse proxy injects an authenticated principal as `X-Auth-User`, which the controller uses for ownership checks and tagging.

---

## 8. In-container API (`agentd`)

`agentd` is the only HTTP server inside a sandbox container that the platform talks to. It binds `0.0.0.0:8080`.

### 8.1 Endpoints (headless flavor)

```
GET    /health                          → 200 once ready
GET    /info                            → { runtime, arch, pid1, ... }

# Shell
POST   /v1/exec                         → run a command, return result
       body: { argv, env?, cwd?, stdin?, timeout_ms?, detached? }
GET    /v1/exec/:cmd_id                 → poll status of a detached command
GET    /v1/exec/:cmd_id/stdout (WS)     → stream stdout
GET    /v1/exec/:cmd_id/stderr (WS)     → stream stderr
POST   /v1/exec/:cmd_id/signal          → send a signal
POST   /v1/exec/:cmd_id/stdin           → write to stdin

# Filesystem
GET    /v1/fs/read?path=...             → file bytes
PUT    /v1/fs/write                     → multipart upload
POST   /v1/fs/mkdir                     → { path, mode? }
DELETE /v1/fs/rm?path=...&recursive=...
GET    /v1/fs/list?path=...             → directory listing with stat
POST   /v1/fs/move                      → { from, to }
GET    /v1/fs/watch?path=... (WS)       → file change events

# Ports
GET    /v1/ports                        → listening sockets in the container's netns

# Process tree
GET    /v1/processes                    → ps-style list of running processes
```

This API surface aligns with what [Vercel's Sandbox SDK exposes](https://vercel.com/docs/vercel-sandbox/sdk-reference) and what [`@agent-infra/sandbox`](https://github.com/agent-infra/sandbox) implements as a reference open-source equivalent.

### 8.2 Endpoints (desktop flavor)

In addition to the headless endpoints, `agentd` exposes computer-use actions. The action names, parameter names, and shapes are kept compatible with [Anthropic's computer-use tool schema](https://github.com/anthropics/anthropic-quickstarts/blob/main/computer-use-demo/computer_use_demo/tools/computer.py). OpenAI's equivalent uses different names; we provide a translation layer rather than diverge.

**Canonical endpoint** (preferred, mirrors Anthropic's tool exactly):

```
POST /v1/computer
  body: { action, coordinate?, text?, scroll_direction?, scroll_amount?,
          duration?, region?, start_coordinate? }
  → { output?: { type: "image", base64: "..." } | { type: "text", text: "..." } }
```

Where `action` is one of (per `Action_20251124` in Anthropic's source):

| Action | Required params | Notes |
|---|---|---|
| `screenshot` | — | Returns base64 PNG. |
| `cursor_position` | — | Returns `{x, y}`. |
| `mouse_move` | `coordinate: [x, y]` | |
| `left_click` | `coordinate: [x, y]` | Optionally `text` for modifier (e.g., `"ctrl"`). |
| `right_click` | `coordinate: [x, y]` | |
| `middle_click` | `coordinate: [x, y]` | |
| `double_click` | `coordinate: [x, y]` | |
| `triple_click` | `coordinate: [x, y]` | Added in `computer_20250124`. |
| `left_click_drag` | `start_coordinate: [x, y]`, `coordinate: [x, y]` | Drag from start to end. |
| `left_mouse_down` | `coordinate: [x, y]` | Fine-grained mouse control (20250124+). |
| `left_mouse_up` | `coordinate: [x, y]` | (20250124+) |
| `key` | `text: "Return"` or `"ctrl+c"` | xdotool-style key combinations. |
| `type` | `text` | Types literal text. |
| `hold_key` | `text`, `duration` | Hold a key for N seconds (20250124+). |
| `scroll` | `coordinate: [x, y]`, `scroll_direction`, `scroll_amount` | Direction: `up`/`down`/`left`/`right`. (20250124+) |
| `wait` | `duration` | Sleep N seconds. (20250124+) |
| `zoom` | `region: [x1, y1, x2, y2]` | View a region at full resolution. Requires `enable_zoom: true` in tool definition. (20251124+) |

All coordinates are `[x, y]` arrays, not objects, to match Anthropic's tool exactly.

Notes:
- `key` and `type` differ semantically: `key` takes xdotool-compatible key combinations (`"Return"`, `"ctrl+c"`, `"shift+Tab"`); `type` takes literal text to be typed character by character.
- Clipboard and X11-selection manipulation are **not** in Anthropic's schema. If you need them, expose `/v1/shell/exec` with `xclip` or `xsel`. Do not invent a `/v1/computer/clipboard` API that won't match the model's expectations.

The reference implementation in `anthropics/anthropic-quickstarts/computer-use-demo` uses `xdotool` and `gnome-screenshot` (falling back to `scrot`) under the hood. Our `agentd` can use the same tools at first, then move to direct XTest/XGetImage calls for latency once the action surface is stable.

### 8.3 Implementation notes

- **PID 1.** `agentd` runs under `tini` to handle zombie reaping for spawned processes. `tini` exits when `agentd` exits, which terminates the container.
- **Concurrency.** `agentd` is a single async runtime (Tokio for Rust, or Go). Per-command goroutines/tasks for execution.
- **Streaming.** WebSocket framing: each frame is one of `{stream: 'stdout'|'stderr', data: bytes}` | `{stream: 'exit', code: int}`. Equivalent JSON or msgpack representation is acceptable.
- **Resource limits.** Per-exec rlimits configurable via the spawn request. Hard caps inherited from the container's cgroup; cannot exceed.
- **Authentication inside.** `agentd` does not authenticate callers — the Plane proxy is the trust boundary. `agentd` listens only on the container's interface; nothing outside the proxy path can reach it.

### 8.4 Build artifacts

```
images/
  base-amd64.Dockerfile       → minimal Debian + tini + agentd
  base-arm64.Dockerfile       → same for ARM64
  runtime-node24.Dockerfile   → FROM base + Node 24
  runtime-python313.Dockerfile
  runtime-bun1.Dockerfile
  runtime-desktop.Dockerfile  → FROM base + Xvfb, openbox, chromium
```

All built with `docker buildx` for `linux/amd64,linux/arm64`. Published to a private registry, with content-addressed tags for reproducibility.

---

## 9. Networking and egress

### 9.1 Per-sandbox networking

Each container gets its own Linux network namespace, virtual ethernet pair (veth) into a per-drone bridge, and a private RFC-1918 IP from a host-local pool. This is standard Docker behavior; nothing custom.

### 9.2 Egress interception

In each container's netns, the drone installs iptables rules that redirect all outbound TCP to the egress proxy:

```
# inside the container netns, conceptually:
iptables -t nat -A OUTPUT -p tcp --dport 80  -j DNAT --to <proxy>:80
iptables -t nat -A OUTPUT -p tcp --dport 443 -j DNAT --to <proxy>:443
iptables -t nat -A OUTPUT -p udp --dport 53  -j DNAT --to <proxy>:53
iptables -A OUTPUT -j REJECT    # block everything else
```

DNS resolution goes through a small resolver inside or in front of the egress proxy, which consults the same policy. This is necessary because without DNS interception, `deny-all` would allow arbitrary DNS exfiltration via lookup of `secret.attacker.com`.

### 9.3 Policy schema

```json
{
  "mode": "rules",
  "rules": [
    { "domains": ["*.npmjs.org", "registry.npmjs.org"], "action": "allow" },
    { "cidrs":   ["10.0.0.0/8"],                        "action": "deny"  },
    { "domains": ["api.example.com"], "inject": "cred-xyz" }
  ]
}
```

Matching order, [following Vercel's documented behavior](https://vercel.com/kb/guide/vercel-sandbox-vs-codesandbox):
1. Explicit denies always win.
2. Rules with `inject` imply allow.
3. Then domain rules.
4. Then CIDR rules.
5. Default: deny if `mode = rules`.

### 9.4 SNI matching

For HTTPS, the proxy parses the TLS ClientHello and matches `server_name` against the policy. Wildcards (`*.example.com`) are supported one level deep. If the connection has no SNI (old clients), it is denied under `mode = rules`. This is a deliberate choice: SNI-less TLS is incompatible with policy enforcement.

### 9.5 Inbound

Inbound traffic is exclusively via the Plane proxy. The sandbox's container has no public IP. Ports exposed via `sandbox.domain(port)` are routed by the proxy from a per-sandbox subdomain to the right `drone:port` mapping. There is no other inbound path.

---

## 10. Credential brokering

The point of credential brokering is to let a sandbox make authenticated outbound requests without ever holding the credential itself. Vercel describes this as ["credentials injected on egress: never enter sandbox scope"](https://vercel.com/sandbox).

### 10.1 Threat model

We protect against:
- Code in the sandbox reading the credential from `os.environ`, files, or process memory.
- Code in the sandbox exfiltrating the credential to an unauthorized destination.
- A sandbox sharing its credentials with another sandbox.

We do *not* protect against:
- A sandbox using its allowed access maliciously against the configured destination. If you scope a credential to `api.openai.com`, the sandbox can spend all the tokens.

### 10.2 Lifecycle

```
1. Operator creates a secret (tenant inferred from auth, not in body):
   POST /pub/v1/secrets
   { value: "Bearer sk-...", name: "openai-team-x" } -> { id: "secret-abc" }

2. Operator binds the secret to a sandbox and destination:
   POST /pub/v1/sandboxes/sb-123/credentials
   { destination: "api.openai.com", header_name: "Authorization", secret_id: "secret-abc" }

3. The sandbox's egress proxy is now configured to MITM TLS to api.openai.com,
   strip any client-provided Authorization header, inject the bound one,
   and forward.

4. Code inside the sandbox does:
     fetch('https://api.openai.com/v1/chat/completions', { ... })
   It never sets an Authorization header. The proxy adds it.

5. Sandbox terminates. Binding is dropped (cascading delete in Postgres).
```

### 10.3 MITM mechanics

For destinations where injection is configured, the egress proxy:
1. Terminates the inbound TLS connection from the sandbox using a per-sandbox internal-CA certificate. `agentd` installs the internal CA root into the container's trust store at boot.
2. Establishes a fresh TLS connection to the real destination.
3. On the request, injects the configured header.
4. Pipes bytes between the two connections.

The internal CA is per-sandbox to avoid a single CA breach being a fleet-wide compromise. The CA private key never leaves the egress proxy.

For destinations *without* injection, the proxy passes the TLS connection through without MITM. This is important: most destinations don't need injection, and MITMing them gains nothing and risks issues with certificate pinning (e.g., mobile API clients).

### 10.4 Secret storage

Secrets are encrypted at rest in Postgres using KMS-derived data keys. Decryption happens only in the controller's memory, and decrypted secrets are pushed to drones over the control WebSocket only when a sandbox bound to that secret is scheduled to that drone. The drone holds the decrypted secret in memory only for the lifetime of the sandbox.

---

## 11. Snapshots and persistence

### 11.1 What we snapshot

We snapshot the **filesystem state** of a container: specifically, the writable overlay layer above the base image. We do *not* snapshot in-memory process state in v1.

Rationale: Vercel Sandbox markets snapshots as capturing ["the complete state of a running sandbox (filesystem and installed packages)"](https://vercel.com/sandbox) — note the absence of "running processes." This is sufficient for the killer use case (skip `npm install`) and dramatically easier than CRIU- or VMM-based memory snapshots.

We may add memory snapshots later via Kata + Firecracker on KVM-capable drones, but that is a v2 concern.

### 11.2 Snapshot procedure

We use `docker commit` (or its containerd equivalent for non-Docker setups), which produces a proper OCI image layer from the running container's filesystem changes. This handles whiteouts correctly, is independent of the storage driver, and integrates with the registry we already run.

```
1. agentd in the sandbox: optionally run pre-snapshot hooks
   (sync filesystems, flush logs, stop background writers).
2. Drone: docker pause <container>          # SIGSTOP-equivalent
3. Drone: docker commit <container> snap:<id>
4. Drone: docker tag snap:<id> <registry>/snap:<id>
5. Drone: docker push <registry>/snap:<id>  # to the host's pull-through registry
6. Drone: read image digest, register snapshot in Postgres via controller WS:
          { id, base_image, image_ref, digest, size_bytes, arch }
7. Drone: docker unpause <container>
8. Drone: docker rmi snap:<id> (local cleanup; registry retains)
9. Controller: respond to SDK with { id, size_bytes }
```

Target: < 2 s for snapshots under 50 MB of diff. Time scales roughly linearly with diff size and registry write throughput. The container is paused for the duration of `docker commit`, which scans the upper layer — for very large diffs this is a real pause, and we should warn users.

`docker commit` writes a new OCI image layer to the local image store; only the *diff* from the base image becomes the new layer, so multi-GB base images don't blow up snapshot size. The registry's content-addressed storage handles deduplication automatically: identical layers across snapshots share blobs.

### 11.3 Restore procedure

```
1. SDK: Sandbox.fromSnapshot(snap_id)
2. Controller: load snapshot row, validate ownership and arch
3. Controller: pick drone of matching arch
4. Drone: docker pull <registry>/snap:<id>   # via local registry mirror
5. Drone: docker create with snapshot image as the rootfs;
          re-apply original env, ports, network policy from sandbox spec
6. Drone: docker start
7. agentd boots inside the restored container; reports ready
```

Target: < 800 ms p95 when the base layers are already cached on the drone and only the per-snapshot layer needs to be fetched. The dominant cost is registry pull, which a pull-through cache (one per host) reduces to a local-disk read for repeated restores.

The snapshot image carries the original image's metadata (entrypoint, env, exposed ports), so we don't need to record those separately — `docker commit` preserves them.

### 11.4 Persistent sandboxes

Implementation detail of the `persistent: true` flag from [§6.4](#64-persistent-sandboxes):
- The sandbox row carries `persistent: true`.
- On termination via any path (idle, explicit, lifetime), the drone runs the snapshot procedure and writes the snapshot id back to `sandbox.snapshot_source`.
- The SDK adds a `Sandbox.resume(sandbox_id)` method that, internally, looks up the most recent snapshot for the sandbox id and calls the restore procedure.
- A retention TTL (default 14 days) defines when old snapshots are garbage-collected. Each resume creates a new snapshot row, replacing the prior one as the "current."

---

## 12. Multi-architecture and trust tiers

### 12.1 Architecture matrix

| Architecture | Host options | KVM available? | Nested virt? |
|---|---|---|---|
| x86_64 | EC2 c8i / m8i / r8i with nested virt | Yes | [Yes, since 16 Feb 2026](https://aws.amazon.com/about-aws/whats-new/2026/02/amazon-ec2-nested-virtualization-on-virtual/) — KVM and Hyper-V as L1 hypervisors. Intel Xeon 6 only. |
| x86_64 | EC2 *.metal | Yes (native) | N/A |
| arm64 | EC2 *.metal (Graviton bare metal) | Yes (native) | [Graviton and AMD instances do not yet support nested virt; only 8th-gen Intel families do](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/amazon-ec2-nested-virtualization.html). |
| arm64 | Raspberry Pi 5 (8 GB) | Yes (native, with 64-bit kernel) | N/A |
| arm64 | Other ARM64 SBCs | Varies | — |

Implication: for a fleet that's predominantly x86 EC2 + Pi 5 with optional EC2 Graviton bare metal, the **container path** works everywhere. The **microVM path** (Kata + Firecracker) works on c8i/m8i/r8i with nested virt enabled, on x86 bare metal, on Graviton bare metal, and on Pis natively. It does *not* work on standard (non-nested) Graviton instances.

### 12.2 Trust tiers

| Tier | Default OCI runtime | Where it runs | Use case |
|---|---|---|---|
| `trusted` | `runc` | any drone | First-party code, CI builds, internal tools. |
| `untrusted` | `runsc` (gVisor) | any drone | AI-generated code, user submissions, agent workflows. The default for external callers. |
| `paranoid` | Kata Containers + Firecracker | KVM-capable drones only | Workloads where customers explicitly require microVM-grade isolation. |

The trust tier is selected by the caller in the SDK. The default for new sandboxes is `untrusted`.

### 12.3 Choosing isolation

[gVisor (runsc)](https://gvisor.dev) is a userspace kernel that intercepts the container's syscalls and re-implements them. It does not require virtualization extensions and runs on any Linux host. It has measurable performance overhead (typically 5–20% on CPU-bound workloads, more on syscall-heavy I/O) and some syscall compatibility gaps. It defends against kernel exploits because the container's syscalls never reach the host kernel directly.

[Kata Containers](https://katacontainers.io) runs each container inside a microVM (Firecracker or Cloud Hypervisor). This provides hardware-virtualization isolation — the strongest practical boundary — at the cost of memory overhead per container (typically 50–150 MB for the guest kernel) and a requirement for KVM access.

[runc](https://github.com/opencontainers/runc) is the standard OCI runtime. It provides namespace, cgroup, capability, and seccomp isolation but shares the host kernel. A kernel exploit from inside the container can escape.

For our threat model (untrusted code as a baseline), gVisor is the default. For the `paranoid` tier, Kata. For trusted internal workloads, runc.

### 12.4 Cluster topology

Following Plane's [cluster concept](https://plane.dev/concepts/clusters) — "a pool of drones associated with a single hostname" — we use clusters to bucket drones by capability:

| Cluster | Arch | Trust tiers | Hosts |
|---|---|---|---|
| `iad-x86-default` | amd64 | trusted, untrusted | EC2 c8i with nested virt |
| `iad-x86-paranoid` | amd64 | paranoid | EC2 c8i.metal or c8i with nested virt + Kata |
| `home-arm-default` | arm64 | trusted, untrusted | Raspberry Pi 5s |
| `home-arm-paranoid` | arm64 | paranoid | Pi 5s with Kata + Firecracker |

The scheduler selects a cluster by `(arch, trust_tier)`, then a drone within the cluster by load.

---

## 13. Computer-use desktop image

The desktop image variant of the sandbox runtime supports AI agents that drive a GUI via screenshots and synthetic mouse/keyboard events — the workload pattern described in [Anthropic's Computer Use tool](https://docs.anthropic.com/en/docs/build-with-claude/computer-use) and [OpenAI's equivalent](https://developers.openai.com/api/docs/guides/tools-computer-use).

### 13.1 Image contents

Building on [Anthropic's `computer-use-demo` reference container](https://github.com/anthropics/anthropic-quickstarts/tree/main/computer-use-demo) (published as `ghcr.io/anthropics/anthropic-quickstarts:computer-use-demo-latest`) and the [OpenAI computer-use Dockerfile](https://developers.openai.com/api/docs/guides/tools-computer-use), our image includes:

- Debian or Ubuntu base.
- `Xvfb` — virtual framebuffer X server providing a headless display.
- A lightweight window manager (`openbox` by default; `xfce4` available as a build flag for users who need a full desktop chrome).
- `x11vnc` and `noVNC` for optional live observation.
- `xdotool` for the initial implementation of input synthesis; later replaced by direct XTest calls from `agentd` for latency.
- A pre-installed Chromium build (the distribution package, not Google Chrome, to keep multi-arch builds working).
- Fonts: `fonts-noto`, `fonts-noto-cjk`, `fonts-liberation`, `fonts-noto-color-emoji`.
- `agentd` (desktop variant) on port 8080, exposing both the headless API and the `/v1/computer/*` extensions.

### 13.2 Process tree

```
PID 1: tini
  ├─ Xvfb :99 -screen 0 1280x800x24
  ├─ openbox
  ├─ x11vnc -display :99 -shared -forever        (optional)
  ├─ websockify --web /usr/share/novnc 6080 ...  (optional)
  ├─ agentd                                       (HTTP :8080)
  └─ user-launched apps (chromium, ...)
```

### 13.3 Display contract

- Pinned resolution: 1280×800 by default. Vision models trained for computer use perform best on a small set of resolutions; varying it degrades accuracy.
- 24-bit color.
- DPI fixed at 96. Window manager configured for no scaling.
- Screenshots are PNGs of the raw framebuffer. `agentd` uses `XGetImage` directly rather than shelling out to `import`, to avoid 30+ ms per screenshot.

### 13.4 Action latency budgets

| Action | Target |
|---|---|
| Screenshot (1280×800 PNG) | < 80 ms |
| Click | < 30 ms |
| Type (per character) | < 15 ms |
| Key | < 30 ms |
| Scroll | < 30 ms |

These are tight enough that screenshot loop iterations dominated by inference time, not synthesis.

### 13.5 Optional VNC viewer

For human observation (debugging, audit, live monitoring), the desktop image exposes noVNC on port 6080 in addition to `agentd` on 8080. Both are reachable via `sandbox.domain(port)`. The viewer is read-only by default (`x11vnc -nopw -viewonly`); a write-enabled mode for human takeover is selectable at create time.

### 13.6 Resource floor

A desktop sandbox has a higher floor than a headless one:

| Component | Memory floor |
|---|---|
| Xvfb + openbox | ~80 MB |
| `agentd` | ~30 MB |
| One Chromium tab | ~400 MB |
| **Total typical floor** | **~600 MB** |

Plan drone capacity accordingly. A Pi 5 with 8 GB RAM can comfortably host 4–6 desktop sandboxes; a c8i.4xlarge (32 GB) can host 30–40.

---

## 14. Observability and metering

### 14.1 Metering

For every running sandbox, the drone samples cgroup v2 statistics every 5 seconds. The exact path depends on Docker's cgroup driver:

| Cgroup driver | Path template |
|---|---|
| `systemd` (default on cgroup v2) | `/sys/fs/cgroup/system.slice/docker-<container-id>.scope/` |
| `cgroupfs` | `/sys/fs/cgroup/docker/<container-id>/` |

Per [Docker's runtime metrics docs](https://docs.docker.com/engine/containers/runmetrics/). Within that directory, the drone reads:

```
cpu.stat       → "usage_usec <N>" line (microseconds, monotonic)
memory.current → bytes (current resident usage)
io.stat        → "rbytes <N> wbytes <N> rios <N> wios <N>" per device
```

Per the [Linux kernel cgroup v2 docs](https://docs.kernel.org/admin-guide/cgroup-v2.html). All time durations are microseconds; `cpu.stat` always reports `usage_usec`, `user_usec`, `system_usec`, and additionally `nr_periods`, `nr_throttled`, `throttled_usec` when the cpu controller is enabled.

Deltas are shipped to the controller as Plane's native `BackendMetrics` WebSocket message (verified shape in [interfaces §5.2](./sandbox-platform-interfaces.md#52-messagefromdrone-drone--controller--plane)). The controller writes derived rollups into our `usage_record` table.

**Unit reconciliation.** Three different time units sit in this pipeline; the controller normalizes:

| Boundary | Unit | Why |
|---|---|---|
| `/sys/fs/cgroup/.../cpu.stat` | microseconds (`usage_usec`) | Linux kernel default for cgroup v2. |
| `BackendMetricsMessage.cpu_used` (Plane wire) | nanoseconds | Plane's documented unit (`common/src/protocol.rs`, `BackendMetricsMessage`). |
| `usage_record.cpu_nsec` (our table) | nanoseconds | We match Plane's unit to avoid one more conversion. |

The drone reads cgroup `usage_usec`, multiplies by 1000 to get nanoseconds, sends as `BackendMetrics`. The controller stores as-is. Anyone hand-rolling a metering reader on the drone must do this conversion to stay compatible with Plane upstream.

Billing reads from `usage_record`:
- CPU billed by `cpu_nsec` summed per hour bucket.
- Memory billed by `memory_mb_seconds`.

This matches Vercel's [Active CPU model](https://vercel.com/sandbox): we charge for time the CPU is actually executing, idle time excludes itself because `cpu_used` only advances when the container is running.

### 14.2 Logs

- **Container stdout/stderr** — Tailed by the drone via the Docker logs API, forwarded to the controller, and made available to the SDK as a streaming endpoint. Retention: 7 days for `untrusted` and `trusted` tiers, configurable per organization.
- **Lifecycle events** — From Plane's status stream, emitted as SSE on `/pub/v1/sandboxes/:id/status`.
- **Egress proxy decisions** — Each blocked or credential-injected request is logged as a structured event. Critical for audit; mandatory for the `paranoid` tier.
- **`agentd` access logs** — Each exec/file op, with request id, sandbox id, and duration.

### 14.3 Metrics

Prometheus endpoints on each component:
- Controller: request rate, error rate, latency per endpoint; sandbox count by state, tier, arch.
- Drone: container count, cgroup stats, image pull rate, egress proxy throughput.
- Proxy: route resolution hits/misses, active route cache size, ACME renewal status, `KeepAlive` emission rate.

Per-tenant rollups are computed nightly from `usage_record` for the dashboard.

---

## 15. Security model and threat analysis

### 15.1 Assets

In decreasing order of value:
1. **Credentials and customer secrets** stored in the platform.
2. **Other tenants' sandbox state** (filesystem contents, code).
3. **Host integrity** (the drone, its kernel, its other workloads).
4. **Egress credibility** (using a platform IP to attack third parties).
5. **Capacity** (denial-of-service against the platform itself).

### 15.2 Attacker capabilities (worst case)

- Arbitrary code execution inside a sandbox container, as root inside that container.
- Arbitrary network egress, modulo our policy.
- Knowledge of all the standard Linux kernel CVEs.
- Time and patience.

### 15.3 Mitigations

| Asset | Mitigation |
|---|---|
| Other tenants' state | gVisor/Kata isolation; per-sandbox network namespaces; per-sandbox overlay filesystems; no shared volumes. |
| Host integrity | gVisor/Kata isolation; drone runs minimal services; no kubelet-style "exec into any pod" surface; seccomp + AppArmor profiles on the OCI runtime. |
| Credentials | Never enter the sandbox; injected by egress proxy; per-sandbox internal CA. |
| Egress credibility | Default-deny network policy for untrusted-tier sandboxes; abuse-rate-limiting per host IP; outbound from a known set of IPs we can have de-listed quickly. |
| Capacity | Per-tenant sandbox limits; per-sandbox cgroup limits (CPU, memory, PIDs); active-CPU budget; lifetime caps. |

### 15.4 Known residual risks

- **Untrusted code with `runc`.** The `trusted` tier explicitly accepts host-kernel exposure. Callers using `trusted` for adversarial code are out of scope.
- **gVisor compatibility.** Some workloads (e.g., low-level eBPF, certain JVM GC paths) hit syscall coverage gaps. We document the supported syscall surface and recommend `paranoid` tier (Kata) for affected workloads.
- **Side channels.** Spectre/Meltdown-class attacks across containers on the same host are not fully mitigated even with gVisor. For workloads where this matters, use `paranoid`.
- **Prompt injection in computer-use workloads.** A webpage can contain text designed to redirect an AI agent driving the sandbox. [Anthropic ships classifiers that flag potential prompt injections in screenshots and steer the model to ask for user confirmation before proceeding](https://docs.anthropic.com/en/docs/build-with-claude/computer-use); we do not duplicate that work on our side, but we recommend callers run their agents with appropriate guardrails.

---

## 16. Operational concerns

### 16.1 Drone provisioning

EC2 drones provision via a custom AMI built with Packer:
- Base: Amazon Linux 2023 or Ubuntu 24.04 LTS.
- Pre-installed: Docker (or containerd directly), `runsc`, optionally `kata-runtime`, our drone binary, the egress proxy binary.
- On first boot: read drone configuration from EC2 user data (controller URL, cluster name, credentials), register with the controller.

For c8i/m8i/r8i instances, enable nested virt at launch with the verified CLI flag `--cpu-options "NestedVirtualization=enabled"` (per [AWS EC2 docs](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/amazon-ec2-nested-virtualization.html)). To toggle on an existing instance, the instance must be stopped first.

Pi drones: image written to SD or NVMe, joined to the network, drone service started via systemd. A small bootstrap utility on the Pi handles initial registration with the controller using a pre-shared token.

### 16.2 Image distribution

A private OCI registry (Distribution or an S3-backed equivalent) serves the runtime images. Each drone runs a pull-through cache to avoid hitting the registry on every cold start.

Multi-arch images are tagged with the same name across architectures; the registry's manifest list resolves to the right blob per drone's `arch`.

### 16.3 Updates

- **`agentd`** ships as part of the image. Updates require rebuilding the runtime images and re-tagging. Existing sandboxes keep their version until restarted.
- **Drone binary.** Rolling restart, drained one host at a time. Plane's [design principle of "dumb executors"](https://plane.dev/developing/design-principles) means drone updates do not require coordinated controller updates.
- **Controller.** Standard zero-downtime deploy: multiple replicas behind the reverse proxy, rolling restart. Plane controllers communicate via Postgres LISTEN/NOTIFY, so there is no leader election to manage.

### 16.4 Backup and DR

- Postgres: standard managed-service backups, point-in-time-recovery, cross-AZ replica.
- Snapshots: object store with versioning and lifecycle policies. Replicated cross-region for paranoid customers (optional, paid).
- Drone fleet: stateless. Lost drones cause their sandboxes to be marked `lost` and the SDK to receive a terminal error. Persistent sandboxes can be recreated from their last snapshot.

### 16.5 Quotas

Per-tenant defaults (modelled on [Vercel Sandbox's pricing page](https://vercel.com/docs/vercel-sandbox/pricing) and [SDK reference](https://vercel.com/docs/vercel-sandbox/sdk-reference) as of May 2026):

- Max concurrent sandboxes: 50 (configurable). Vercel allows up to 2,000 on Pro/Enterprise per [Northflank's comparison](https://northflank.com/blog/modal-vs-vercel-sandbox).
- Max sandbox lifetime: 5 hours on Pro/Enterprise, 45 minutes on Hobby (verified from the [`@vercel/sandbox` README](https://github.com/vercel/sandbox)). Default 5 minutes. Extensible at runtime via `extendTimeout`.
- Max vCPUs per sandbox: 8 on Hobby/Pro, 32 on Enterprise (verified from the same source).
- Memory: 2 GB (2048 MB) per vCPU. Tied to vCPU count, not independently configurable.
- Max open ports per sandbox: 4 (per the [Vercel Sandbox product page](https://vercel.com/sandbox)).
- Max snapshots: 1000 per tenant, max snapshot size 5 GB. Default 30-day retention, configurable up to never-expires per [Vercel docs](https://vercel.com/kb/guide/vercel-sandbox-vs-codesandbox).
- Persistent sandboxes: in beta upstream; we ship our equivalent as v1 GA.

---

## 17. Build phases and milestones

### Phase 1 — Walking skeleton (2–3 weeks)

**Goal:** End-to-end create/exec/stop on one EC2 host and one Pi.

- Deploy stock Plane (controller + drone + proxy + Postgres) via Docker Compose.
- Build `agentd` v0 with `/health`, `/v1/exec`, `/v1/fs/read`, `/v1/fs/write`.
- Build base + node24 runtime images (multi-arch).
- SDK skeleton: `Sandbox.create`, `runCommand`, `writeFiles`, `stop`.
- Single trust tier (`trusted`, `runc`).
- Demo: SDK creates a sandbox, runs `node --version`, writes a file, stops.

### Phase 2 — Hardening (2–3 weeks)

**Goal:** Untrusted code can run safely.

- Add `runsc` runtime to drone hosts.
- Add `trust: 'untrusted'` SDK flag; controller routes to runsc-capable drones.
- Build egress proxy with `allow-all`/`deny-all`/simple-rules modes.
- Wire iptables in container netns to force outbound through proxy.
- Add `sandbox.updateNetworkPolicy()` SDK method.
- Cgroup-based CPU and memory metering; usage_record table populating.

### Phase 3 — The killer feature: snapshots (3–4 weeks)

**Goal:** Restore from snapshot under 1 s; `persistent: true` works.

- Snapshot procedure on the drone: pause, export overlay, upload, resume.
- Restore procedure: pull base, download overlay, mount, start.
- Object store integration (S3 or self-hosted MinIO).
- Persistent sandboxes: auto-snapshot on stop, resume API.
- Snapshot retention and GC.

### Phase 4 — Credential brokering (1–2 weeks)

**Goal:** Outbound requests can use secrets the sandbox never sees.

- Secrets table, KMS integration, controller-side encryption.
- Egress proxy MITM with per-sandbox internal CA.
- Header injection on configured destinations.
- `sandbox.bindCredential()` SDK method.
- Audit log for every injection.

### Phase 5 — Computer-use desktop image (2–3 weeks)

**Goal:** Anthropic's computer-use tool works end-to-end against our sandbox.

- Desktop runtime image with Xvfb + openbox + Chromium + agentd.
- `agentd` `/v1/computer/*` endpoints.
- Optional noVNC observer.
- Compatibility test suite running the Anthropic and OpenAI computer-use SDKs against our backend.

### Phase 6 — Polish and scale (ongoing)

- Multi-runtime images: python3.13, bun1, custom.
- Tags + filtering.
- Observability dashboard.
- Billing pipeline.
- `paranoid` tier (Kata + Firecracker) where KVM is available.

---

## 18. Open questions

1. **Region strategy.** First region: us-east-1 (iad). Open question: when do we add a second, and which? GCP us-central1 is a strong candidate for redundancy without the AWS-only blast radius.
2. **Bare metal for `paranoid`.** Is the customer demand for memory-snapshot-grade isolation worth maintaining a separate fleet of c8i.metal hosts, or can nested-virt c8i with Kata cover this use case acceptably?
3. **GPU.** Not in v1 scope, but the schema should accommodate it. Adding a `resources.gpu` field at create time and a `gpu-capable` drone capability flag is cheap insurance.
4. **Browser fingerprinting.** Desktop sandboxes will hit anti-bot defenses on many sites. Do we ship a "stealth Chromium" build, or document that this is out of scope and recommend customers use a third-party browser API (e.g., Browserbase) for those workloads?
5. **In-memory snapshots.** When (not if) we add Firecracker-backed memory snapshots, does the SDK distinguish them from filesystem snapshots, or is `snapshot()` a single primitive whose fidelity depends on tier? The latter is simpler but obscures cost differences.
6. **Multi-tenancy on Pi drones.** A Pi 5 is small enough that running multiple tenants' sandboxes on one feels uncomfortable from a noisy-neighbor standpoint. Do we restrict Pis to single-tenant, or accept the noise and let scheduling handle it?

---

## 19. References

### Primary sources

- Vercel Sandbox product page: <https://vercel.com/sandbox>
- Vercel Sandbox docs: <https://vercel.com/docs/vercel-sandbox>
- Vercel Sandbox SDK reference: <https://vercel.com/docs/vercel-sandbox/sdk-reference>
- Vercel Sandbox pricing and limits: <https://vercel.com/docs/vercel-sandbox/pricing>
- Vercel Sandbox GA blog: <https://vercel.com/blog/vercel-sandbox-is-now-generally-available>
- Vercel Sandbox vs CodeSandbox (Vercel KB): <https://vercel.com/kb/guide/vercel-sandbox-vs-codesandbox>
- Vercel Sandbox vs E2B (Vercel KB): <https://vercel.com/kb/guide/vercel-sandbox-vs-e2b>
- `@vercel/sandbox` source and README: <https://github.com/vercel/sandbox>
- `@vercel/sandbox` on npm: <https://www.npmjs.com/package/@vercel/sandbox>
- Plane documentation: <https://plane.dev>
- Plane architecture: <https://plane.dev/concepts/architecture>
- Plane backend lifecycle: <https://plane.dev/concepts/backend-lifecycle>
- Plane keys and fencing tokens: <https://plane.dev/developing/keys>
- Plane connection authorization: <https://plane.dev/concepts/auth>
- Plane vs. Jamsocket (scope): <https://plane.dev/plane-vs-jamsocket>
- Plane source: <https://github.com/jamsocket/plane>

### Isolation primitives

- Firecracker: <https://github.com/firecracker-microvm/firecracker>
- gVisor: <https://gvisor.dev>
- Kata Containers: <https://katacontainers.io>
- Cloud Hypervisor: <https://www.cloudhypervisor.org>
- AWS EC2 nested virtualization announcement (16 Feb 2026): <https://aws.amazon.com/about-aws/whats-new/2026/02/amazon-ec2-nested-virtualization-on-virtual/>
- AWS EC2 nested virtualization docs: <https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/amazon-ec2-nested-virtualization.html>
- InfoQ coverage of AWS nested virt: <https://www.infoq.com/news/2026/03/aws-ec2-nested-virtualization/>

### Computer-use specifications

- Anthropic Computer Use tool docs: <https://docs.anthropic.com/en/docs/build-with-claude/computer-use>
- OpenAI Computer Use guide: <https://developers.openai.com/api/docs/guides/tools-computer-use>
- Anthropic `computer-use-demo` reference container: <https://github.com/anthropics/anthropic-quickstarts/tree/main/computer-use-demo>
- Anthropic prompt injection defenses (Nov 2025): <https://www.anthropic.com/news/prompt-injection-defenses>
- cua (open-source computer-use infrastructure): <https://github.com/trycua/cua>
- agent-infra/sandbox (all-in-one container reference): <https://github.com/agent-infra/sandbox>
- ScreenEnv (HuggingFace desktop sandbox): <https://huggingface.co/blog/screenenv>
- Warp on computer use in cloud sandboxes: <https://www.warp.dev/blog/computer-use-cloud-agents>

### Comparable products

- E2B: <https://e2b.dev>
- Modal Sandbox: <https://modal.com/docs/guide/sandbox>
- Daytona: <https://daytona.io>
- CodeSandbox SDK: <https://codesandbox.io/docs/sdk>
- Cloudflare Sandboxes: <https://blog.cloudflare.com/sandboxes-on-cloudflare/>
- Northflank on Kubernetes-based sandboxes: <https://northflank.com/blog/sandboxes-on-kubernetes>
- Northflank AI sandbox pricing comparison (2026): <https://northflank.com/blog/ai-sandbox-pricing>

### Standards and protocols

- OCI Runtime Spec: <https://github.com/opencontainers/runtime-spec>
- cgroup v2: <https://docs.kernel.org/admin-guide/cgroup-v2.html>
- ACME (RFC 8555): <https://datatracker.ietf.org/doc/html/rfc8555>


