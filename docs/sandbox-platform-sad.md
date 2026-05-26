# Sandbox Platform — Software Architecture Document

**Status:** v0.1
**Audience:** Engineers building, operating, or reviewing the platform
**Companion:** [`sandbox-platform-spec.md`](./sandbox-platform-spec.md) (defines what we build; this document defines how it's structured)

---

## How to read this document

This SAD follows the [C4 model](https://c4model.com): we start at the system as a single box (Level 1, system context), break it into deployable units (Level 2, containers), then into the internal components of each (Level 3). After that we cover dynamic behavior (runtime views), where each component runs (deployment view), and the concerns that cut across everything (cross-cutting).

Architectural decisions appear inline where the topic comes up, and again in §10 as numbered ADRs you can cite from PRs.

The spec is the source of truth for contracts and limits; this document is the source of truth for structure and rationale.

---

## Table of contents

1. [Quality attributes](#1-quality-attributes)
2. [Level 1 — System context](#2-level-1--system-context)
3. [Level 2 — Containers](#3-level-2--containers)
4. [Level 3 — Components](#4-level-3--components)
5. [Runtime views](#5-runtime-views)
6. [Deployment view](#6-deployment-view)
7. [Data architecture](#7-data-architecture)
8. [Cross-cutting concerns](#8-cross-cutting-concerns)
9. [Risks and known limitations](#9-risks-and-known-limitations)
10. [Architectural decisions](#10-architectural-decisions)
11. [Glossary](#11-glossary)

---

## 1. Quality attributes

The architecture optimizes for, in priority order:

1. **Isolation strength.** Untrusted code must not escape its sandbox. Defense in depth: kernel-level boundary by default (gVisor), microVM available for paranoid tier. A working LPE inside the sandbox should not be sufficient to compromise the host or other tenants.
2. **Cold-start latency.** Sub-second from `Sandbox.create()` to a ready container is the product. Anything that adds non-deterministic latency to the create path is suspect.
3. **Operational simplicity.** A small team must be able to run this. We accept a few percent of theoretical efficiency to avoid building a custom orchestrator or hypervisor.
4. **Multi-tenant fairness.** Per-tenant quotas; no noisy-neighbor surprises within a host.
5. **Cost.** Active-CPU billing depends on accurate metering; per-host packing density depends on isolation overhead.
6. **Portability across the fleet.** Same control plane runs against EC2 hosts and Raspberry Pi drones. Same images run on amd64 and arm64.

Trade-offs we accept:
- We are not the fastest sandbox in any single dimension. We're competitive enough on each.
- We tolerate ~5–20% gVisor overhead for the default tier in exchange for never requiring nested virtualization.
- Filesystem-only snapshots in v1; memory-state snapshots are a v2 concern requiring Kata + Firecracker.

---

## 2. Level 1 — System context

At the 30,000-foot view the platform is one box with five categories of external actors.

```
                                    External world
                                          │
   ┌──────────────────┐   ┌───────────────┴───────────────┐  ┌─────────────────┐
   │ Caller           │   │ End user / agent harness      │  │ Operator        │
   │ (your backend,   │   │ (browser, AI agent loop,      │  │ (you, on-call)  │
   │  CI system,      │   │  chat client; consumes        │  │                 │
   │  AI orchestrator)│   │  backend URLs)                │  │                 │
   └────────┬─────────┘   └────────────────┬──────────────┘  └────────┬────────┘
            │ SDK calls                    │ HTTPS to                │ Ops APIs,
            │ (create, exec, …)            │ sandbox URL             │ dashboards
            │                              │                         │
            ▼                              ▼                         ▼
   ┌────────────────────────────────────────────────────────────────────────────┐
   │                                                                            │
   │                          SANDBOX PLATFORM                                  │
   │                                                                            │
   │   Receives requests to provision ephemeral compute, runs untrusted code    │
   │   inside isolated containers across a fleet of hosts, mediates outbound    │
   │   traffic, and meters usage.                                               │
   │                                                                            │
   └─────┬──────────────────────┬─────────────────────┬─────────────────────────┘
         │                      │                     │
         ▼                      ▼                     ▼
   ┌──────────────┐    ┌────────────────┐    ┌──────────────────┐
   │ OCI Registry │    │ Third-party    │    │ KMS / Secret     │
   │ (Docker Hub, │    │ APIs           │    │ store            │
   │  ECR, our    │    │ (LLM APIs,     │    │ (AWS KMS, Vault, │
   │  pull-       │    │  package       │    │  HashiCorp,      │
   │  through)    │    │  registries,   │    │  cloud-native)   │
   └──────────────┘    │  customer      │    └──────────────────┘
                       │  endpoints)    │
                       └────────────────┘
```

### 2.1 Actors

- **Caller.** Programmatic users of the SDK. Typically server-side: a developer's backend, a CI runner, an AI agent harness, an automation tool. Authenticates with our control plane.
- **End user / agent harness.** Consumes the URLs the platform mints (e.g., `https://sb-abc-3000.sandboxes.example.com/`). May be a browser, an AI agent loop screenshotting a desktop, or anything else. Authenticates via the connection token in the URL, not directly with the control plane.
- **Operator.** Us. Has access to admin APIs, observability, and the underlying infrastructure.

### 2.2 External systems

- **OCI Registry.** Source of runtime images. We run a pull-through cache per host so the wide internet isn't on the hot path.
- **Third-party APIs.** Whatever the sandbox calls outward. Some destinations require credential injection; the egress proxy handles that mediation.
- **KMS / secret store.** Used by the controller to encrypt credentials at rest and decrypt only when binding to a running sandbox.

### 2.3 Trust boundaries

Three boundaries with materially different threat models:

1. **Caller → control plane.** Authenticated, trusted. Bug here causes wrong sandboxes to be created. Not catastrophic.
2. **End user / sandbox content → platform internals.** Untrusted. Code inside the sandbox is the attacker. Bug here causes data exfiltration, lateral movement, or host compromise. This is the boundary the architecture is built to defend.
3. **Platform → external destinations.** Mediated by the egress proxy. Bug causes credential leakage or policy bypass.

---

## 3. Level 2 — Containers

The platform decomposes into eight deployable units. Six of them are long-lived; two are ephemeral. We use *container* in the C4 sense (unit of deployment) and reserve *OCI container* for the Docker sense.

```
                ┌─────────────────────── Control plane ────────────────────────┐
                │                                                              │
                │   ┌──────────────┐         ┌──────────────┐                  │
   SDK ─────────┼──▶│  Controller  │◀───────▶│   Postgres   │                  │
                │   │  (Rust,Axum) │         │              │                  │
                │   └─────┬────────┘         └──────────────┘                  │
                │         │ WebSocket                                          │
                │         │                                                    │
                └─────────┼────────────────────────────────────────────────────┘
                          │
            ┌─────────────┴─────────────┐
            │                           │
            ▼                           ▼
    ┌──────────────┐            ┌──────────────┐
    │    Proxy     │            │    Drone     │  ◀── one per host
    │  (Rust)      │            │   (Rust)     │
    │  TLS, route  │            │              │
    └──────┬───────┘            └───┬──────┬───┘
           │                        │      │
   end-user│ HTTPS                  │      │ supervises
   traffic │                        │      │
           │                        │      ▼
           ▼                        │   ┌──────────────────┐
    ┌──────────────────┐            │   │  Egress proxy    │
    │   Sandbox        │  data plane│   │  (Envoy)         │
    │   container      │◀───────────┤   │  TLS-MITM where  │  ◀── one per host
    │   ┌──────────┐   │            │   │  configured      │
    │   │  agentd  │   │            │   └────────┬─────────┘
    │   │  :8080   │   │            │            │
    │   └──────────┘   │  spawn,    │            │ outbound
    │   user workload  │  monitor   │            │ to internet
    │                  │◀───────────┘            ▼
    └──────────────────┘                  ┌──────────────────┐
                                          │  External APIs   │
                                          └──────────────────┘
                                                  ▲
                                                  │
                                          ┌───────┴──────┐
                                          │ OCI Registry │  ◀── pull-through
                                          │  (per host)  │      cache, also
                                          │              │      stores snaps
                                          └──────────────┘
```

### 3.1 Container catalog

| # | Container | Cardinality | Lifetime | Stateful? | Implementation |
|---|---|---|---|---|---|
| 1 | **Controller** | N replicas (HA) | persistent | no | Plane controller (Rust), extended |
| 2 | **Proxy** | N per cluster | persistent | small caches | Plane proxy (Rust), extended |
| 3 | **Drone** | one per host | persistent | local-only | Plane drone (Rust), extended |
| 4 | **Egress proxy** | one per host | persistent | policy cache | Envoy (sidecar) |
| 5 | **Sandbox container** | one per sandbox | ephemeral | per-tenant | OCI runtime: runc / runsc / Kata |
| 6 | **agentd** | one per sandbox | ephemeral | none | Go/Rust binary inside sandbox |
| 7 | **Postgres** | 1 primary + replica | persistent | yes — source of truth | managed Postgres |
| 8 | **OCI Registry** | one per host (cache) + central | persistent | image blobs | Distribution; S3-backed |

### 3.2 Inter-container communication

Three protocols, used consistently:

- **HTTP/REST** between the SDK and the controller's public API (`/pub/*`), and between the controller and external systems (KMS, registry).
- **WebSocket** for long-lived control connections: drone↔controller, proxy↔controller, controller↔Postgres (LISTEN/NOTIFY internally).
- **TCP/HTTP** for data-plane traffic: end-user → proxy → sandbox-container:port, and sandbox → egress proxy → internet.

The controller has a dual API per Plane's design: `/pub/*` for SDK calls (sits behind a reverse proxy that handles auth), `/ctrl/*` for component-to-component coordination (firewalled, internal-only).

### 3.3 Why this decomposition

- **Controller stateless across replicas.** Lets us run as many as we want behind a load balancer. All state in Postgres; coordination via LISTEN/NOTIFY (inherited from Plane).
- **Drone as the only thing that talks to the OCI runtime.** Concentrates the Docker socket privilege in one binary per host. Compromising the controller doesn't directly let you spawn containers; it lets you ask the drone to.
- **Egress proxy separate from the drone.** Different lifecycle (policy updates are frequent; drone updates are rare). Different failure mode (egress policy bug shouldn't crash drone). Different language even (Envoy vs Rust).
- **agentd as a stable in-sandbox contract.** Every runtime image embeds it. The data plane talks to one shape regardless of what's running inside.

---

## 4. Level 3 — Components

We now open each container and describe its internal components. For each: responsibility, key collaborators, interfaces, and operational notes.

### 4.1 Controller

The controller is the orchestrator. Below: components inside the controller process. Most map to Rust modules.

```
┌────────────────────────────── Controller process ──────────────────────────────┐
│                                                                                │
│   ┌─────────────────┐       ┌──────────────────┐      ┌─────────────────┐     │
│   │ HTTP server     │       │ WebSocket hub    │      │ Auth shim       │     │
│   │ (Axum, async)   │       │ (drone, proxy)   │      │ (forward-auth)  │     │
│   └────────┬────────┘       └─────────┬────────┘      └────────┬────────┘     │
│            │                          │                        │              │
│            ▼                          ▼                        │              │
│   ┌─────────────────────────────────────────────────┐          │              │
│   │              Request dispatcher                 │◀─────────┘              │
│   │   /pub/* → SDK handlers                         │                         │
│   │   /ctrl/* → component & ops handlers            │                         │
│   └─────┬──────────────────────────────────────┬────┘                         │
│         │                                      │                              │
│         ▼                                      ▼                              │
│   ┌─────────────────────────┐         ┌───────────────────┐                   │
│   │  Lifecycle manager      │         │  Cluster state    │                   │
│   │  (per-sandbox state     │         │  (drone roster,   │                   │
│   │   machine, key locks    │         │   proxy roster,   │                   │
│   │   with fencing tokens)  │         │   capacity)       │                   │
│   └─────┬───────────────────┘         └─────────┬─────────┘                   │
│         │                                       │                             │
│         ├──────────┐               ┌────────────┘                             │
│         ▼          ▼               ▼                                          │
│   ┌─────────┐  ┌──────────┐  ┌──────────────┐                                 │
│   │Scheduler│  │ Snapshot │  │ Policy /     │                                 │
│   │         │  │ registry │  │ secret store │                                 │
│   └────┬────┘  └────┬─────┘  └──────┬───────┘                                 │
│        │            │               │                                         │
│        │     ┌──────┴───────────────┴──────┐                                  │
│        │     │     Metering aggregator     │◀── drone reports                 │
│        │     └─────────────────────────────┘                                  │
│        │                                                                      │
│        ▼                                                                      │
│   ┌─────────────────────────────────────────────────┐                         │
│   │             Postgres adapter (sqlx)             │                         │
│   │   tables: sandbox, snapshot, network_policy,    │                         │
│   │           credential_binding, secret,           │                         │
│   │           usage_record, plus Plane tables       │                         │
│   └─────────────────────────────────────────────────┘                         │
│                                                                                │
└────────────────────────────────────────────────────────────────────────────────┘
```

#### 4.1.1 HTTP server
Axum-based; serves `/pub/*` for SDK and `/ctrl/*` for trusted callers. Stateless. Multiple replicas share a Postgres-backed view. The split mirrors [Plane's design](https://plane.dev/concepts/architecture).

#### 4.1.2 WebSocket hub
Long-lived bidirectional channels to drones and proxies. The drone-socket channel is at `/ctrl/c/:cluster/drone-socket`; the proxy-socket at `/ctrl/c/:cluster/proxy-socket`.

The wire protocol on the drone channel is Plane's `MessageToDrone` / `MessageFromDrone` pair (verified in `common/src/protocol.rs`):

- `MessageToDrone::Action` carries a `BackendAction` (`Spawn` or `Terminate` — Plane's only two actions). We add extension variants for `PushNetworkPolicy`, `PushCredential`, `RevokeCredential`, and `Snapshot`.
- `MessageFromDrone` carries `Heartbeat`, `BackendEvent` (state transitions), `BackendMetrics`, `AckAction`, and `RenewKey`. We add an `AckExtensionAction` variant for our extension actions.

Actions are delivered with at-least-once semantics via Plane's durable `backend_action` table (a row per dispatched action; the drone acks by `action_id`; unacked rows are redelivered on reconnect). Our extension actions follow the same pattern using a parallel `extension_action` table.

Full message-by-message protocol in [interfaces §5](./sandbox-platform-interfaces.md#5-drone--controller-websocket-plane--our-additions).

The proxy channel uses Plane's `MessageFromProxy` / `MessageToProxy` pair, which is a **request/response** pattern (proxy asks `RouteInfoRequest { token }`, controller answers `RouteInfoResponse`), not a push of route tables. See [interfaces §6](./sandbox-platform-interfaces.md#6-proxy--controller-websocket-plane--our-additions).

#### 4.1.3 Lifecycle manager
The state machine for every sandbox. Built on Plane's `BackendStatus` enum (`scheduled → loading → starting → waiting → ready → terminating → hard-terminating → terminated`) with semantic layers on top:

- **Key acquisition with fencing tokens.** Before scheduling, the manager acquires a lock on the sandbox's key in Postgres. The lock is monotonic-versioned: a stale drone trying to claim a key with an older fencing token fails. Lifted from Plane's design.
- **Pool routing.** A spawn request lands on a drone tagged with the matching `pool`. The trust-tier-to-pool mapping is a fixed contract: `trusted→trusted`, `untrusted→untrusted`, `paranoid→paranoid` (the strings are equal by design). Authoritative table in [interfaces §3.2](./sandbox-platform-interfaces.md#32-post-pubv1sandboxes).
- **Auto-termination triggers.** Idle timeout (driven by `KeepAlive` messages from proxies updating `backend.last_keepalive`), lifetime cap, active-CPU budget.

#### 4.1.4 Scheduler
Picks a drone for a new sandbox. Inputs: requested arch, trust tier (→ pool), resource request, and current drone capacity. Algorithm: filter by capability, sort by load (cgroup-derived), pick the least-loaded with capacity. Falls back to "no drone available" if nothing fits; the SDK returns a recoverable error.

The scheduler is per-request, not a background loop. Decisions are durable (written to Postgres) before being dispatched to the drone.

#### 4.1.5 Snapshot registry
Tracks `snapshot` rows. Each row is metadata; the actual snapshot is an OCI image in the local registry. Operations: register (after a drone reports a successful `docker commit + push`), resolve (look up by id; return image ref + metadata), expire (GC old snapshots per retention policy).

#### 4.1.6 Policy / secret store
- **Network policy.** JSON documents per sandbox; pushed to drones on change. Validation happens here.
- **Secret store.** Ciphertext in Postgres, encrypted with KMS-derived data keys. Decrypt-on-bind: when a sandbox is scheduled to a drone, the controller fetches the data key, decrypts the secret, and ships it to the drone over the WebSocket. Drone holds it in memory only.
- **Credential binding.** Tuple of `{sandbox, destination, header_name, secret}`. Pushed to the drone's egress proxy.

#### 4.1.7 Metering aggregator
Receives 30-second batches of cgroup deltas from each drone. Bucketizes by hour into the `usage_record` table. Read by the billing pipeline downstream (not part of this system).

#### 4.1.8 Auth shim (forward-auth)
Plane supports forwarding `/ctrl/*` requests to an external auth service via `--forward-auth`. We use this to enforce admin auth on the control API without baking auth into the controller. `/pub/*` auth is handled by the front-door reverse proxy.

### 4.2 Drone

The drone is the platform's foothold on a host. It's the only component that talks to the OCI runtime.

```
┌──────────────────────────── Drone process (host) ─────────────────────────────┐
│                                                                               │
│   ┌────────────────────┐         ┌─────────────────────┐                      │
│   │ Controller WS      │◀──────▶│ Command dispatcher  │                       │
│   │ client (recv cmd,  │  events │ (handle spawn,      │                      │
│   │ send events)       │   ▲     │ terminate, snapshot,│                      │
│   └────────────────────┘   │     │ push-policy, etc.)  │                      │
│                            │     └─────────┬───────────┘                      │
│                            │               │                                  │
│           ┌────────────────┼───────────────┼─────────────┐                    │
│           │                │               │             │                    │
│           ▼                │               ▼             ▼                    │
│   ┌────────────────┐       │     ┌────────────────┐  ┌──────────────────┐    │
│   │ OCI runtime    │       │     │ Snapshot       │  │ Policy applier   │    │
│   │ adapter        │       │     │ worker         │  │ (config to       │    │
│   │ (Docker socket)│       │     │ (docker commit │  │  egress proxy)   │    │
│   │                │       │     │  + push)       │  │                  │    │
│   └────┬───────────┘       │     └────────────────┘  └──────────────────┘    │
│        │                   │                                                  │
│        │     ┌─────────────┴──────┐                                           │
│        │     │ Metering reader    │                                           │
│        │     │ (cgroup scraper,   │                                           │
│        │     │  5-sec sample,     │                                           │
│        │     │  30-sec batch)     │                                           │
│        │     └────────────────────┘                                           │
│        │                                                                      │
│        ▼                                                                      │
│   ┌────────────────────────────────────────────────────────────┐              │
│   │                  Docker / containerd socket                │              │
│   │       (mounted from /var/run/docker.sock as root)          │              │
│   └────────────────────────────────────────────────────────────┘              │
│                                                                               │
└───────────────────────────────────────────────────────────────────────────────┘
```

#### 4.2.1 Controller WebSocket client
Single persistent connection to the controller's `/ctrl/c/:cluster/drone-socket`. **There is no application-level handshake or "hello" message**: identity is established by the WebSocket upgrade path itself (the controller looks up the drone by IP + cluster from the upgrade context). Capabilities (arch, supported runtimes, pool) are reported separately via the drone's CLI args and the `drone` table row written at registration time.

Send-side emits `MessageFromDrone` (Plane's enum, extended with our `AckExtensionAction`); receive-side dispatches `MessageToDrone` (Plane's enum, extended with our `PushNetworkPolicy`, `PushCredential`, `RevokeCredential`, `Snapshot`). Reconnects with exponential backoff; the controller redelivers any unacked actions from the `backend_action` and `extension_action` tables on reconnect.

#### 4.2.2 Command dispatcher
The drone receives `MessageToDrone::Action(BackendActionMessage)` for Plane's two action kinds (`Spawn`, `Terminate`) and `MessageToDrone::{PushNetworkPolicy, PushCredential, RevokeCredential, Snapshot}` for our four extension kinds. Each handler is short and delegates to a worker. The dispatcher dedupes on `action_id` (because Plane's redelivery model is at-least-once, not at-most-once).

#### 4.2.3 OCI runtime adapter
Translates platform-level commands into Docker API calls. The runtime backend (`runc`, `runsc`, `kata-runtime`) is selected per-sandbox via Docker's `HostConfig.Runtime` field, which maps to Plane's `--docker-runtime` startup option for the drone.

Spawn flow:
1. Pull image (skip if cached) via the local pull-through registry.
2. `docker create` with: image, env, port bindings, requested runtime, cgroup limits, labels.
3. Wait for the container's port to be listening (Plane's existing readiness mechanism).
4. Report `ready`.

Critical detail: every spawned container gets a per-sandbox tap or veth pair set up by the drone's network module (not shown above for brevity), with iptables NAT rules that route all outbound traffic through the egress proxy in the host's root namespace.

#### 4.2.4 Snapshot worker
Owns the snapshot procedure ([spec §11.2](./sandbox-platform-spec.md#112-snapshot-procedure)). Pause container → `docker commit` → tag → `docker push` to the local registry → register with controller → unpause.

Restore: pull image from registry → create container with snapshot image as rootfs → start.

#### 4.2.5 Policy applier
Receives network-policy and credential-binding pushes from the controller. Translates them to the egress proxy's xDS feed (if Envoy) or to direct config (if a custom proxy). Reloads the proxy with no traffic interruption.

#### 4.2.6 Metering reader
Samples cgroup v2 stats every 5 seconds for every running container. Computes deltas. Batches every 30 seconds; ships via the WebSocket. See [spec §14.1](./sandbox-platform-spec.md#141-metering) for paths and units.

### 4.3 Proxy

Mostly stock Plane proxy, with two extensions for our use case.

```
┌──────────────────────────── Proxy process ────────────────────────────────────┐
│                                                                               │
│   ┌────────────────────┐    ┌────────────────────┐                            │
│   │ TLS terminator     │    │ Controller WS      │                            │
│   │ (ACME-managed)     │    │ client             │                            │
│   └─────────┬──────────┘    └─────────┬──────────┘                            │
│             │                         │                                       │
│             ▼                         ▼                                       │
│   ┌────────────────────────────────────────────┐                              │
│   │   Connection router                        │                              │
│   │     - parses URL or subdomain              │                              │
│   │     - resolves to (drone_ip, container_port)│                             │
│   │     - reverse-proxies bytes                │                              │
│   └─────────┬──────────────────────────────────┘                              │
│             │                                                                 │
│             ├──── route cache ──── keepalive emitter ──── BackendRemoved hook │
│             │     (token →                                                    │
│             │      RouteInfo)                                                 │
│             ▼                                                                 │
│   to drone:container_port over the cluster's internal network                 │
│                                                                               │
└───────────────────────────────────────────────────────────────────────────────┘
```

#### 4.3.1 Connection router
Two routing modes, both terminating at the same `RouteInfo` (drone IP + container port):
- **Token path** (Plane standard): URL is `https://{cluster}/{token}/...`. Parse token, **request** route info from the controller via `MessageFromProxy::RouteInfoRequest { token }` (pull, not push), cache the response, reverse-proxy.
- **Subdomain port** (our extension): URL is `https://sb-abc-3000.sandboxes.example.com/...`. Parse `(sandbox_id_prefix, port)` from the subdomain, look up via a separate cache populated by our `SubdomainPortRouteUpdate` extension message (push, because the cache is small and addressable by stable keys).

Token-path lookups miss into the controller. `BackendRemoved { backend }` messages invalidate cached entries when backends terminate.

#### 4.3.2 ACME-managed TLS
Plane already implements [ACME DNS-01](https://plane.dev/deploy-to-prod) for per-cluster certs, with multi-proxy lease coordination via the `acme_txt_entries` table. Flow: `CertLeaseRequest → SetTxtRecord → ReleaseCertLease` over the proxy WebSocket. We use a wildcard cert for the per-port subdomain space.

#### 4.3.3 KeepAlive (idle detection input)
The proxy sends `MessageFromProxy::KeepAlive(BackendName)` for each backend that has live traffic. The controller writes to `backend.last_keepalive`; the controller's idle-timeout logic compares this against `allowed_idle_seconds` to decide when to terminate. There is no separate periodic "connection count" message.

### 4.4 Egress proxy

A separate process per host, in the root network namespace. Envoy by default; an alternative custom proxy is possible.

```
┌──────────────────────── Egress proxy (Envoy) ─────────────────────────────────┐
│                                                                               │
│   ┌────────────────────┐    ┌────────────────────┐                            │
│   │ xDS feeder         │◀───│ Drone control      │                            │
│   │ (config from drone)│    │ socket             │                            │
│   └─────────┬──────────┘    └────────────────────┘                            │
│             │ pushes Envoy config when policies / bindings change             │
│             ▼                                                                 │
│   ┌────────────────────────────────────────────────────────────┐              │
│   │ Listeners                                                  │              │
│   │   - 0.0.0.0:80   HTTP, plain text                          │              │
│   │   - 0.0.0.0:443  TLS passthrough OR MITM (per destination) │              │
│   │   - 0.0.0.0:53   UDP/TCP DNS proxy                         │              │
│   │  iptables in each sandbox netns redirects all outbound     │              │
│   │  traffic here                                              │              │
│   └─────────┬─────────────────────────────────────────┬────────┘              │
│             │                                         │                       │
│             ▼                                         ▼                       │
│   ┌────────────────────┐                ┌────────────────────────┐            │
│   │ Policy evaluator   │                │ Credential injector    │            │
│   │ (SNI-based;        │                │ (MITM TLS, add header) │            │
│   │  deny precedence)  │                │                        │            │
│   └─────────┬──────────┘                └────────────┬───────────┘            │
│             │ allowed                                │                        │
│             ▼                                        │                        │
│   ┌────────────────────────────────────────────────┴──────────────┐           │
│   │              Upstream connection pool                         │           │
│   │     forwards to the real destination over TLS                 │           │
│   └───────────────────────────────────────────────────────────────┘           │
│                                                                               │
└───────────────────────────────────────────────────────────────────────────────┘
```

#### 4.4.1 Listeners
One per protocol. Iptables rules inside each sandbox container's network namespace `DNAT` outbound traffic to the egress proxy's listener on the host. The drone installs these rules at sandbox spawn time, removes them at teardown.

#### 4.4.2 Policy evaluator
Reads the policy from xDS config. For TLS, parses the ClientHello's SNI. For HTTP, parses the Host header. For DNS, parses the question. Evaluates against the policy (deny → block; allow with inject → MITM; allow → passthrough; default deny in `rules` mode).

#### 4.4.3 Credential injector
For destinations with an `inject` rule, the proxy terminates the inbound TLS using a per-sandbox internal CA. The sandbox's image has the internal CA root in its trust store, installed at boot by `agentd`. The proxy adds the configured header, opens a fresh TLS connection to the real destination, and pipes bytes.

The internal CA private key never leaves the egress proxy. CAs are per-sandbox so a single breach doesn't fleet-poison.

#### 4.4.4 xDS feeder
Receives config updates from the drone (which received them from the controller). Pushes them into Envoy's xDS APIs (LDS, RDS, CDS, EDS). Update latency target: < 500 ms p95 from controller-decided to in-effect.

### 4.5 Sandbox container

Ephemeral. Two image flavors:

- **Headless.** Base distro + `agentd` + a runtime (Node, Python, Bun). Used for code-execution sandboxes.
- **Desktop.** Headless + Xvfb + openbox + Chromium + `agentd`'s computer-use endpoints. Used for visual-agent sandboxes.

```
┌──────────────── Sandbox container (OCI) ─────────────────────────┐
│                                                                  │
│   PID 1: tini                                                    │
│           │                                                      │
│           ├─ agentd  (HTTP :8080, the platform's contact point)  │
│           │                                                      │
│           ├─ user workload (started on demand by agentd)         │
│           │                                                      │
│           └─ [desktop only]                                      │
│                ├─ Xvfb :99                                       │
│                ├─ openbox                                        │
│                ├─ x11vnc + noVNC (optional human observation)    │
│                └─ Chromium / other GUI apps                      │
│                                                                  │
│   Filesystem: read-only base layers + per-sandbox writable layer │
│   Network: own netns, single veth to host bridge,                │
│            all outbound via egress proxy                         │
│   cgroups: per-container limits set by drone at spawn            │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

### 4.6 agentd

The in-sandbox HTTP daemon. Its job is to expose a stable contract to the platform regardless of the user code running alongside it.

```
┌────────────────────── agentd (Go or Rust) ───────────────────────┐
│                                                                  │
│   ┌──────────────────┐                                           │
│   │ HTTP/WS server   │  binds 0.0.0.0:8080                       │
│   │ (Axum / fiber)   │                                           │
│   └────────┬─────────┘                                           │
│            │                                                     │
│   ┌────────┼─────────┬─────────────┬──────────────┐              │
│   ▼        ▼         ▼             ▼              ▼              │
│ ┌────┐ ┌──────┐ ┌────────┐  ┌────────────┐  ┌─────────────┐      │
│ │heal│ │exec  │ │ fs     │  │ computer   │  │ bootstrap   │      │
│ │th  │ │engine│ │handler │  │ (desktop   │  │ (CA install,│      │
│ │    │ │      │ │        │  │  flavor)   │  │  initial    │      │
│ │    │ │PTY,  │ │read,   │  │            │  │  setup)     │      │
│ │    │ │detach│ │write,  │  │xdotool /   │  │             │      │
│ │    │ │ed    │ │watch,  │  │XTest, screen│ │             │      │
│ │    │ │sess. │ │stream  │  │shot via    │  │             │      │
│ │    │ │      │ │        │  │XGetImage   │  │             │      │
│ └────┘ └──────┘ └────────┘  └────────────┘  └─────────────┘      │
│                                                                  │
│   PID 1 (via tini); exits when the parent (tini) gets SIGTERM    │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

#### 4.6.1 Health endpoint
`GET /health` returns 200 once all sub-components are initialized. Used by the drone for readiness.

#### 4.6.2 Exec engine
Runs commands. Supports synchronous (wait-and-return) and detached (return id, stream later) modes. Maintains a small in-memory table of running commands. Each command gets a PTY or pipe pair, stdout/stderr streamed over WebSocket on demand.

#### 4.6.3 Filesystem handler
Read/write/list/mkdir/remove/watch. The watch endpoint uses `inotify` and streams change events. All operations are scoped to a configurable root (`/workspace` by default).

#### 4.6.4 Computer-use handler (desktop only)
Implements the Anthropic-schema action set ([spec §8.2](./sandbox-platform-spec.md#82-endpoints-desktop-flavor)). v0 shells out to `xdotool` and `scrot`/`gnome-screenshot`. v1 calls XTest and XGetImage directly via C bindings, for sub-30-ms action latency.

#### 4.6.5 Bootstrap
Runs once at startup. Installs the internal CA root into the container's trust store (so MITM credential injection works). Optionally fetches source from git per the create request's `source` field.

### 4.7 SDK

Two client libraries (TypeScript, Python); same shape and protocol. Light:

```
┌────────────────── SDK ─────────────────┐
│                                        │
│   Sandbox        ◀── public class      │
│    .create       │  with create/list/  │
│    .list         │  fromSnapshot/...   │
│   ─────────      │                     │
│   instance:                            │
│    .runCommand                         │
│    .writeFiles / .readFile             │
│    .domain(port)                       │
│    .updateNetworkPolicy                │
│    .snapshot                           │
│    .stop                               │
│    .extendTimeout                      │
│                                        │
│   Internal:                            │
│    - Auth: OIDC token or API key       │
│    - Token cache                       │
│    - Connection broker (Plane token →  │
│      backend URL)                      │
│    - Streaming helpers (logs())        │
│    - Retry/backoff                     │
└────────────────────────────────────────┘
```

The SDK is the only place a caller talks HTTP. Once a sandbox is up, command execution and file ops go to the data-plane URL (via the Plane proxy), bypassing the controller. The SDK manages this transparently.

---

## 5. Runtime views

Six scenarios that exercise the architecture. Each is a sequence diagram followed by a few annotations.

### 5.1 Sandbox cold start

```
Caller     SDK      Controller    Drone        Container      Proxy
  │         │            │           │              │            │
  │ create()│            │           │              │            │
  ├────────▶│            │           │              │            │
  │         │  POST /pub/v1/sandboxes│              │            │
  │         ├───────────▶│           │              │            │
  │         │            │ (build Plane ConnectRequest)         │
  │         │            │  ┌── POST /ctrl/connect ──┐           │
  │         │            │  │  acquire key, write    │           │
  │         │            │  │  backend row, write    │           │
  │         │            │  │  backend_action row    │           │
  │         │            │  │  (kind=Spawn),         │           │
  │         │            │  │  emit notification     │           │
  │         │            │  └────────────────────────┘           │
  │         │            │                                       │
  │         │ ConnectResponse {                                  │
  │         │   backend_id, spawned: true,                       │
  │         │   status: "scheduled",                             │
  │         │   token, url, status_url, drone                    │
  │         │ }                                                  │
  │         │◀───────────┤                                       │
  │         │                                                    │
  │         │ (wrap in our CreateSandboxResponse)                │
  │ sandbox │                                                    │
  │◀────────┤                                                    │
  │         │            │                                       │
  │         │            │  (controller replica owning the drone │
  │         │            │   WS picks up the notification)       │
  │         │            │                                       │
  │         │            │  MessageToDrone::Action(              │
  │         │            │    BackendActionMessage {             │
  │         │            │      action_id: "ak-...",             │
  │         │            │      action: Spawn { ... } } )        │
  │         │            ├──────────▶│                           │
  │         │            │           │  docker pull (cached)     │
  │         │            │           │  docker create            │
  │         │            │           │  install iptables in netns│
  │         │            │           │  docker start             │
  │         │            │           ├─────────────▶│            │
  │         │            │           │              │ agentd up  │
  │         │            │  MessageFromDrone::BackendEvent       │
  │         │            │    { state: Loading }                 │
  │         │            │◀──────────┤                           │
  │         │            │  AckEvent { event_id } ──▶            │
  │         │            │  ... Starting, Waiting, Ready ...     │
  │         │            │           │  poll /health             │
  │         │            │           ├─────────────▶│            │
  │         │            │           │◀─────────────┤            │
  │         │            │  BackendEvent { state: Ready {        │
  │         │            │    address: "10.0.0.5:NNNN" } }       │
  │         │            │◀──────────┤                           │
  │         │            │  AckAction { action_id } ──▶          │
  │         │            │                                       │
  │  (SDK was streaming /pub/v1/sandboxes/:id/status this whole  │
  │   time; sees the "ready" event and resolves the create promise
  │   if its options say to wait-for-ready)                      │
  │         │                                                    │
  │         │  later: runCommand() goes to                       │
  │         │  https://cluster/token/v1/exec via the proxy ─────▶│
  │         │                                                    ├──▶ agentd
```

**Verified flow:** `POST /ctrl/connect` returns immediately after the controller writes the `backend_action` row — it does not wait for the drone to actually start the container. The SDK either (a) returns the create result with `status: "scheduled"` and lets the caller wait via the status stream, or (b) blocks the create call until it observes `ready` on the stream. The default for our SDK is (b) with a configurable timeout.

Latency budget (cold, image cached): 1.5 s p95. The components on the critical path: controller HTTP round-trip (~50 ms), drone receive-and-dispatch (~10 ms), `docker create + start` under runsc (~300 ms), agentd boot (~150 ms), readiness poll (~100 ms), response (~50 ms). Most of the headroom is in `docker create` and `agentd` boot.

### 5.2 Restore from snapshot

```
Caller   SDK      Controller    Drone        Registry      Container
  │        │           │           │             │             │
  │ create({source:{type:'snapshot',snapshotId}})│             │
  ├───────▶│           │           │             │             │
  │        │ POST /pub/v1/sandboxes (source.type=snapshot)     │
  │        ├──────────▶│           │             │             │
  │        │           │ load snapshot row                     │
  │        │           │ → image_ref, base_image, arch         │
  │        │           │                                       │
  │        │           │ build Plane ConnectRequest with       │
  │        │           │ spawn_config.executable.image =       │
  │        │           │   <snapshot image_ref>                │
  │        │           │ POST /ctrl/connect → ConnectResponse  │
  │        │ wrapped CreateSandboxResponse                     │
  │        │◀──────────┤                                       │
  │ sandbox│                                                   │
  │◀───────┤                                                   │
  │        │           │  MessageToDrone::Action(Spawn) ──▶    │
  │        │           │           │  docker pull <image_ref>  │
  │        │           │           ├────────────▶│             │
  │        │           │           │◀────────────┤             │
  │        │           │           │  (base layers cached;     │
  │        │           │           │   only snapshot diff pulled)
  │        │           │           │                           │
  │        │           │           │  docker create + start    │
  │        │           │           ├──────────────────────────▶│
  │        │           │           │                           │ agentd up
  │        │           │  BackendEvent { Ready { address } }   │
  │        │           │◀──────────┤                           │
  │  (SDK status stream observes Ready, resolves create promise)
```

Latency budget: 800 ms p95 when base layers cached, snapshot layer < 100 MB.

### 5.3 Egress request with credential injection

```
Sandbox       Egress proxy            External
container     (host)                  API
   │            │                       │
   │ DNS query: api.example.com        │
   ├───────────▶│                       │
   │            │  policy: allow + inject=cred-xyz
   │            │  return real IP
   │◀───────────┤                       │
   │                                    │
   │ TCP 443 → egress (iptables DNAT)   │
   ├───────────▶│                       │
   │            │  TLS ClientHello with SNI=api.example.com
   │            │  policy match: inject
   │            │  terminate TLS using per-sandbox CA
   │◀───────────┤                       │
   │ TLS handshake using internal CA    │
   │ (agentd already installed root)    │
   │            │                       │
   │ HTTP GET /v1/chat (no Authorization header)
   ├───────────▶│                       │
   │            │  fetch secret from KMS-encrypted store (in memory)
   │            │  add header: Authorization: Bearer sk-...
   │            │  open new TLS to api.example.com
   │            ├──────────────────────▶│
   │            │  HTTP 200 + JSON      │
   │            │◀──────────────────────┤
   │  HTTP 200  │                       │
   │◀───────────┤                       │
```

The sandbox never sees the secret. If the code is fully compromised, an attacker can use the authenticated channel toward `api.example.com` (within whatever rate limits we've configured), but cannot read the credential out.

### 5.4 Snapshot create

```
Caller   SDK    Controller    Drone           Container        Registry
  │       │         │           │                │                │
  │ sb.snapshot()   │           │                │                │
  ├──────▶│         │           │                │                │
  │       │ POST /pub/v1/sandboxes/:id/snapshot  │                │
  │       ├────────▶│           │                │                │
  │       │         │ resolve drone holding the sandbox           │
  │       │         │ INSERT extension_action (kind=snapshot)     │
  │       │         │ NOTIFY drone's controller-replica           │
  │       │         │                                             │
  │       │         │ MessageToDrone::Snapshot (our extension)    │
  │       │         ├──────────▶│                │                │
  │       │         │           │ docker pause   │                │
  │       │         │           ├───────────────▶│                │
  │       │         │           │                │ (frozen)       │
  │       │         │           │ docker commit  │                │
  │       │         │           ├───────────────▶│                │
  │       │         │           │ docker tag + push               │
  │       │         │           ├────────────────────────────────▶│
  │       │         │           │ docker unpause + rmi local      │
  │       │         │           ├───────────────▶│                │
  │       │         │                                             │
  │       │         │ ◀── POST /ctrl/snapshots                    │
  │       │         │     RegisterSnapshotRequest { ... }         │
  │       │         │ INSERT snapshot row, emit snapshot_registered
  │       │         │                                             │
  │       │         │ ◀── MessageFromDrone::AckExtensionAction    │
  │       │         │     { action_id }   (our extension)         │
  │       │         │ UPDATE extension_action SET acked_at = now()│
  │       │         │                                             │
  │       │  HTTP 201 { snapshot: { id, size_bytes, ... } }       │
  │       │◀────────┤                                             │
  │ snap  │                                                       │
  │◀──────┤                                                       │
```

The Plane controller is not on the hot path of the snapshot operation itself — it's just the durability surface and the orchestration of `extension_action` rows. The actual work (`docker commit + push`) happens entirely on the drone. The SDK call is held open by the controller waiting on the `snapshot_registered` event, then returns the resulting `Snapshot` row.

Pause-to-unpause is the user-visible latency hit. Mostly bounded by `docker commit`'s layer scan and the push's network bandwidth to the local registry. For ~50 MB diffs on local registry, well under 2 seconds. Larger diffs scale linearly.

### 5.5 Persistent sandbox auto-snapshot on stop

```
... idle timeout fires ...

Controller    Drone           Container       Registry
   │           │                  │              │
   │ MessageToDrone::Action(Terminate { kind: soft }) ──▶
   │           │                  │              │
   │           │ on the drone, before docker stop:│
   │           │  if backend's sandbox.persistent │
   │           │  is true, run snapshot procedure │
   │           │  (pause / commit / push /        │
   │           │   unpause / register).           │
   │           │  Only THEN proceed to terminate. │
   │           │                                  │
   │           │ docker stop -t <grace>           │
   │           ├─────────────────▶│               │
   │           │  (SIGTERM, then SIGKILL on grace expiry)
   │           │                                  │
   │           │ tear down netns, iptables, etc.  │
   │           │                                  │
   │           │ MessageFromDrone::BackendEvent   │
   │           │   { state: Terminated { ... } }  │
   │◀──────────┤                                  │
   │ AckEvent ──▶                                 │
   │                                              │
   │ MessageFromDrone::AckAction { action_id }    │
   │◀──────────┤                                  │
   │ UPDATE sandbox SET snapshot_source = id      │
   │  (via the snapshot_registered event)         │
```

Subsequent `Sandbox.create({ source: { type: 'snapshot', snapshotId } })` (or our SDK's higher-level `Sandbox.resume(sandboxId)` wrapper) follows the restore path (§5.2).

### 5.6 Drone failure

```
Drone goes away (network partition, crash, host reboot)

Controller                    Drone (dead)
    │                              X
    │  Missed heartbeats:          │
    │  no MessageFromDrone::Heartbeat
    │  for > heartbeat_threshold   │
    │                              │
    │  Mark node.controller = NULL │
    │  (drone is offline)          │
    │                              │
    │  Sweeper writes BackendState │
    │  Terminated { reason: lost,  │
    │    termination: null,        │
    │    exit_code: null,          │
    │    last_status: <whatever it was> }
    │  for every backend on this drone
    │                              │
    │  Existing /pub/b/:b/status-stream subscribers
    │  see the terminal event and disconnect.
```

We don't migrate live sandboxes. A sandbox is bound to its drone; if the drone dies, the sandbox dies. Callers receive a `BackendState::Terminated { reason: lost }` and can decide whether to retry (with or without a snapshot). Plane's `TerminationReason::Lost` exists for exactly this case.

### 5.7 Controller failover

Multiple controller replicas behind a load balancer. State in Postgres; coordination via LISTEN/NOTIFY. A replica dying drops its WebSocket connections to drones/proxies; they reconnect to whichever replica the load balancer assigns next. No leader election (inherited from Plane's stateless-controller design).

The only "stickiness" is that a long-running streaming SSE response from `/pub/b/.../status-stream` ends if the replica behind it dies. The SDK reconnects transparently.

---

## 6. Deployment view

The platform deploys to two distinct topologies: a dev fleet on developer workstations, and a production fleet on cloud + edge hardware.

### 6.1 Production topology

```
                              Internet
                                  │
                       ┌──────────┴────────────┐
                       │  Load balancer / WAF  │
                       │  (ALB, Cloudflare)    │
                       └──────────┬────────────┘
                                  │
              ┌───────────────────┴───────────────────┐
              │                                       │
              ▼                                       ▼
       ┌─────────────┐                        ┌─────────────┐
       │ Controller  │                        │   Proxy     │
       │ replicas    │                        │  replicas   │
       │  (3+)       │                        │   (2+)      │
       └──────┬──────┘                        └──────┬──────┘
              │                                      │
              │      ┌───────────────────────────────┘
              │      │
              ▼      ▼
       ┌──────────────────┐
       │     Postgres     │ ◀── managed (RDS, Cloud SQL),
       │     (primary +   │     PITR + cross-AZ replica
       │      replica)    │
       └──────────────────┘

                ──────────────────────────────────────────

Drone fleet:

   x86 hosts (EC2 c8i / m8i with nested virt enabled)
     ├─ pool=trusted    (runc; CI builds, internal)
     ├─ pool=untrusted  (runsc; AI agents, user code)  ◀── default
     └─ pool=paranoid   (Kata + Firecracker)
                          requires nested virt or .metal

   ARM hosts (Raspberry Pi 5 / Graviton .metal)
     ├─ pool=trusted    (runc)
     ├─ pool=untrusted  (runsc)
     └─ pool=paranoid   (Kata, .metal only — Graviton nested-virt
                         not supported as of Feb 2026)

Each drone runs:
   - plane-drone (Rust binary, our fork)
   - envoy (egress proxy)
   - docker daemon + one or more OCI runtimes
   - per-host pull-through registry (read-only cache)
```

### 6.2 Networking

- **Public traffic** lands at the load balancer, routes to controller (for `/pub/*`) or proxy (for backend URLs).
- **Control-plane traffic** (drone↔controller, proxy↔controller) over WebSocket, on a private network or VPC.
- **Egress traffic** from sandboxes goes through the per-host egress proxy, out via NAT.
- **Inter-host traffic** (proxy → drone:container_port) over a private network.

### 6.3 Image distribution

```
                  Central registry (Distribution + S3)
                     │
              ┌──────┴──────┐
              │             │
              ▼             ▼
     ┌────────────┐  ┌────────────┐
     │ Host A:    │  │ Host B:    │
     │ pull-      │  │ pull-      │
     │ through    │  │ through    │
     │ cache      │  │ cache      │
     │ (registry: │  │ (registry: │
     │ 2)         │  │ 2)         │
     └────────────┘  └────────────┘
```

Per-host pull-through caches mean the second sandbox using the same image on the same host pulls at local-disk speed. New images warm caches on demand; pre-warming on critical paths is an optimization we apply selectively (e.g., for the default Node 24 image).

Snapshots are pushed by their drone of origin to the central registry, then pullable by any drone — that's how a restored sandbox can land on a different host.

### 6.4 Dev topology

VirtualBox VMs as drones on a developer workstation. See [the dev-fleet README](./dev-fleet/README.md) for the full setup. Same control plane, same SDK, same images — running with our forked controller and runtime images.

---

## 7. Data architecture

### 7.1 What lives where

| Data | Store | Retention | Access pattern |
|---|---|---|---|
| Sandbox metadata, lifecycle state | Postgres | While running + a few hours | Write-heavy at create/transition; read on lookup |
| Snapshot metadata | Postgres | 30 days default, configurable | Write at snapshot, read at restore |
| Network policies, credential bindings | Postgres | Lifetime of sandbox | Read on policy push, write on update |
| Secret ciphertexts | Postgres | Indefinite | Read on bind |
| Usage records | Postgres (hourly buckets) | 90 days hot, then archive | Append-heavy, range reads at billing |
| Sandbox runtime images | OCI Registry (S3-backed) | Indefinite for tagged | Read at sandbox spawn |
| Snapshot images | OCI Registry | Tied to snapshot retention | Read at restore |
| Container logs (stdout/stderr) | Drone disk → controller stream → log store | 7 days default | Write-heavy, occasionally read |
| Metrics | Prometheus / equivalent | 30 days | Write-heavy, dashboarded |
| Audit logs (egress decisions, cred injections) | Append-only log store | 1 year for paranoid tier | Write-heavy, occasionally queried |

### 7.2 Postgres as the source of truth

Inherited from Plane. Everything that needs to survive a process restart is in Postgres. Coordination between controller replicas is via Postgres LISTEN/NOTIFY — no separate message bus.

This makes Postgres a single point of failure unless run as HA. We run a primary with a synchronous replica in a different AZ, with PITR backups.

### 7.3 No object store for snapshots

The original design (see [spec §11](./sandbox-platform-spec.md#11-snapshots-and-persistence)) used an object store (S3) for snapshot blobs. We pivoted to OCI Registry because it's already in the system, content-addresses and deduplicates blobs natively, and integrates with the same image-pull path drones already use.

### 7.4 Clock discipline

Plane's design discipline: the controller doesn't trust its own clock for cross-replica decisions; it uses Postgres's clock. Drones use their local clock for key-lease expiry (so a partitioned drone can self-terminate its leases on time). We preserve this discipline.

---

## 8. Cross-cutting concerns

### 8.1 Security

Layered defense, in order from the sandbox outward:

1. **Container isolation.** Default `runsc` (gVisor) for untrusted code; the host kernel is never touched directly. `paranoid` tier upgrades to Kata + Firecracker for hardware-virt isolation.
2. **Capability and seccomp drops.** Default Docker capability set further restricted by per-tier policies.
3. **Network namespace isolation.** Each container has its own netns, no shared bridge, no inter-sandbox routing.
4. **Egress mediation.** All outbound traffic through the host's egress proxy. SNI-aware policy enforcement. Default deny in rules mode.
5. **Credential brokering.** Secrets never enter the sandbox process. Sandboxes that need third-party credentials reach a host-mediated proxy that injects them.
6. **Drone privilege.** Only the drone holds the Docker socket. Compromising the controller is bad but doesn't directly grant container spawn.
7. **Postgres encryption-at-rest.** Standard for the storage layer. Application-level KMS encryption for secret material.

What we do *not* defend against:
- A compromised host kernel. If the host kernel is owned, gVisor doesn't help.
- Side channels (Spectre, Meltdown). Mitigated only on `paranoid` tier (microVM boundary).
- The customer giving their own sandbox unsafe credentials. The platform mediates injection by destination; what the destination does with the request is up to it.

### 8.2 Observability

Three pillars, each on a separate path:

- **Metrics.** Prometheus endpoints on controller, proxy, drone, egress proxy. Labels: tenant, pool, arch, sandbox id (for short-window views).
- **Logs.** Structured logs to stdout from every component; collected by host agents (Vector/Fluent Bit). Container stdout/stderr collected by the drone, forwarded to the controller's streaming endpoint for SDK consumption and to long-term storage.
- **Traces.** OpenTelemetry from SDK → controller → drone → agentd. Critical for diagnosing cold-start latency tail.

Audit log (security-sensitive, separate from operational logs):
- Every credential injection.
- Every blocked egress.
- Every paranoid-tier sandbox creation.
- Every snapshot create/restore.

### 8.3 Reliability and disaster recovery

| Failure | Blast radius | Recovery |
|---|---|---|
| Drone crash | Sandboxes on that drone | Marked `lost`; persistent sandboxes recoverable from last snapshot |
| Drone network partition | Same as crash from controller's view | Drone reconnects, controller reconciles state |
| Controller replica crash | In-flight WebSocket streams from SDK | LB shifts traffic; clients reconnect |
| Postgres primary loss | All write paths until failover | Sync replica promoted; PITR for catastrophic loss |
| Egress proxy crash | Sandboxes on that host can't reach internet | systemd restart; <5 s outage typical |
| Region loss | Entire platform | Manual failover (no v1 multi-region); RPO measured in minutes via PITR; RTO hours |
| OCI registry loss | New sandbox creation; existing sandboxes unaffected | Per-host caches keep running sandboxes alive; rebuild registry from object storage |

### 8.4 Performance budgets

| Path | p95 budget |
|---|---|
| `Sandbox.create()` cold (image cached) | 1.5 s |
| `Sandbox.create()` warm (from snapshot, base cached) | 800 ms |
| `runCommand()` round-trip | 50 ms (excluding command runtime) |
| `writeFile()` 1 MB | 200 ms |
| `screenshot()` (desktop, 1280×800 PNG) | 80 ms |
| `click()` (desktop) | 30 ms |
| Egress proxy added latency (allowed, passthrough) | 5 ms |
| Egress proxy added latency (MITM with injection) | 25 ms |
| Snapshot create (50 MB diff) | 2 s |
| Network policy update propagation | 500 ms |

These are design budgets; actual measurement happens once each phase ships.

### 8.5 Versioning and compatibility

Three things have separate version stories:

- **SDK.** Semver. Breaking changes require a major bump. Old clients keep working until we sunset their major.
- **Public HTTP API.** Versioned by URL path (`/pub/v1/`). Additions are backward-compatible; breaking changes require `/pub/v2/`.
- **`agentd` ↔ platform contract.** The drone speaks to `agentd` over HTTP. Each runtime image is built with a known `agentd` version; the drone tolerates a range.

The `executable` field in spawn requests is a generic JSON value (Plane's convention) so it can evolve without protocol changes.

---

## 9. Risks and known limitations

| # | Risk / limitation | Mitigation | When to revisit |
|---|---|---|---|
| 1 | gVisor syscall coverage gaps break some workloads (e.g., low-level eBPF, certain JVM GC paths) | Document supported surface; recommend `paranoid` tier for affected workloads | Per quarter; tied to gVisor releases |
| 2 | No live migration; drone failure ends sandboxes | Persistent snapshots enable manual recovery | When customer demand justifies CRIU or Firecracker memory snapshots |
| 3 | Single-region | Document explicitly; users design accordingly | When >1% of traffic comes from outside iad latency cone |
| 4 | Snapshot semantics differ from process snapshots (no running state) | Document the distinction; provide pre-snapshot hooks for graceful state flush | If a customer specifically needs running-process snapshots |
| 5 | Per-host egress proxy is a SPOF for that host's sandboxes | Local restarts; multi-AZ deployment of drone fleet | If MTTR exceeds SLA budgets |
| 6 | Postgres on the hot path of every create | Read replicas for non-write paths; LISTEN/NOTIFY for fanout | If DB CPU > 60% sustained |
| 7 | Active-CPU billing depends on accurate cgroup sampling | Sample at 5 s; record raw + smoothed | Audit quarterly against ground truth |
| 8 | Trust tier is a coarse knob; some workloads want fine-grained syscall policies | runc + custom seccomp as a future tier | Per customer ask |

---

## 10. Architectural decisions

ADRs in the format: **Context → Decision → Consequences**.

### ADR-001: Plane as orchestration substrate

**Context.** We need a session-backend orchestrator: per-key process, TLS-fronted, lifecycle-managed. Building from scratch would take months; Kubernetes is the wrong shape (long-lived workloads, no per-key routing primitive); ECS/Nomad similar.

**Decision.** Adopt [Plane](https://plane.dev) as the substrate. Use stock components where possible; fork the controller and proxy with backward-compatible extensions.

**Consequences.**
- ✅ Lifecycle, scheduling, key locks, ACME, idle detection — all free.
- ✅ Open source, MIT, small surface area we can read end-to-end.
- ⚠️ Coupling to Plane's release cadence. We need to track upstream and merge.
- ⚠️ Plane is designed for "isolated code environments" but doesn't include the hardening we need; we add the egress proxy and credential brokering on top.

### ADR-002: OCI containers (with tiered runtime) over pure microVMs

**Context.** Vercel Sandbox, E2B, Daytona all chose Firecracker microVMs. The microVM path requires nested virt or bare-metal, which restricts the fleet. The container path works everywhere.

**Decision.** Use OCI containers as the default isolation primitive. Allow per-tenant trust tier to upgrade to gVisor (untrusted) or Kata + Firecracker (paranoid).

**Consequences.**
- ✅ Fleet works on stock c8i/m8i, Pi 5, anything Linux. No nested-virt requirement for the default path.
- ✅ Operational simplicity. Same OCI tooling across tiers.
- ✅ gVisor gives meaningful isolation against kernel exploits without microVM cost.
- ⚠️ Doesn't get to Firecracker's sub-100 ms snapshot/restore. Our snapshot path is filesystem-only.
- ⚠️ Some workloads hit gVisor syscall gaps.

### ADR-003: `docker commit` for snapshots (not memory snapshots)

**Context.** Vercel's snapshot semantics are filesystem + installed packages, not process state. Our v1 doesn't need the latter. Memory snapshots (CRIU, Firecracker) are dramatically harder.

**Decision.** Snapshots are `docker commit` + push to the local registry. Restore is `docker pull` + start.

**Consequences.**
- ✅ Works with any OCI storage driver; no overlay2-specific tar acrobatics.
- ✅ Reuses the OCI Registry already in the stack.
- ✅ Content-addressed dedup for free.
- ⚠️ Pause-and-commit is a real outage for the sandbox; large diffs are slow.
- ⚠️ Doesn't preserve running processes; users must design for restart.

### ADR-004: Egress proxy as the credential-injection point

**Context.** Vercel injects credentials on egress so they never enter the sandbox. We need the same. The injection point must be outside the sandbox's trust boundary.

**Decision.** Per-host egress proxy (Envoy) is the credential injection point. TLS MITM with per-sandbox internal CA for destinations with `inject` rules.

**Consequences.**
- ✅ Credentials live in the controller's encrypted store and on the egress proxy in memory; never in the sandbox.
- ✅ Same component handles policy enforcement; one place to audit.
- ⚠️ MITM breaks certificate-pinned clients on injected destinations.
- ⚠️ Per-sandbox CA means lots of short-lived certs; we generate at sandbox spawn.

### ADR-005: `agentd` as the stable in-container contract

**Context.** We need a stable HTTP surface for command execution, file ops, and computer use, regardless of what runtime (Node, Python, etc.) is in the container.

**Decision.** Every runtime image embeds `agentd` as PID 1 (via tini). The platform talks to `agentd` over HTTP via the Plane proxy. User workloads are children of `agentd`.

**Consequences.**
- ✅ One contract, many runtimes.
- ✅ Stream stdio, expose ports, install certs at boot — one place.
- ⚠️ Every image build needs to bundle `agentd`. Solved via a base image layer.
- ⚠️ `agentd` itself becomes a security-critical surface inside the container.

### ADR-006: Postgres as the only stateful component

**Context.** Distributed systems benefit from having a clear source of truth. Plane chose Postgres; we extend that choice.

**Decision.** Everything persistent goes to Postgres. Coordination between controller replicas via LISTEN/NOTIFY. No Redis, no Kafka, no etcd.

**Consequences.**
- ✅ One thing to operate, back up, monitor.
- ✅ Strong consistency where we need it (key locks, lifecycle).
- ⚠️ Postgres on the hot path. Needs to be sized and tuned.
- ⚠️ Cross-region multi-master is not Postgres' strength. Multi-region is deferred to v2.

### ADR-007: WebSocket (not REST polling) from drones and proxies

**Context.** The controller needs to dispatch actions to drones (Plane's `Spawn` and `Terminate`, plus our extensions) and serve route-info lookups for proxies. Polling at high frequency from many hosts is wasteful; persistent WebSockets fit.

**Decision.** Each drone and proxy holds a single persistent WebSocket to the controller. Inherited from Plane. The drone channel is **push from controller** (dispatch actions) with durable redelivery from the `backend_action` (and our `extension_action`) table. The proxy channel is **request/response** (proxy asks `RouteInfoRequest { token }`, controller answers `RouteInfoResponse`) with `BackendRemoved` invalidations pushed.

**Consequences.**
- ✅ Low-latency bidirectional. < 100 ms control-plane round-trip.
- ✅ Reconnect-friendly; replicas survive client moves. Unacked actions redelivered.
- ⚠️ Drones must dedupe by `action_id` because delivery is at-least-once.
- ⚠️ Long-lived connections need careful timeout handling and load balancer config.

### ADR-008: Cluster names encode dev/prod via port presence

**Context.** Plane's convention: a cluster name like `localhost:9090` is dev (HTTP only, no ACME); `sandboxes.example.com` is prod (HTTPS via ACME). We need both.

**Decision.** Adopt the convention unchanged. Dev cluster: `192.168.56.1:9090`. Prod cluster: `sandboxes.example.com`.

**Consequences.**
- ✅ Same code paths run dev and prod with one config knob.
- ✅ Dev cluster works without DNS or certs.
- ⚠️ Cluster names with ports look unusual in URLs; document this.

### ADR-009: Trust tier as a per-sandbox flag mapped to a Plane pool

**Context.** Different workloads need different isolation strength. Default `untrusted` for safety; opt-down to `trusted` for performance; opt-up to `paranoid` for hardware-virt isolation.

**Decision.** Three named tiers. Each maps to a Plane drone pool. Scheduler routes by pool.

**Consequences.**
- ✅ Explicit, auditable, easy to reason about.
- ✅ Maps cleanly onto Plane's pool primitive — no new orchestration concept.
- ⚠️ Coarse-grained. Some workloads want hybrids (gVisor + per-syscall policy); deferred.

### ADR-010: SDK proxies long-running data-plane traffic, not control-plane traffic

**Context.** The controller is on the create path; if it were also on the per-command path, it would bottleneck.

**Decision.** `Sandbox.create()` hits the controller. Everything after that — `runCommand`, file ops, port URLs — talks directly to the data plane (Plane proxy → `agentd`) using the connection token.

**Consequences.**
- ✅ Controller scales independently of per-sandbox throughput.
- ✅ Mirrors Plane's design.
- ⚠️ Two URL spaces in the SDK (controller endpoint + per-sandbox URLs); the SDK manages this transparently.

---

## 11. Glossary

- **agentd** — The HTTP daemon inside every sandbox container. The stable contact point for the platform's data plane.
- **Backend** — Plane's term for a running process; in our system, equivalent to a sandbox container.
- **Cluster** — A pool of drones sharing a hostname. In dev, includes a port; in prod, doesn't.
- **Connection token** — A bearer URL segment minted by the controller, used by the proxy to route data-plane traffic to a specific sandbox.
- **Credential binding** — A `(sandbox, destination, header, secret)` tuple that tells the egress proxy to inject a header into matching outbound requests.
- **Drone** — The platform's per-host agent. Runs as a Docker container on the host, with the Docker socket mounted in.
- **Egress proxy** — The host-resident outbound interceptor. Enforces network policy and injects credentials.
- **Fencing token** — A monotonically-increasing version on each key acquisition; protects against stale drones double-claiming a key.
- **Key** — Plane's identity for a sandbox slot. Locked in Postgres while held.
- **Pool** — Plane's tag on a drone; we use it for trust tier (`trusted`, `untrusted`, `paranoid`).
- **Sandbox** — One ephemeral execution environment. Maps 1:1 to a Plane backend.
- **Snapshot** — A `docker commit`-ed image of a sandbox's filesystem at a point in time.
- **Trust tier** — Our SDK-level enum determining isolation strength; selects the OCI runtime.
