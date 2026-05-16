# Sandbox Platform — Data Contracts and Interfaces

**Status:** v0.2 (verified)
**Companions:** [spec](./sandbox-platform-spec.md), [SAD](./sandbox-platform-sad.md)

This document fixes every wire-level interaction. Every contract inherited from Plane is verified against the upstream source at <https://github.com/jamsocket/plane>; each such item names the source file. Every contract that is our extension is explicitly marked. If two components disagree, this file decides.

Notation: TypeScript-style types for readability; the wire format is JSON (UTF-8) unless stated otherwise. `?` marks optional fields. snake_case field names. Times are ISO 8601 UTC. Plane uses `lowercase` enum serialization (`scheduled`, not `Scheduled`) — assume that throughout.

---

## Table of contents

1. [Interface map](#1-interface-map)
2. [Cross-cutting conventions](#2-cross-cutting-conventions)
3. [Public API — SDK ↔ Controller (our extension)](#3-public-api--sdk--controller-our-extension)
4. [Control API — `/ctrl/*` (Plane + our additions)](#4-control-api--ctrl-plane--our-additions)
5. [Drone ↔ Controller WebSocket (Plane + our additions)](#5-drone--controller-websocket-plane--our-additions)
6. [Proxy ↔ Controller WebSocket (Plane + our additions)](#6-proxy--controller-websocket-plane--our-additions)
7. [Drone ↔ Egress Proxy (our extension)](#7-drone--egress-proxy-our-extension)
8. [Drone ↔ Docker (Docker Engine API)](#8-drone--docker-docker-engine-api)
9. [Data plane — Proxy ↔ agentd](#9-data-plane--proxy--agentd)
10. [agentd HTTP API (our extension)](#10-agentd-http-api-our-extension)
11. [Egress proxy ↔ internet (our extension)](#11-egress-proxy--internet-our-extension)
12. [Storage contracts](#12-storage-contracts)
13. [Internal coordination — Postgres LISTEN/NOTIFY](#13-internal-coordination--postgres-listennotify)
14. [Shared type reference](#14-shared-type-reference)
15. [Postgres schema](#15-postgres-schema)

---

## 1. Interface map

| # | Edge | Protocol | Initiator | Origin | Section |
|---|---|---|---|---|---|
| I-1 | SDK → Controller (public) | HTTP/1.1 + JSON | SDK | **ours** | [§3](#3-public-api--sdk--controller-our-extension) |
| I-2 | SDK → Controller (status stream) | SSE | SDK | mostly Plane | [§3.5](#35-sandbox-status-stream-sse) |
| I-3 | Admin/operator → Controller | HTTP/1.1 + JSON | Operator | Plane + ours | [§4](#4-control-api--ctrl-plane--our-additions) |
| I-4 | Drone ↔ Controller | WebSocket + JSON | Drone | **Plane** | [§5](#5-drone--controller-websocket-plane--our-additions) |
| I-5 | Proxy ↔ Controller | WebSocket + JSON | Proxy | **Plane** | [§6](#6-proxy--controller-websocket-plane--our-additions) |
| I-6 | Drone → Egress proxy | xDS (gRPC) over UDS | Drone | ours | [§7](#7-drone--egress-proxy-our-extension) |
| I-7 | Drone → Docker daemon | Docker Engine API + UDS | Drone | external | [§8](#8-drone--docker-docker-engine-api) |
| I-8 | End-user → Proxy | HTTPS / WSS | End-user | Plane URL format | [§9](#9-data-plane--proxy--agentd) |
| I-9 | Proxy → agentd | HTTP/1.1 / WS | Proxy | Plane + ours | [§9](#9-data-plane--proxy--agentd) |
| I-10 | Sandbox → Egress proxy | TCP via iptables DNAT | Sandbox kernel | ours | [§11](#11-egress-proxy--internet-our-extension) |
| I-11 | Egress proxy → External | TCP / TLS | Egress proxy | ours | [§11](#11-egress-proxy--internet-our-extension) |
| I-12 | Controller → Postgres | SQL (TCP, TLS) | Controller | Plane + ours | [§12.1](#121-postgres) |
| I-13 | Controller ↔ Controller | Postgres LISTEN/NOTIFY | Controller | Plane + ours | [§13](#13-internal-coordination--postgres-listennotify) |
| I-14 | Drone → OCI Registry | Docker Registry HTTP API V2 | Drone | external | [§12.2](#122-oci-registry) |
| I-15 | Controller → KMS | Cloud provider SDK | Controller | external | [§12.3](#123-kms--secret-storage) |

"Origin" tells you whose contract this is: Plane (we don't change), ours (we own), external (third-party spec), or a combination.

---

## 2. Cross-cutting conventions

### 2.1 Wire formats

- **JSON** (UTF-8) for HTTP request/response bodies and WebSocket messages. `Content-Type: application/json` or `application/x-ndjson` for streams.
- **Binary** for file content (raw bytes) and screenshots (PNG bytes), as `application/octet-stream` or `image/png` with `Content-Length`.
- **Server-Sent Events** (`text/event-stream`) for status streams.
- **WebSocket** frames are individual JSON messages, one per frame. Binary frames not used.

### 2.2 ID conventions

Plane uses entity-name strings with type-specific prefixes:

| Type | Plane prefix | Notes |
|---|---|---|
| `BackendName` | (none; random or user-specified) | The backend id; user-provided or `random_prefixed_string`. |
| `DroneName` | `dr-` | E.g., `dr-7f3d1a`. Drones identify themselves by this string. |
| `ProxyName` | `px-` | |
| `ControllerName` | `co-` | |
| `BackendActionName` | `ak-` | Per-action id used for acknowledgment. |
| `KeyConfig.name` (random) | `lk-` | When a random key is generated. |

Source: `common/src/names.rs` lines 160–165.

At the database level, **drones, proxies, and DNS servers all share a single `node` table with an integer `id`**. The string name is `node.name`, the integer is `node.id`. The `drone` table's primary key is this same integer. Wire protocols carry the string names; the database stores integer IDs as foreign keys. Source: `plane/schema/derived_schema.sql`.

Our additional entity types (introduced by this platform; not in Plane):

| Type | Prefix | Notes |
|---|---|---|
| Sandbox | `sb-` | Our wrapper over a Plane backend. `sandbox.backend_id` references `backend.id`. |
| Snapshot | `snap-` | Our wrapper over an OCI image tag in our registry. |
| Credential binding | `cred-` | Our extension. |
| Secret | `secret-` | Our extension. |
| agentd command | `cmd-` | Internal to `agentd`. |
| Tenant | `tenant-` | Owner; format is application-specific. |

### 2.3 Timestamps and durations

- Timestamps: ISO 8601 UTC with `Z`. Field names end in `_at` (`created_at`).
- Plane uses `chrono::DateTime<Utc>` internally, serialized as RFC 3339.
- Durations: integers, with explicit suffix in the field name. Plane's `BackendMetricsMessage` uses **nanoseconds** for CPU times (`cpu_used`, `sys_cpu`) and **bytes** for memory. Our extensions use `_ms`, `_seconds`, `_usec` consistently.

### 2.4 Error envelopes

There are **two different error formats** in the system, and you must use the right one per route.

#### 2.4.1 Plane error envelope (used by inherited routes under `/ctrl/*`)

Source: `common/src/protocol.rs` (`ApiError`, `ApiErrorKind`). Verified shape:

```ts
type PlaneApiError = {
  id: string;         // random per-error id for log correlation
  message: string;    // human-readable
  kind: PlaneApiErrorKind;
}

type PlaneApiErrorKind =
  | "FailedToAcquireKey"
  | "KeyUnheldNoSpawnConfig"
  | "KeyHeldUnhealthy"
  | "KeyHeld"
  | "NoDroneAvailable"
  | "FailedToRemoveKey"
  | "DatabaseError"
  | "NoClusterProvided"
  | "NotFound"
  | "InvalidClusterName"
  | "Other";
```

Note: Plane returns this **not wrapped** — the JSON body is the bare `ApiError`, not `{"error": ApiError}`.

#### 2.4.2 Our error envelope (used by `/pub/v1/*` routes we introduce)

```ts
type OurError = {
  error: {
    code: string;        // stable machine-readable, SCREAMING_SNAKE
    message: string;
    details?: object;
    request_id: string;
  }
}
```

Standard codes:

| HTTP | `code` | When |
|---|---|---|
| 400 | `INVALID_REQUEST` | Schema validation failed; `details` lists fields. |
| 401 | `UNAUTHENTICATED` | No or invalid auth credential. |
| 403 | `PERMISSION_DENIED` | Authenticated but unauthorized. |
| 404 | `NOT_FOUND` | Resource doesn't exist or not visible to tenant. |
| 409 | `CONFLICT` | Sandbox in wrong state for the operation. |
| 409 | `KEY_HELD` | Mirrors Plane's `KeyHeld`. |
| 429 | `RATE_LIMITED` | Quota or burst limit; `Retry-After` header set. |
| 500 | `INTERNAL` | Bug. Alert. |
| 503 | `NO_DRONE_AVAILABLE` | Mirrors Plane's `NoDroneAvailable`. |
| 503 | `UNAVAILABLE` | Downstream dependency failed; retryable. |
| 504 | `DEADLINE_EXCEEDED` | Operation exceeded its deadline. |

When our `/pub/v1/*` handler delegates to a `/ctrl/connect` call and Plane returns a `PlaneApiError`, our handler maps the kind to our `code` and re-wraps:

| Plane `kind` | Our `code` | HTTP |
|---|---|---|
| `FailedToAcquireKey`, `KeyHeld`, `KeyHeldUnhealthy`, `KeyUnheldNoSpawnConfig` | `KEY_HELD` | 409 |
| `NoDroneAvailable` | `NO_DRONE_AVAILABLE` | 503 |
| `DatabaseError`, `Other` | `INTERNAL` | 500 |
| `NoClusterProvided`, `InvalidClusterName` | `INVALID_REQUEST` | 400 |
| `NotFound` | `NOT_FOUND` | 404 |
| `FailedToRemoveKey` | `INTERNAL` | 500 |

### 2.5 Authentication

- **SDK → Controller (`/pub/v1/*`)** — `Authorization: Bearer <token>` (OIDC access token or long-lived API key). Validation by a front-door reverse proxy that injects `X-Auth-Tenant` and `X-Auth-User` headers. Controller trusts those because `/pub/v1/*` is only reachable through the front-door, and `/ctrl/*` is firewalled.
- **Admin → Controller (`/ctrl/*`)** — Same mechanism, admin scope. Plane natively supports an arbitrary `--forward-auth <url>` argument that turns `/ctrl/*` into a forward-auth-protected surface. Source: `plane/src/controller/mod.rs`.
- **End-user → Proxy (data plane)** — The connection `token` in the URL is the bearer (Plane native). The proxy looks the token up via Plane's `RouteInfoRequest`/`Response` flow against the controller. Source: `common/src/protocol.rs` `RouteInfo`.

`agentd` does no authentication; reaching it requires going through the proxy with a valid token, which is the trust boundary.

### 2.6 Versioning

- HTTP API versioned by path: `/pub/v1/`. Plane's own routes (`/ctrl/*`, `/pub/health`, `/pub/b/...`) are unversioned because Plane has stable in-source contracts the platform either matches or forks.
- WebSocket protocols: Plane's `MessageFromDrone`/`MessageToDrone` are versioned implicitly by the Plane release. Our additions live in extension variants that fail gracefully on older Plane releases (see [§5.3](#53-our-extensions-to-messagetodrone)).
- Field additions to existing types: always optional. Removals/renames: require a coordinated major bump.
- Server tolerates unknown fields on inbound messages.

### 2.7 Idempotency

`POST` endpoints in `/pub/v1/*` accept `Idempotency-Key: <opaque>`. Controller caches the response (status + body) for 24 hours; same key + same body returns the cached response. Different body with the same key → `409 CONFLICT`.

Plane's `POST /ctrl/connect` is **already naturally idempotent on the key**: if the key is held by a running backend with a matching `tag`, the same backend is returned without spawning. See [§4.3](#43-postctrlconnect-plane).

### 2.8 Retries

The SDK retries on `5xx`, `429`, and connection-reset errors with exponential backoff (initial 100 ms, multiplier 2, max 30 s, max 5 attempts). It does not retry on `4xx` except `429`.

WebSocket clients reconnect with backoff: 1 s, 2 s, 4 s, 8 s, capped at 30 s. **Plane's design is at-least-once for actions**: after reconnect, the controller re-sends any `backend_action` row whose `acked_at IS NULL` for that drone (verified in `plane/src/database/backend_actions.rs`). Drones must therefore be idempotent on action execution; Plane uses the unique `action_id` to dedupe.

### 2.9 Rate limits

Per-tenant rate limits at the front door. 429 with `Retry-After`. Headers on every `/pub/v1/*` response:

```
X-RateLimit-Limit: 200
X-RateLimit-Remaining: 173
X-RateLimit-Reset: 1715864400
```

---

## 3. Public API — SDK ↔ Controller (our extension)

**Edge:** I-1, I-2. **Base path:** `/pub/v1/`. **Direction:** SDK → Controller. **Auth:** Bearer.

Plane's `/pub/*` is reserved for `/pub/health`, `/pub/b/:backend/status`, and `/pub/b/:backend/status-stream`. We add `/pub/v1/*` under the same root for the SDK contract, using a distinct version segment.

### 3.1 Endpoint summary

| Method | Path | Purpose |
|---|---|---|
| POST | `/pub/v1/sandboxes` | Create a sandbox |
| GET | `/pub/v1/sandboxes` | List sandboxes |
| GET | `/pub/v1/sandboxes/:id` | Fetch one sandbox |
| GET | `/pub/v1/sandboxes/:id/status` | SSE stream of status transitions |
| POST | `/pub/v1/sandboxes/:id/snapshot` | Snapshot a running sandbox |
| POST | `/pub/v1/sandboxes/:id/network-policy` | Update egress policy |
| POST | `/pub/v1/sandboxes/:id/credentials` | Bind a secret for egress injection |
| DELETE | `/pub/v1/sandboxes/:id/credentials/:cred_id` | Remove a binding |
| POST | `/pub/v1/sandboxes/:id/extend` | Extend the sandbox lifetime |
| DELETE | `/pub/v1/sandboxes/:id` | Stop a sandbox |
| GET | `/pub/v1/snapshots/:id` | Fetch snapshot metadata |
| DELETE | `/pub/v1/snapshots/:id` | Delete a snapshot |
| POST | `/pub/v1/secrets` | Create a secret |
| DELETE | `/pub/v1/secrets/:id` | Delete a secret |

### 3.2 `POST /pub/v1/sandboxes`

**Request:**

```ts
type CreateSandboxRequest = {
  runtime: "node22" | "node24" | "python313" | "bun1" | "desktop" | "custom";
  image?: string;            // required if runtime === "custom"
  trust?: "trusted" | "untrusted" | "paranoid";  // default "untrusted"
  arch?: "amd64" | "arm64";  // default: scheduler chooses

  resources?: {
    vcpus?: number;          // 1 or even 2..32; default 2. Memory = vcpus * 2 GB.
  };

  timeout_ms?: number;       // default 300_000 (5 min)

  source?:
    | { type: "git"; url: string; revision?: string; depth?: number }
    | { type: "snapshot"; snapshot_id: string }
    | { type: "image"; image: string };

  ports?: number[];          // up to 4
  env?: Record<string, string>;
  tags?: Record<string, string>;

  persistent?: boolean;      // default false
  initial_network_policy?: NetworkPolicy;  // see §14
  use_static_token?: boolean;  // default false; corresponds to Plane's use_static_token

  // Plane-passthrough: forwarded into the spawn_config's KeyConfig.
  key?: { name: string; namespace?: string; tag?: string };
}
```

Internally the handler builds a Plane `ConnectRequest` (see [§14.8](#148-connectrequest-plane)) with:
- `key`: either the user-provided `key` or a fresh `KeyConfig` with a random name.
- `spawn_config.executable`: serialized `DockerExecutorConfig` constructed from `runtime`, `image`, `env`, and the resolved per-tier overrides.
- `spawn_config.pool`: derived from `trust` per the **mapping below**.
- `spawn_config.lifetime_limit_seconds`, `max_idle_seconds`, `use_static_token`: from the request.

**Trust tier → Plane pool name (normative contract):**

| `trust` value (SDK) | `spawn_config.pool` (Plane wire) | Drone CLI flag at startup | Default OCI runtime on those drones |
|---|---|---|---|
| `"trusted"` | `"trusted"` | `--pool trusted` | `runc` |
| `"untrusted"` (default) | `"untrusted"` | `--pool untrusted` | `runsc` |
| `"paranoid"` | `"paranoid"` | `--pool paranoid` | `kata-runtime` |

These pool names are written verbatim into the `drone.pool` column (Plane's `VARCHAR(255) DEFAULT ''`) when each drone connects. The mapping is therefore enforced at the join of three places: (a) the SDK request validator, (b) the controller's `ConnectRequest` builder, (c) each drone's startup script that sets `--pool`. Changing any one without the others creates routing failures (sandboxes never schedule because no drone matches the requested pool).

Note: Plane's default pool is the empty string `""`. The platform reserves that default pool name as **not in use** — every drone must be started with an explicit `--pool` for one of our three tiers. The controller treats spawn requests whose resolved pool is `""` as misconfiguration.

**Response (HTTP 201):**

```ts
type CreateSandboxResponse = {
  sandbox: Sandbox;          // §14
  connection: {
    token: string;           // BearerToken from Plane's ConnectResponse
    url: string;             // verbatim from Plane: protocol://[subdomain.]cluster/token/
    secret_token?: string;   // present only if use_static_token=true
    status_url: string;      // mirrors Plane's status_url
  };
}
```

**Errors:** see [§2.4.2](#242-our-error-envelope-used-by-pubv1-routes-we-introduce). Plane's `KeyHeld` becomes 409 `KEY_HELD`; `NoDroneAvailable` becomes 503 `NO_DRONE_AVAILABLE`.

### 3.3 `GET /pub/v1/sandboxes/:id`

Returns the current `Sandbox` (§14). 404 if not visible to tenant.

### 3.4 `GET /pub/v1/sandboxes` — List

Query: `tag=k=v` (repeatable), `status` (any `BackendStatus`), `limit` (default 100, max 500), `cursor` (opaque).

```ts
type ListSandboxesResponse = {
  items: Sandbox[];
  next_cursor?: string;
}
```

### 3.5 Sandbox status stream (SSE)

**Endpoint:** `GET /pub/v1/sandboxes/:id/status`. Tenant-filtered passthrough of Plane's `GET /pub/b/:backend/status-stream`. **The wire shape exactly mirrors Plane's stream** — we don't add fields or re-key, because the SDK already knows which sandbox it's streaming (it's in the URL).

`Content-Type: text/event-stream`. Each event:

```
id: ready
data: {"status":"ready","time":"2026-05-16T12:34:56.789Z"}

id: terminating
data: {"status":"terminating","termination_kind":"soft","termination_reason":"external","time":"..."}

: keepalive
```

Notes — verified against `plane/src/controller/backend_state.rs:handle_backend_status_stream`:

- **`id:` is the status string**, not a numeric ID. Values are exactly the lowercase status strings from `BackendStatus` (`scheduled`, `loading`, `starting`, `waiting`, `ready`, `terminating`, `hard-terminating`, `terminated`).
- **No `event:` type is set** — events use the default (empty) event type.
- **`data:` is a `BackendStatusStreamEntry`** (§14.4), NOT a full `BackendStateMessage`. The shape does not include the backend's address; if the client needs the address, it must fetch the backend separately (or wait for `ready` and use the data-plane URL it was issued at create time).
- **`Last-Event-ID` is parsed as a `BackendStatus`**, not a number. On reconnection, the server filters out any events whose `status.as_int()` is `<=` the parsed last status (using the integer ordering in §14.1). If `Last-Event-ID` is missing or unparseable, all events are sent. This means resume semantics are coarse-grained: clients resume at the next status transition past the one they last saw, not at the next event row. Because statuses are forward-only, this is sufficient for normal SDK usage.
- **Keepalive comments** (`: keepalive\n\n`) every 15 s are emitted by our `/pub/v1/sandboxes/:id/status` wrapper; Plane's upstream `/pub/b/:backend/status-stream` does not emit keepalives, so the wrapper inserts them on idle.

Stream ends when `status` reaches `terminated` (last event), then the server closes the connection. Per Plane's implementation, only one event per status value is emitted across the stream's lifetime (no duplicates).

#### 3.5.1 Single-shot status

`GET /pub/v1/sandboxes/:id/status?stream=false` (or the upstream `GET /pub/b/:backend/status`) returns the latest `BackendStatusStreamEntry` as a JSON body, not an SSE stream. Useful for non-streaming clients that poll.

```ts
// Response body (single-shot):
type BackendStatusStreamEntry = {
  status: BackendStatus;
  termination_reason?: TerminationReason;
  termination_kind?: "soft" | "hard";
  exit_error?: boolean;
  time: string;                      // RFC 3339
}
```

See §14.4 for the full type.

### 3.6 `POST /pub/v1/sandboxes/:id/snapshot`

```ts
type SnapshotRequest = {
  name?: string;
  ttl_days?: number;
  tags?: Record<string, string>;
}

type SnapshotResponse = {
  snapshot: Snapshot;
}
```

Synchronous: returns after the snapshot blob is durable in the OCI registry. Idempotent on `Idempotency-Key`.

### 3.7 `POST /pub/v1/sandboxes/:id/network-policy`

```ts
type UpdateNetworkPolicyRequest = NetworkPolicy;
```

**Response (HTTP 200):** the effective `NetworkPolicy`. Propagation deadline: 500 ms p95 from response to in-effect on the egress proxy.

### 3.8 `POST /pub/v1/sandboxes/:id/credentials`

```ts
type CreateCredentialBindingRequest = {
  secret_id: string;
  destination: string;       // domain ("*." allowed) or CIDR
  header_name: string;
  header_format?: string;    // template; default "{value}"
}

type CredentialBinding = {
  id: string;                // cred-...
  sandbox_id: string;
  destination: string;
  header_name: string;
  header_format: string;
  secret_id: string;
  created_at: string;
}
```

### 3.9 `POST /pub/v1/sandboxes/:id/extend`

```ts
type ExtendTimeoutRequest = {
  additional_ms: number;
}
```

Updates `backend.expiration_time` in Postgres. Response is the updated `Sandbox`.

### 3.10 `DELETE /pub/v1/sandboxes/:id`

Query: `force=true` for hard-terminate. Internally calls Plane's `POST /ctrl/b/:backend/soft-terminate` (or `hard-terminate`). Response: 204.

### 3.11 `POST /pub/v1/secrets`

```ts
type CreateSecretRequest = {
  value: string;
  name?: string;
  tags?: Record<string, string>;
}

type CreateSecretResponse = {
  id: string;                // secret-...
  name?: string;
  created_at: string;
  // value never returned again
}
```

Stored as KMS-encrypted ciphertext.

---

## 4. Control API — `/ctrl/*` (Plane + our additions)

**Edge:** I-3. **Auth:** forward-auth admin scope (recommended) or firewalled.

### 4.1 Inherited Plane routes (verified)

Verified against `plane/src/controller/mod.rs` lines 222–245. **This is the complete list of `/ctrl/*` routes in upstream Plane.**

| Method | Path | Handler | Purpose |
|---|---|---|---|
| GET | `/ctrl/status` | `status` | Controller status (`StatusResponse`) |
| GET | `/ctrl/c/:cluster/state` | `handle_cluster_state` | `ClusterState` |
| GET | `/ctrl/c/:cluster/drone-socket` | `handle_drone_socket` | Upgrade to WebSocket (§5) |
| GET | `/ctrl/c/:cluster/proxy-socket` | `handle_proxy_socket` | Upgrade to WebSocket (§6) |
| GET | `/ctrl/dns-socket` | `handle_dns_socket` | DNS server WebSocket |
| POST | `/ctrl/connect` | `handle_connect` | Plane connect request |
| POST | `/ctrl/c/:cluster/d/:drone/drain` | `handle_drain` | Drain a drone |
| POST | `/ctrl/b/:backend/soft-terminate` | `terminate::handle_soft_terminate` | SIGTERM with grace |
| POST | `/ctrl/b/:backend/hard-terminate` | `terminate::handle_hard_terminate` | SIGKILL |
| POST | `/ctrl/b/revoke` | `handle_revoke` | Revoke a connection token |

Public:

| Method | Path | Purpose |
|---|---|---|
| GET | `/pub/health` | Liveness; returns `{"status":"OK"}` |
| GET | `/pub/b/:backend/status` | Latest `BackendStatus` |
| GET | `/pub/b/:backend/status-stream` | SSE stream of `BackendStateMessage` |

### 4.2 `GET /ctrl/c/:cluster/state` — `ClusterState`

Returns the verified shape (`common/src/types/mod.rs:498`):

```ts
type ClusterState = {
  drones: DroneState[];
  proxies: NodeState[];
}

type DroneState = {
  ready: boolean;
  draining: boolean;
  last_heartbeat_age: number;  // serialized as seconds (custom serde)
  backend_count: number;
  node: NodeState;
}

type NodeState = {
  name: string;                // AnyNodeName
  plane_version: string;
  plane_hash: string;
  controller: string;          // ControllerName
  controller_heartbeat_age: number;  // serialized as seconds
}
```

### 4.3 `POST /ctrl/connect` (Plane)

The heart of the Plane API. Request: `ConnectRequest` (§14.8). Response: `ConnectResponse` (§14.9). Errors return `ApiError` (§2.4.1).

`spawn_config.executable` is `serde_json::Value` (untyped on the wire); the drone deserializes it into a `DockerExecutorConfig` (§14.5). This is the extension point for non-Docker executors in future, but for our platform we always send a `DockerExecutorConfig`.

The handler:

1. Looks up the key (if `key` is set). If a backend is running with this key and the tag matches, returns that backend's `ConnectResponse` with `spawned: false`.
2. Otherwise, if `spawn_config` is set, picks a drone matching the cluster (and pool), acquires the key (writes `backend_key` row with a new `fencing_token`), creates the `backend` row with status `scheduled`, writes a `backend_action` row of kind `Spawn`, emits a notification, returns `ConnectResponse` with `spawned: true`.
3. If `spawn_config` is absent and no running backend matches, returns `KeyUnheldNoSpawnConfig`.

URL format constructed by `ConnectResponse::new`:

```
{protocol}://{[subdomain.]cluster}/{token}/
```

`protocol` is `https` if the cluster name lacks a port; `http` if it has one (dev cluster). Verified at `common/src/types/mod.rs:407`.

### 4.4 Our extensions to `/ctrl/*`

These are routes we add to a forked controller. They follow the same auth model and error format as Plane's `/ctrl/*` (the `PlaneApiError` envelope from §2.4.1), to keep the surface uniform for admins.

| Method | Path | Purpose |
|---|---|---|
| POST | `/ctrl/snapshots` | Drone registers a snapshot after `docker push` |
| GET | `/ctrl/snapshots/:id` | Resolve snapshot metadata |
| DELETE | `/ctrl/snapshots/:id` | Delete a snapshot |
| POST | `/ctrl/policies/:sandbox_id` | Update network policy (also reachable via `/pub/v1/sandboxes/:id/network-policy`) |
| POST | `/ctrl/credentials` | Create a credential binding |
| DELETE | `/ctrl/credentials/:id` | Revoke a credential binding |
| GET | `/ctrl/c/:cluster/d/:drone/metrics` | Per-drone metering summary |

#### 4.4.1 `POST /ctrl/snapshots`

Called by drones immediately after a successful `docker push`.

```ts
type RegisterSnapshotRequest = {
  sandbox_id: string;
  image_ref: string;         // e.g., "registry.local:5000/snap:snap-7f3d1a"
  digest: string;            // "sha256:..."
  base_image: string;
  size_bytes: number;
  arch: "amd64" | "arm64";
  runtime: string;
  ttl_days?: number;
  name?: string;
}

type RegisterSnapshotResponse = {
  snapshot: Snapshot;
}
```

---

## 5. Drone ↔ Controller WebSocket (Plane + our additions)

**Edge:** I-4. **URL:** `ws://<controller>/ctrl/c/:cluster/drone-socket`. **Direction:** drone dials; bidirectional thereafter.

**This entire section is verified against `common/src/protocol.rs`.** Plane's enums are listed first; our extensions to those enums are clearly marked.

### 5.1 Channel pattern

Plane uses paired enums `MessageFromDrone` / `MessageToDrone` connected by a `ChannelMessage` trait whose `Reply` associated type points at the other side. The serialization is JSON, one message per WebSocket text frame.

```rust
// common/src/protocol.rs
impl ChannelMessage for MessageFromDrone { type Reply = MessageToDrone; }
impl ChannelMessage for MessageToDrone { type Reply = MessageFromDrone; }
```

There is **no handshake or "hello" message**. The connection is established by the HTTP upgrade itself; identity is via path params (the drone's name is part of the URL path: `/ctrl/c/:cluster/drone-socket` — the controller looks up the drone by IP + cluster from the upgrade context).

### 5.2 `MessageFromDrone` (drone → controller) — Plane

Verified shape (`common/src/protocol.rs`):

```ts
type MessageFromDrone =
  | { Heartbeat: Heartbeat }
  | { BackendEvent: BackendStateMessage }
  | { BackendMetrics: BackendMetricsMessage }
  | { AckAction: { action_id: string } }    // BackendActionName
  | { RenewKey: RenewKeyRequest };

type Heartbeat = {
  local_time: string;                       // RFC 3339 timestamp from the drone
};

type BackendStateMessage = {
  event_id: number;                         // BackendEventId (i64), monotonic per drone
  backend_id: string;                       // BackendName
  state: BackendState;                      // tagged union, see §14.2
  timestamp: string;                        // RFC 3339
};

type BackendMetricsMessage = {
  backend_id: string;
  mem_used: number;                         // bytes; excludes inactive file cache
  mem_total: number;                        // bytes; kernel + page cache
  mem_active: number;
  mem_inactive: number;
  mem_unevictable: number;
  mem_limit: number;
  cpu_used: number;                         // NANOSECONDS since last message
  sys_cpu: number;                          // NANOSECONDS, total system CPU
};

type RenewKeyRequest = {
  backend: string;                          // BackendName
  local_time: string;                       // drone's local time
};
```

**Critical detail:** `BackendMetrics.cpu_used` and `sys_cpu` are in **nanoseconds**, not microseconds. This is different from our own metering aggregation (§13). The drone may need to convert when populating our `usage_record` table.

#### Heartbeat cadence

Plane's heartbeat interval is **7 seconds** (`HEARTBEAT_INTERVAL` in `plane/src/heartbeat_consts.rs`). The controller treats a drone as unhealthy after **45 seconds** without a heartbeat (`UNHEALTHY_SECONDS`), soft-terminates lock-holding backends after **60 seconds** (`SOFT_TERMINATE_DEADLINE_SECONDS`), hard-terminates after **90 seconds**, and considers the drone lost (removing its locks) after **120 seconds** (`ASSUME_LOST_SECONDS`).

### 5.3 `MessageToDrone` (controller → drone) — Plane + our extensions

Verified shape:

```ts
type MessageToDrone =
  | { Action: BackendActionMessage }
  | { AckEvent: { event_id: number } }      // ack receipt of a BackendStateMessage
  | { RenewKeyResponse: RenewKeyResponse }
  // ---- our extensions (require forked controller AND forked drone) ----
  | { PushNetworkPolicy: PushNetworkPolicyMessage }   // OURS
  | { PushCredential: PushCredentialMessage }         // OURS
  | { RevokeCredential: { binding_id: string } }      // OURS
  | { Snapshot: SnapshotCommand }                     // OURS
  ;

type BackendActionMessage = {
  action_id: string;                        // BackendActionName, used to ack
  backend_id: string;
  drone_id: number;                         // NodeId (i32)
  action: BackendAction;
}

type BackendAction =
  | { Spawn: { executable: object; key: AcquiredKey; static_token: string | null } }
  | { Terminate: { kind: "soft" | "hard"; reason: TerminationReason } };

type AcquiredKey = {
  key: KeyConfig;
  deadlines: {
    renew_at: string;
    soft_terminate_at: string;
    hard_terminate_at: string;
  };
  token: number;                            // fencing token (i64)
}

type RenewKeyResponse = {
  backend: string;
  deadlines: KeyDeadlines | null;           // null = renewal denied
};

// Our extensions:

type PushNetworkPolicyMessage = {
  backend_id: string;
  policy: NetworkPolicy;                    // §14.12
  version: number;                          // monotonic per backend
};

type PushCredentialMessage = {
  backend_id: string;
  binding: {
    id: string;                             // cred-...
    destination: string;
    header_name: string;
    header_format: string;
    value: string;                          // plaintext secret; sent on this WS only
  };
};

type SnapshotCommand = {
  action_id: string;                        // for ack matching, follows BackendActionName pattern
  backend_id: string;
  ttl_days?: number;
  name?: string;
};
```

**Action durability:** Plane writes every `BackendActionMessage` to the `backend_action` table before emitting the notification. The drone acknowledges with `AckAction { action_id }`, which sets `acked_at`. On reconnect, the controller queries pending (unacked) actions for that drone and re-sends. **The drone must therefore handle duplicate deliveries idempotently using `action_id`.** Source: `plane/src/database/backend_actions.rs`.

**Our extensions should follow the same durability model.** We add `policy_push` and `credential_push` rows to similar tables (§15), use the same ack-on-WS mechanism via a separate `AckExtensionAction` variant of `MessageFromDrone` (also our extension):

```ts
// OUR extension to MessageFromDrone:
type MessageFromDroneExtension =
  | { AckExtensionAction: { action_id: string } };
```

In our forked Plane, we add this variant to the existing `MessageFromDrone` enum so the dispatch is unified.

### 5.4 Sequence: spawn

```
Drone                                Controller
  │                                      │
  │   (SDK calls /pub/v1/sandboxes)      │
  │                                      │
  │   (controller writes backend_action  │
  │    row with action_id="ak-..."       │
  │    and emits notification keyed by   │
  │    drone_id)                         │
  │                                      │
  │   ◀── MessageToDrone::Action(        │
  │         BackendActionMessage {       │
  │           action_id: "ak-7f3d1a",    │
  │           backend_id: "be-...",      │
  │           drone_id: 42,              │
  │           action: Spawn { ... }      │
  │         }                            │
  │       )                              │
  │                                      │
  │   docker pull, create, start...      │
  │                                      │
  │   ── MessageFromDrone::BackendEvent  │
  │      { event_id: 100,                │
  │        backend_id: "be-...",         │
  │        state: { status: "loading" }, │
  │        timestamp: "..." }            │
  │      ──▶                             │
  │                                      │
  │                                      │  (controller appends to backend_state,
  │                                      │   updates backend.last_status,
  │                                      │   emits backend_state notification)
  │   ◀── MessageToDrone::AckEvent       │
  │       { event_id: 100 }              │
  │                                      │
  │   ── MessageFromDrone::BackendEvent  │
  │      { event_id: 101,                │
  │        state: { status: "starting" } │
  │      }                               │
  │      ──▶                             │
  │   ◀── MessageToDrone::AckEvent { 101}│
  │                                      │
  │   ...                                │
  │                                      │
  │   ── MessageFromDrone::BackendEvent  │
  │      { state: { status: "ready",     │
  │        address: "10.0.0.5:3000" } }  │
  │      ──▶                             │
  │                                      │
  │   ── MessageFromDrone::AckAction     │
  │      { action_id: "ak-7f3d1a" }      │
  │      ──▶                             │
  │                                      │
  │                                      │  (controller sets backend_action.acked_at)
```

### 5.5 Sequence: terminate

```
Drone                                Controller
  │                                      │
  │   ◀── MessageToDrone::Action(        │
  │         { action_id: "ak-...",       │
  │           backend_id: "be-...",      │
  │           action: Terminate {        │
  │             kind: "soft",            │
  │             reason: "external"       │
  │           } } )                      │
  │                                      │
  │   docker stop -t <grace>             │
  │                                      │
  │   ── BackendEvent                    │
  │      { state: { status: "terminating",│
  │        last_status: "ready",         │
  │        termination: "soft",          │
  │        reason: "external" } }        │
  │   ── BackendEvent                    │
  │      { state: { status: "terminated",│
  │        last_status: "ready",         │
  │        termination: "soft",          │
  │        reason: "external",           │
  │        exit_code: 0 } }              │
  │   ── AckAction { action_id }         │
```

### 5.6 Reconnection

On WebSocket close, the drone reconnects with backoff (§2.8). The controller's responsibilities on a fresh connection:

1. Update `node.controller` to the new controller's id.
2. Query unacked `backend_action` rows for this drone (`WHERE drone_id=$1 AND acked_at IS NULL ORDER BY created_at`).
3. Resend each as a `MessageToDrone::Action`.
4. Wait for `AckAction` from each before considering re-sync complete.

Our extension actions (snapshot, push_network_policy, push_credential) follow the same pattern from their respective tables.

The drone's responsibilities on reconnect:

1. Resume sending `BackendEvent` for each running backend, starting from a fresh `event_id` (monotonic per drone-controller session — Plane uses a per-session counter, not a database-wide one).
2. Re-send `BackendMetrics` on the normal cadence.

**Important:** Plane's `event_id` is per-drone, monotonically increasing across a drone's lifetime. The controller stores them in `backend_state` table.

---

## 6. Proxy ↔ Controller WebSocket (Plane + our additions)

**Edge:** I-5. **URL:** `ws://<controller>/ctrl/c/:cluster/proxy-socket`.

**Verified against `common/src/protocol.rs`.** This is fundamentally a **request/response pull model**, not a push model.

### 6.1 `MessageFromProxy` — Plane

```ts
type MessageFromProxy =
  | { RouteInfoRequest: { token: string } }      // ask: "what backend does this token go to?"
  | { KeepAlive: string }                         // BackendName: "this backend is active"
  | { CertManagerRequest: CertManagerRequest };

type CertManagerRequest =
  | "CertLeaseRequest"
  | { SetTxtRecord: { txt_value: string } }
  | "ReleaseCertLease";
```

The proxy pulls route info on demand for each unknown token, rather than holding a synced table of all routes. This is the **opposite** of what I had originally — there's no `route_table_update`. The proxy maintains its own cache of resolved `RouteInfo` keyed by token; on cache miss, it asks the controller.

`KeepAlive(BackendName)` signals that the proxy currently sees traffic for that backend. The controller's idle-detection logic reads `backend.last_keepalive` to decide whether to terminate idle backends.

### 6.2 `MessageToProxy` — Plane + our additions

```ts
type MessageToProxy =
  | { RouteInfoResponse: RouteInfoResponse }
  | { CertManagerResponse: CertManagerResponse }
  | { BackendRemoved: { backend: string } }     // BackendName: "stop routing to this"
  // ---- our extensions ----
  | { SubdomainPortRouteUpdate: SubdomainPortRouteUpdate }  // OURS
  ;

type RouteInfoResponse = {
  token: string;
  route_info: RouteInfo | null;                 // null = unknown token
};

type RouteInfo = {
  backend_id: string;                           // BackendName
  address: string;                              // BackendAddr: "ip:port"
  secret_token: string;                         // SecretToken
  cluster: string;                              // ClusterName
  user: string | null;
  user_data: object | null;
  subdomain: string | null;
};

type CertManagerResponse =
  | { CertLeaseResponse: { accepted: boolean } }
  | { SetTxtRecordResponse: { accepted: boolean } };
```

`RouteInfo` carries the resolved backend address and metadata. The proxy uses `address` to dial, and sets `X-Verified-Username: <user>` and `X-Verified-Auth: <user_data as JSON>` headers when forwarding (Plane's documented auth-forwarding convention).

#### Our extension: subdomain-port routing

For URLs like `https://sb-abc-3000.sandboxes.example.com/...` (our spec §3.3), the proxy needs a different lookup. We add a single message:

```ts
type SubdomainPortRouteUpdate = {
  sandbox_id_prefix: string;        // e.g., "sb-abc"
  port: number;                     // 3000
  address: string;                  // "ip:port" of the container port
  expires_at: string;
};
```

Pushed when a sandbox is created with `ports` set, removed when the sandbox terminates. The proxy maintains a separate cache keyed by `(sandbox_id_prefix, port)`.

We could alternately implement this purely via the existing `subdomain` field on `RouteInfo`, but the keyed-by-port lookup matches our SDK shape more cleanly.

### 6.3 Cert manager flow

Plane uses ACME DNS-01. The flow is multi-proxy-aware:

```
Proxy                                 Controller
  │                                      │
  │   ── CertLeaseRequest ──▶            │
  │                                      │  (writes lease to acme_txt_entries
  │                                      │   for cluster; only one proxy holds it)
  │   ◀── CertLeaseResponse { accepted } │
  │                                      │
  │   (if accepted, ACME challenge)      │
  │   ── SetTxtRecord { txt_value } ──▶  │
  │                                      │  (updates acme_txt_entries.txt_value;
  │                                      │   DNS server reads from here)
  │   ◀── SetTxtRecordResponse {accepted}│
  │                                      │
  │   (ACME completes; proxy has cert)   │
  │   ── ReleaseCertLease ──▶            │
```

`acme_txt_entries` table is shared; the `leased_by` column is the proxy's `NodeId` and acts as the leasing primitive.

---

## 7. Drone ↔ Egress Proxy (our extension)

**Edge:** I-6. **Origin:** ours (Plane has no egress proxy concept). **Transport:** Envoy xDS over UDS.

### 7.1 xDS resources

The drone embeds a small xDS server (using one of: `envoy-control-plane-go`, `tonic` for Rust). Envoy connects over UDS at `/var/run/sandbox-platform/xds.sock`.

| xDS resource | What we publish |
|---|---|
| LDS (listeners) | One listener per L4/L7 combination we intercept: HTTP (8080), TLS (8443), DNS (8053 TCP/UDP). All bind `127.0.0.0/8` to use the per-sandbox loopback trick. |
| RDS (routes) | One virtual host per `sandbox_id`; route rules implement the policy. |
| CDS (clusters) | One Envoy cluster per `(destination, sandbox_id)` pair for injected destinations; one shared "passthrough_dns" cluster for the upstream resolver. |
| EDS (endpoints) | DNS-resolved IPs for upstream destinations. Updated on TTL. |

### 7.2 Per-sandbox identification

Sandboxes share the host's egress proxy. To attribute an inbound connection to a sandbox, the drone assigns each sandbox a host-side loopback IP from `127.{a}.{b}.{c}` (e.g., `127.20.7.5`), installs iptables DNAT rules in the sandbox's netns redirecting outbound to this loopback IP, and configures Envoy listener filter chains to match destination IP → sandbox_id.

Capacity: with three octets, we have ~16M unique sandboxes per host (vastly more than any host's capacity). If this scheme is found unsuitable, fall back to transparent proxy mode using `SO_ORIGINAL_DST` and per-sandbox veth IPs.

### 7.3 Access logs

Envoy emits gRPC ALS to a collector inside the drone:

```ts
type EgressAccessLog = {
  at: string;                        // RFC 3339
  sandbox_id: string;
  destination_host: string;          // SNI or Host header value
  destination_port: number;
  destination_ip: string;
  protocol: "http" | "tls" | "dns";
  action: "allow" | "allow_inject" | "deny";
  policy_rule_index?: number;        // which rule matched
  bytes_sent: number;
  bytes_received: number;
  duration_ms: number;
  tls_sni?: string;
  http_host?: string;
  http_path?: string;
  http_status?: number;
}
```

Drone batches these and forwards to the controller via a new variant on `MessageFromDrone` (our extension):

```ts
// our extension on MessageFromDrone:
type MessageFromDroneEgressExtension =
  | { EgressLogBatch: { logs: EgressAccessLog[] } };
```

---

## 8. Drone ↔ Docker (Docker Engine API)

**Edge:** I-7. **Spec:** [Docker Engine API v1.43+](https://docs.docker.com/reference/api/engine/). **Transport:** Unix socket `/var/run/docker.sock`.

### 8.1 Calls we make

| Operation | Method + endpoint | Notes |
|---|---|---|
| Pull image | `POST /images/create?fromImage=<ref>&tag=<tag>` | Auth via `X-Registry-Auth` header for private registries |
| Create container | `POST /containers/create?name=<id>` | Body includes `HostConfig` (§8.2) |
| Start container | `POST /containers/<id>/start` | |
| Stop container | `POST /containers/<id>/stop?t=<grace>` | SIGTERM then SIGKILL |
| Kill container | `POST /containers/<id>/kill?signal=SIGKILL` | |
| Inspect | `GET /containers/<id>/json` | For state and exit code |
| Pause | `POST /containers/<id>/pause` | For snapshot |
| Unpause | `POST /containers/<id>/unpause` | |
| Commit | `POST /commit?container=<id>&repo=<r>&tag=<t>&pause=false` | Container should already be paused |
| Push image | `POST /images/<ref>/push` | Stream of progress JSON |
| Tag image | `POST /images/<id>/tag?repo=<r>&tag=<t>` | |
| Remove image | `DELETE /images/<ref>?force=true` | Cleanup after push |
| Stats | `GET /containers/<id>/stats?stream=false` | Fallback when cgroup files unavailable |
| Logs | `GET /containers/<id>/logs?stdout=1&stderr=1&follow=1&since=<unix>` | For log streaming |
| Wait | `POST /containers/<id>/wait` | Block until container exits |

### 8.2 `HostConfig` at create time

```json
{
  "Runtime": "runsc",
  "Memory": 2147483648,
  "NanoCpus": 2000000000,
  "PidsLimit": 4096,
  "NetworkMode": "bridge",
  "PortBindings": { "3000/tcp": [{ "HostIp": "0.0.0.0", "HostPort": "" }] },
  "RestartPolicy": { "Name": "no" },
  "AutoRemove": false,
  "ReadonlyRootfs": false,
  "Privileged": false,
  "SecurityOpt": ["no-new-privileges"],
  "CapDrop": ["ALL"],
  "CapAdd": ["CHOWN", "SETUID", "SETGID", "NET_BIND_SERVICE"]
}
```

Notes:
- `Runtime` is the OCI runtime name (matches the drone's `--docker-runtime` flag's value for default; this field overrides it per-container).
- `NanoCpus`: nanoseconds of CPU per second; 1 vCPU = 1_000_000_000. We compute as `vcpus * 1_000_000_000`.
- Empty `HostPort` lets Docker assign a port; the drone reads the assigned port back from `/containers/:id/json`.
- `ReadonlyRootfs` is `false` because the writable filesystem is the point.

### 8.3 Container labels

Every sandbox container carries labels for ops correlation:

```
sandbox-platform.role=sandbox
sandbox-platform.sandbox-id=<id>
sandbox-platform.backend-id=<plane-backend-id>
sandbox-platform.tenant=<tenant>
sandbox-platform.pool=<pool>
sandbox-platform.created-at=<rfc3339>
```

---

## 9. Data plane — Proxy ↔ agentd

**Edges:** I-8, I-9. **Transport:** HTTPS (or HTTP in dev), WebSocket for streams.

### 9.1 URL shapes

**Token path (Plane native):** `https://<cluster>/<token>/<path>`. The proxy:
1. Strips the first path segment as the token.
2. Looks up `RouteInfo` (cached or via `RouteInfoRequest` to the controller).
3. Reverse-proxies to `route_info.address` (the `drone_ip:container_port`).

**Subdomain port (our extension):** `https://sb-<sid-prefix>-<port>.<base-domain>/<path>`. The proxy:
1. Extracts `(sid_prefix, port)` from the subdomain.
2. Looks up the address via the subdomain-port cache (§6.2 extension).
3. Reverse-proxies.

### 9.2 Headers the proxy sets on forwarded requests

Plane's documented convention (token-path mode): when `RouteInfo.user` is non-null:

```
X-Verified-Username: <user>
X-Verified-Auth: <user_data as JSON>
```

We additionally add (our extension):

```
X-Sandbox-Id: sb-<id>
X-Forwarded-For: <original client>
X-Forwarded-Proto: https
```

`agentd` trusts these headers because no other path reaches port 8080.

### 9.3 WebSocket upgrade

WS is byte-piped through the proxy: client opens `wss://<cluster>/<token>/v1/exec/<cmd>/logs`, proxy upgrades and forwards bytes. No application-level inspection.

---

## 10. agentd HTTP API (our extension)

**Edge:** I-9. **Port:** 8080 (inside the sandbox container). **Base path:** `/v1/`.

### 10.1 Endpoint summary

| Method | Path | Purpose |
|---|---|---|
| GET | `/health` | Readiness probe; used by drone |
| GET | `/info` | Runtime info |
| POST | `/v1/exec` | Run a command |
| GET | `/v1/exec/:cmd_id` | Poll status |
| GET (WS) | `/v1/exec/:cmd_id/stdout` | stdout stream |
| GET (WS) | `/v1/exec/:cmd_id/stderr` | stderr stream |
| GET (WS) | `/v1/exec/:cmd_id/logs` | combined stream |
| POST | `/v1/exec/:cmd_id/stdin` | Write to stdin |
| POST | `/v1/exec/:cmd_id/signal` | Send a signal |
| GET | `/v1/fs/read` | Read file |
| PUT | `/v1/fs/write` | Write file (multipart) |
| POST | `/v1/fs/mkdir` | Create directory |
| DELETE | `/v1/fs/rm` | Remove |
| GET | `/v1/fs/list` | List directory |
| POST | `/v1/fs/move` | Move/rename |
| GET (WS) | `/v1/fs/watch` | inotify stream |
| GET | `/v1/ports` | List listening sockets |
| GET | `/v1/processes` | ps-style list |
| POST | `/v1/computer` | Computer-use action (desktop variant only) |

### 10.2 `GET /health`

200 once initialized: `{"status":"ready","version":"<agentd-version>"}`. Before init: 503 with `{"status":"starting"}`.

Drone uses this for readiness. The drone considers a backend ready (Plane state transition `Waiting → Ready` with `address`) when `/health` returns 200.

### 10.3 `POST /v1/exec`

```ts
type ExecRequest = {
  cmd: string;
  args?: string[];
  env?: Record<string, string>;
  cwd?: string;                       // default "/workspace"
  stdin?: string;                     // initial stdin (base64)
  timeout_ms?: number;
  detached?: boolean;                 // default false
  pty?: boolean;
  pty_size?: { rows: number; cols: number };
  user?: string;
}

// Sync response (detached=false), HTTP 200:
type ExecSyncResponse = {
  cmd_id: string;
  exit_code: number;                  // signal: 128 + signum
  stdout: string;                     // entire output, UTF-8
  stderr: string;
  duration_ms: number;
  truncated?: boolean;                // true if output > 10 MB
}

// Detached response (detached=true), HTTP 202:
type ExecDetachedResponse = {
  cmd_id: string;
  status: "running";
}
```

### 10.4 WebSocket log streaming

Frames are JSON:

```ts
type LogFrame =
  | { stream: "stdout"; data: string }   // base64 of raw bytes
  | { stream: "stderr"; data: string }
  | { stream: "exit"; code: number; signal?: number }
  | { stream: "error"; code: "BACKPRESSURE" | "TIMEOUT" };
```

`data` is base64-encoded raw bytes because user-program output may not be valid UTF-8. Buffering up to 1 MB per connection; further writes drop with `BACKPRESSURE` frame and close.

### 10.5 `POST /v1/exec/:cmd_id/signal`

```ts
type SignalRequest = {
  signal: "TERM" | "KILL" | "INT" | "HUP" | "USR1" | "USR2";
}
```

Response 204.

### 10.6 Filesystem endpoints

#### `GET /v1/fs/read?path=<abs>&offset=<n>&length=<n>`

Returns raw bytes. `Content-Type: application/octet-stream`. Optional `offset` and `length` for partial reads. Path must be inside the configured root (default `/workspace`).

#### `PUT /v1/fs/write`

Multipart form-data:
- `path` (text, required)
- `mode` (text, optional, octal as string, default `"0644"`)
- `file` (binary, the content)

201 no body. Atomic via `open(O_TMPFILE)` + `linkat`, or `rename` from a sibling tmpfile.

#### `POST /v1/fs/mkdir`

```ts
type MkdirRequest = { path: string; mode?: string; recursive?: boolean };
```

#### `DELETE /v1/fs/rm?path=<abs>&recursive=true|false`

#### `GET /v1/fs/list?path=<abs>`

```ts
type ListResponse = {
  path: string;
  entries: Array<{
    name: string;
    type: "file" | "dir" | "symlink" | "other";
    size?: number;
    mode: string;
    modified_at: string;
  }>;
}
```

#### `GET /v1/fs/watch?path=<abs>&recursive=true` (WS)

```ts
type FsEvent = {
  event: "create" | "modify" | "delete" | "rename" | "attrib";
  path: string;
  is_dir: boolean;
  at: string;
}
```

### 10.7 `GET /v1/ports`

```ts
type PortsResponse = {
  listening: Array<{
    port: number;
    protocol: "tcp" | "udp";
    process_pid?: number;
    process_name?: string;
  }>;
}
```

Derived from `/proc/<pid>/net/{tcp,tcp6,udp,udp6}` inside the container's netns.

### 10.8 `POST /v1/computer` (desktop variant)

Maps 1:1 to Anthropic's computer-use schema. Verified against `anthropics/anthropic-quickstarts/blob/main/computer-use-demo/computer_use_demo/tools/computer.py`.

```ts
type ComputerActionRequest = {
  action:
    | "screenshot" | "cursor_position"
    | "mouse_move" | "left_click" | "right_click" | "middle_click"
    | "double_click" | "triple_click" | "left_click_drag"
    | "left_mouse_down" | "left_mouse_up"
    | "key" | "type" | "hold_key"
    | "scroll" | "wait" | "zoom";
  coordinate?: [number, number];
  start_coordinate?: [number, number];
  text?: string;
  duration?: number;
  region?: [number, number, number, number];
  scroll_direction?: "up" | "down" | "left" | "right";
  scroll_amount?: number;
}

type ComputerActionResponse = {
  output: ComputerOutput;
}

type ComputerOutput =
  | { type: "image"; base64: string; media_type: "image/png" }
  | { type: "text"; text: string }
  | { type: "ok" };
```

---

## 11. Egress proxy ↔ internet (our extension)

**Edges:** I-10, I-11. **Origin:** ours.

### 11.1 Iptables redirect (I-10)

Per [§7.2](#72-per-sandbox-identification), the drone installs in the container's netns:

```bash
# Conceptual; exact rules use the per-sandbox loopback IP.
SBOX_LO="127.20.7.5"   # assigned per sandbox
iptables -t nat -A OUTPUT -p tcp --dport 80  -j DNAT --to ${SBOX_LO}:8080
iptables -t nat -A OUTPUT -p tcp --dport 443 -j DNAT --to ${SBOX_LO}:8443
iptables -t nat -A OUTPUT -p udp --dport 53  -j DNAT --to ${SBOX_LO}:8053
iptables -t nat -A OUTPUT -p tcp --dport 53  -j DNAT --to ${SBOX_LO}:8053
iptables -A OUTPUT -j REJECT --reject-with icmp-net-prohibited
```

The egress proxy listens on `127.0.0.0/8:{8080,8443,8053}`, identifies the sandbox by the destination IP it sees, and applies the policy.

### 11.2 Egress → external (I-11)

Two modes:

- **Passthrough TLS:** SNI inspected at handshake (or Host header for HTTP); on `allow`, bytes are piped. End-to-end TLS verification happens between sandbox and real upstream.
- **MITM TLS:** for destinations with `inject` set. Egress terminates inbound TLS using a per-sandbox internal CA. `agentd` installs the internal CA root into the container's trust store on first boot (writes to `/usr/local/share/ca-certificates/sandbox-platform.crt` and runs `update-ca-certificates`). Egress opens fresh TLS to upstream, injects the configured header, pipes bytes.

Header injection:

```
Authorization: Bearer <decrypted secret value>
# header_format from the binding, with {value} replaced by the secret
```

Any inbound value for the same header is stripped before forwarding.

### 11.3 DNS

DNS goes through the proxy. Parsing: extract the question's QNAME, match against policy. Denied → NXDOMAIN with TTL 0. Allowed → forward to a configured upstream resolver, return response. Cache: positive responses for RFC TTL, negative for 60 s.

---

## 12. Storage contracts

### 12.1 Postgres

**Edge:** I-12. **Driver:** sqlx for the (forked) controller. **TLS:** required in production.

Schema (Plane tables + our extensions) is in [§15](#15-postgres-schema). Migrations: forward-only, applied via `sqlx migrate run` on controller startup. Our migrations are appended to Plane's; numbering uses our own date prefix (e.g., `20260601000000_add-sandbox-platform.sql`).

### 12.2 OCI Registry

**Edge:** I-14. **API:** [Docker Registry HTTP API V2](https://distribution.github.io/distribution/spec/api/).

Image naming:

| Kind | Repository | Tag |
|---|---|---|
| Sandbox runtime base | `sandbox-runtime/<name>` | `<version>` (e.g., `node24-v1.2.3`) |
| Snapshot | `snap` | `<snapshot-id>` (e.g., `snap-7f3d1a`) |

Snapshots deleted by tag deletion + registry GC sweep.

### 12.3 KMS / secret storage

**Edge:** I-15. Envelope encryption:

1. **Create:** generate a 256-bit data key via KMS `GenerateDataKey`; encrypt the plaintext locally with AES-256-GCM (random nonce); store `{ciphertext, wrapped_data_key, nonce, kms_key_id}` in the `secret` table.
2. **Use:** fetch row, unwrap data key via KMS `Decrypt`, decrypt plaintext in controller memory, push plaintext to drone over the WSS channel as a `PushCredential` message.

Plaintext never persists outside the controller's memory. The controller-to-drone WSS is the only place plaintext traverses the network; mTLS on that channel is required in production.

---

## 13. Internal coordination — Postgres LISTEN/NOTIFY

**Edge:** I-13.

Plane implements pub/sub on top of the `event` table plus Postgres LISTEN/NOTIFY. The mechanism (verified at `plane/src/database/subscribe.rs`):

1. Publisher executes a CTE that inserts a row into `event` with `(kind, key, data)` and calls `pg_notify()` in the same statement — **not** via a database trigger. The NOTIFY payload is JSON: `{"payload": <T>, "timestamp": <datetime>, "id": <serial>, "kind": <string>, "key": <string|null>}`.
2. Subscribers LISTEN on channel `"plane_events"`, then on each notification, check the embedded `id` against a local watermark to deduplicate. On reconnect, they catch up by querying `event` rows with `id > last_seen` (batch of 100).
3. **Metrics are ephemeral:** `BackendMetricsMessage` is sent via a separate channel `"plane_backend_metrics"` with direct NOTIFY only (no event table persistence).

The `kind` strings are defined by the `NotificationPayload` trait. Verified kinds from upstream:

| `kind` | Type (Rust) | Payload (JSON shape) |
|---|---|---|
| `backend_action` | `BackendActionMessage` | §5.3 |
| `backend_metrics` | `BackendMetricsMessage` | §5.2 |
| `backend_state` | `BackendState` | §14.2 |
| `node_connection` | `NodeConnectionStatusChangeNotification` | `{ node_id: i32, connected: bool }` |

Source: `plane/src/database/{backend.rs,node.rs}`.

Each notification can have an optional `key` (a string), which lets subscribers filter (e.g., subscribe to `backend_action` for `drone_id=42` by keying on the drone id string).

### 13.1 Our additional kinds

We add (with the same `NotificationPayload` mechanism):

| `kind` | Payload | When emitted |
|---|---|---|
| `snapshot_registered` | `{ snapshot_id, sandbox_id, image_ref }` | After drone calls `/ctrl/snapshots` |
| `policy_update` | `{ sandbox_id, version }` | API handler updates `network_policy` table |
| `credential_update` | `{ binding_id, sandbox_id, action: "create" \| "revoke" }` | API handler updates `credential_binding` table |
| `egress_log` | `{ sandbox_id, count }` | Drone batched egress logs persisted |

Subscribers: controller replicas (for SSE fanout in `policy_update` and `snapshot_registered`); the controller replica owning a drone's WS (for `policy_update` and `credential_update` to push to the drone).

NOTIFY payload size limit: ~8 KB. Our payloads are small references; consumers fetch full data from Postgres after the notification, never inline.

### 13.2 Cleanup and retention (Plane native)

Verified against `plane/src/cleanup.rs` and `plane/src/database/{backend.rs,subscribe.rs}`. Every controller replica runs a cleanup loop every **180 seconds** (3 minutes), with a random initial offset to stagger replicas:

- **If the controller is started with `--cleanup-min-age-days <N>`**: the loop deletes, for backends with `last_status = 'terminated'` whose `last_status_time` is older than `N` days, all related rows in this transactional order: `token`, `backend_action`, `backend_state`, `backend_key` (only if `expires_at < now()`), then `backend`. Default batch size: 100 backends per pass (`--cleanup-batch-size`).
- Same flag also deletes from the `event` table any rows older than `N` days (so `LISTEN/NOTIFY` history is bounded).
- **If `--cleanup-min-age-days` is not set**, none of the above runs. The database grows unboundedly until the operator intervenes.

Separately, **expired tokens** (`token.expiration_time < now()`) are deleted every cleanup pass regardless of `--cleanup-min-age-days`.

**Our extension tables** (`sandbox`, `snapshot`, `network_policy`, `secret`, `credential_binding`, `extension_action`, `usage_record`, `port_route`, `audit_egress`) are **not** touched by Plane's cleanup. We add our own cleanup logic to a sibling loop that runs in the same cadence:

| Our table | Cleanup rule |
|---|---|
| `sandbox` | When the backing `backend` row is deleted by Plane cleanup, the FK cascades (sandbox row goes too). |
| `snapshot` | Delete rows whose `expires_at < now()`. Also enqueue a registry tag deletion for the image. |
| `network_policy` | CASCADE on `sandbox` deletion. |
| `credential_binding` | CASCADE on `sandbox` deletion. |
| `secret` | Soft-delete via `deleted_at`; hard-delete after 30 days (avoids breaking audit traces). |
| `extension_action` | Delete rows where `acked_at IS NOT NULL AND acked_at < now() - interval '1 day'`. |
| `usage_record` | Move to cold storage after 90 days; delete after 1 year. |
| `port_route` | CASCADE on `sandbox` deletion. |
| `audit_egress` | Partition by month; drop partitions older than the audit-retention policy (1 year for paranoid tier). |

Recommended operator setting for production: `--cleanup-min-age-days 7`. Anyone running the controller without this flag in production is signing up for unbounded growth.

---

## 14. Shared type reference

### 14.1 `BackendStatus` — Plane

```ts
type BackendStatus =
  | "scheduled"
  | "loading"
  | "starting"
  | "waiting"
  | "ready"
  | "terminating"
  | "hard-terminating"
  | "terminated";
```

Serialized as `lowercase`. Source: `common/src/types/backend_state.rs:9`.

Forward-only. The integer ordering (used for `last_status_number`):

| Status | Int |
|---|---|
| `scheduled` | 10 |
| `loading` | 20 |
| `starting` | 30 |
| `waiting` | 40 |
| `ready` | 50 |
| `terminating` | 60 |
| `hard-terminating` | 65 |
| `terminated` | 70 |

### 14.2 `BackendState` — Plane

A **tagged union**, not a flat string. Wire shape: `{"status": "<status>", ...other fields}`. Source: `common/src/types/backend_state.rs:88`.

```ts
type BackendState =
  | { status: "scheduled" }
  | { status: "loading" }
  | { status: "starting" }
  | { status: "waiting"; address: string }      // "ip:port"
  | { status: "ready"; address: string }
  | { status: "terminating"; last_status: BackendStatus; termination: "soft" | "hard"; reason: TerminationReason }
  | { status: "hard-terminating"; last_status: BackendStatus; reason: TerminationReason }
  | { status: "terminated"; last_status: BackendStatus; termination: "soft" | "hard" | null; reason: TerminationReason | null; exit_code: number | null };

type TerminationReason =
  | "swept"
  | "external"
  | "keyexpired"
  | "lost"
  | "startuptimeout"
  | "internalerror";
```

The `terminating.termination` field is marked `#[deprecated]` in the source; new code should treat `hard-terminating` as its own status, but the field is still on the wire.

### 14.3 `BackendStateMessage` — Plane

```ts
type BackendStateMessage = {
  event_id: number;            // i64
  backend_id: string;
  state: BackendState;
  timestamp: string;           // RFC 3339
}
```

This is the type carried on the **Drone↔Controller WebSocket** (`MessageFromDrone::BackendEvent`). It is NOT what's emitted to SDK callers on the SSE status stream — see §14.4 below.

### 14.4 `BackendStatusStreamEntry` — Plane

The type returned by `GET /pub/b/:backend/status` and emitted as the `data:` payload on `GET /pub/b/:backend/status-stream`. Source: `common/src/types/backend_state.rs`.

```ts
type BackendStatusStreamEntry = {
  status: BackendStatus;
  termination_reason?: TerminationReason;       // present if status is terminating, hard-terminating, or terminated
  termination_kind?: "soft" | "hard";           // ditto
  exit_error?: boolean;                          // true if container exited with non-zero; false if zero; absent if still running
  time: string;                                  // RFC 3339, when the controller observed this status
}
```

Derivation from `BackendState` (the richer type the drone reports):

| Source `BackendState` variant | `termination_reason` | `termination_kind` | `exit_error` |
|---|---|---|---|
| `Scheduled` / `Loading` / `Starting` / `Waiting` / `Ready` | — | — | — |
| `Terminating { reason, .. }` | `reason` | `"soft"` | — |
| `HardTerminating { reason, .. }` | `reason` | `"hard"` | — |
| `Terminated { reason, termination, exit_code, .. }` | `reason` | `termination` | derived: `exit_code != 0` |

**The `address` field present in `BackendState::{Waiting, Ready}` is NOT carried in the stream entry.** Consumers needing the address must obtain it from the `ConnectResponse.url` issued at create time, not from this stream.

### 14.5 `DockerExecutorConfig` — Plane

Source: `common/src/types/mod.rs:224`.

```ts
type DockerExecutorConfig = {
  image: string;
  pull_policy?: PullPolicy;          // default IfNotPresent
  credentials?: DockerRegistryAuth;
  env?: Record<string, string>;      // default {}
  resource_limits?: ResourceLimits;  // default all-null
  mount?: Mount;
  network_name?: string;
}

type PullPolicy = "IfNotPresent" | "Always" | "Never";

type DockerRegistryAuth = {
  username: string;
  password: string;
}

type Mount = boolean | string;       // untagged: bool OR path string

type ResourceLimits = {
  cpu_period?: number;               // microseconds; serialized via serde_with::DurationMicroSeconds; default 100_000 (100ms)
  cpu_period_percent?: number;       // u8, 0-100
  cpu_time_limit?: number;           // seconds; serialized via serde_with::DurationSeconds
  memory_limit_bytes?: number;       // i64
  disk_limit_bytes?: number;         // i64
}
```

**Critical:** `PullPolicy` values are **PascalCase on the wire** (`IfNotPresent`, `Always`, `Never`), not lowercase. This is unusual for Plane (which is mostly lowercase) but is what the source emits.

`Mount` is an untagged enum: send `true`/`false` for a default mount, or a string for a specific path.

### 14.6 `KeyConfig` — Plane

Source: `common/src/types/mod.rs:287`.

```ts
type KeyConfig = {
  name: string;
  namespace?: string;          // default ""
  tag?: string;                // default ""
}
```

A random key has `name: "lk-<random>"`, namespace and tag empty.

### 14.7 `SpawnConfig` — Plane

Source: `common/src/types/mod.rs:251`.

```ts
type SpawnConfig = {
  id?: string;                       // BackendName; let Plane assign if omitted
  cluster?: string;                  // ClusterName; uses controller default if omitted
  pool?: string;                     // default "" (default pool)
  executable: object;                // serde_json::Value; deserialized as DockerExecutorConfig
  lifetime_limit_seconds?: number;   // i32
  max_idle_seconds?: number;         // i32
  use_static_token?: boolean;        // default false
  subdomain?: string;
}
```

### 14.8 `ConnectRequest` — Plane

Source: `common/src/types/mod.rs:315`.

```ts
type ConnectRequest = {
  key?: KeyConfig;
  spawn_config?: SpawnConfig;
  user?: string;                     // → X-Plane-User header
  auth?: Record<string, any>;        // default {} → X-Plane-Auth header
}
```

### 14.9 `ConnectResponse` — Plane

Source: `common/src/types/mod.rs:382`.

```ts
type ConnectResponse = {
  backend_id: string;                // BackendName
  spawned: boolean;
  status: BackendStatus;
  token: string;                     // BearerToken
  url: string;                       // constructed by ConnectResponse::new
  subdomain?: string;
  secret_token?: string;
  status_url: string;
  drone?: string;                    // DroneName, present if spawned
}
```

URL construction (verified at `mod.rs:407`):

```
{https if cluster has no port else http}://{[subdomain.]cluster}/{token}/
```

### 14.10 `AcquiredKey` — Plane

Source: `common/src/protocol.rs`.

```ts
type AcquiredKey = {
  key: KeyConfig;
  deadlines: KeyDeadlines;
  token: number;                     // i64 fencing token
}

type KeyDeadlines = {
  renew_at: string;
  soft_terminate_at: string;
  hard_terminate_at: string;
}
```

### 14.11 `Sandbox` (ours)

```ts
type Sandbox = {
  id: string;                        // sb-...
  backend_id: string;                // Plane's BackendName
  tenant: string;
  status: BackendStatus;

  runtime: string;
  trust: "trusted" | "untrusted" | "paranoid";
  arch: "amd64" | "arm64";
  image: string;                     // resolved OCI ref

  resources: {
    vcpus: number;
    memory_mb: number;
  };

  ports: number[];
  tags: Record<string, string>;

  persistent: boolean;
  snapshot_source?: string;          // snap-id if restored

  created_at: string;
  updated_at: string;
  expires_at: string;
  ready_at?: string;
  terminated_at?: string;

  drone_name?: string;               // DroneName once assigned
  cluster: string;
}
```

### 14.12 `NetworkPolicy` (ours)

```ts
type NetworkPolicy = {
  mode: "allow-all" | "deny-all" | "rules";
  rules?: NetworkRule[];
}

type NetworkRule = {
  domains?: string[];                // "*.example.com" patterns
  cidrs?: string[];
  ports?: number[];
  action?: "allow" | "deny";         // default: allow if inject; else allow
  inject?: string;                   // credential binding id
}
```

Evaluation:
1. Explicit `deny` wins.
2. `inject` implies allow.
3. Other `allow` rules.
4. Default in `rules` mode: deny.

### 14.13 `Snapshot` (ours)

```ts
type Snapshot = {
  id: string;                        // snap-...
  sandbox_id: string;
  tenant: string;
  image_ref: string;
  digest: string;
  base_image: string;
  size_bytes: number;
  arch: "amd64" | "arm64";
  runtime: string;
  name?: string;
  tags: Record<string, string>;
  created_at: string;
  expires_at: string;
}
```

### 14.14 `UsageRecord` (ours)

```ts
type UsageRecord = {
  sandbox_id: string;
  bucket_start: string;              // hour-aligned RFC 3339
  cpu_nsec: number;                  // matches Plane BackendMetrics units (nanoseconds)
  memory_mb_seconds: number;
  rx_bytes: number;
  tx_bytes: number;
}
```

---

## 15. Postgres schema

The platform's database has two parts:

- **Plane's schema** (unchanged from upstream). This is the source of truth for backends, drones, proxies, keys, tokens, ACME state, and event log.
- **Our extensions**, added in a separate migration. These tables reference Plane's `backend` and `node` tables by foreign key.

**Operational parameters (verified from source):**

- **Connection pool:** `max_connections = 20` (hardcoded in `PlaneDatabase::connect()`). No explicit idle timeout, acquire timeout, or statement cache size.
- **Transaction isolation:** Not explicitly configured — uses PostgreSQL's default of `READ COMMITTED`.
- **No foreign keys use CASCADE:** All FKs in Plane's schema are implicit `NO ACTION`. Cleanup must delete children first, in reverse dependency order. This is why our extension tables that reference `backend(id)` use `ON DELETE CASCADE` — Plane's cleanup already handles the deletion order for its own tables, and our rows cascade automatically when `backend` is finally deleted.
- **Cleanup order (verified in `backend.rs`):** Within a single transaction per batch: `token` → `backend_action` → `backend_key` (only if `expires_at < now()`) → `backend_state` → `backend`. Our `sandbox` row cascades from the `backend` delete; everything under `sandbox` cascades from there.

### 15.1 Plane's schema (inherited)

Verified against `plane/schema/derived_schema.sql` (the canonical reference; auto-generated from migrations in `plane/schema/migrations/`).

```sql
-- The sqlx migrations table.
CREATE TABLE _sqlx_migrations (
    version BIGINT PRIMARY KEY,
    description TEXT NOT NULL,
    installed_on TIMESTAMPTZ DEFAULT now() NOT NULL,
    success BOOLEAN NOT NULL,
    checksum BYTEA NOT NULL,
    execution_time BIGINT NOT NULL
);

-- ACME DNS challenge state. Doubles as a lease.
CREATE TABLE acme_txt_entries (
    cluster   VARCHAR(255) PRIMARY KEY,
    leased_at TIMESTAMPTZ DEFAULT now() NOT NULL,
    leased_by INTEGER NOT NULL,
    txt_value VARCHAR(255),
    FOREIGN KEY (leased_by) REFERENCES node(id)
);

-- All nodes (drones, proxies, DNS servers).
CREATE TABLE node (
    id                        SERIAL PRIMARY KEY,
    kind                      VARCHAR(255) NOT NULL,   -- "Drone" | "Proxy" | "AcmeDnsServer"
    name                      VARCHAR(255) NOT NULL,   -- e.g., "dr-7f3d1a"
    cluster                   VARCHAR(255),            -- null only for DNS servers
    plane_version             VARCHAR(255) NOT NULL,
    plane_hash                VARCHAR(255) NOT NULL,
    controller                VARCHAR(255),            -- ControllerName; null = offline
    ip                        INET NOT NULL,
    last_connection_start_time TIMESTAMPTZ,
    FOREIGN KEY (controller) REFERENCES controller(id)
);
CREATE UNIQUE INDEX idx_cluster_name ON node (cluster, name);

-- Drone-specific data (one row per drone node).
CREATE TABLE drone (
    id              INTEGER PRIMARY KEY REFERENCES node(id),
    ready           BOOLEAN NOT NULL,
    draining        BOOLEAN DEFAULT false NOT NULL,
    last_heartbeat  TIMESTAMPTZ,
    last_local_time TIMESTAMPTZ,
    pool            VARCHAR(255) DEFAULT '' NOT NULL    -- trust tier name in our system
);
CREATE INDEX idx_drone_last_heartbeat ON drone (last_heartbeat);

-- Controllers (one row per controller process).
CREATE TABLE controller (
    id             VARCHAR(255) PRIMARY KEY,
    first_seen     TIMESTAMPTZ DEFAULT now() NOT NULL,
    last_heartbeat TIMESTAMPTZ NOT NULL,
    is_online      BOOLEAN NOT NULL,
    plane_version  VARCHAR(255) NOT NULL,
    plane_hash     VARCHAR(255) NOT NULL,
    ip             INET
);

-- Backends (the central entity; our Sandbox wraps these).
CREATE TABLE backend (
    id                   VARCHAR(255) PRIMARY KEY,
    cluster              VARCHAR(255) NOT NULL,
    last_status          VARCHAR(255) NOT NULL,        -- BackendStatus serialized
    last_status_time     TIMESTAMPTZ NOT NULL,
    last_status_number   INTEGER,                      -- for ordering, see §14.1
    cluster_address      VARCHAR(255),                 -- "ip:port"
    drone_id             INTEGER NOT NULL REFERENCES drone(id),
    expiration_time      TIMESTAMPTZ,                  -- hard deadline
    last_keepalive       TIMESTAMPTZ NOT NULL,         -- from proxy KeepAlive
    allowed_idle_seconds INTEGER,
    state                JSONB NOT NULL,               -- BackendState
    static_token         VARCHAR(256),
    subdomain            VARCHAR(255)
);
CREATE INDEX idx_backend_drone_id ON backend (drone_id) WHERE last_status <> 'terminated';
CREATE INDEX idx_backend_static_token ON backend (static_token);

-- History of state transitions (one row per BackendEvent).
CREATE TABLE backend_state (
    id         SERIAL PRIMARY KEY,
    backend_id VARCHAR(255) NOT NULL REFERENCES backend(id),
    state      JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT now() NOT NULL
);
CREATE INDEX idx_backend_state_created_at ON backend_state (backend_id, created_at);

-- The action queue. Controllers insert; drones ack.
CREATE TABLE backend_action (
    id         VARCHAR(255) PRIMARY KEY,               -- BackendActionName, "ak-..."
    backend_id VARCHAR(255) REFERENCES backend(id),
    drone_id   INTEGER NOT NULL REFERENCES drone(id),  -- denormalized for indexing
    action     JSONB NOT NULL,                          -- BackendActionMessage
    created_at TIMESTAMPTZ DEFAULT now() NOT NULL,
    acked_at   TIMESTAMPTZ                              -- null = pending
);
CREATE INDEX idx_backend_action_backend ON backend_action (backend_id);
CREATE INDEX idx_backend_action_created_at ON backend_action (created_at);
CREATE INDEX idx_backend_action_pending ON backend_action (drone_id, created_at)
    WHERE acked_at IS NULL;                              -- partial index for redelivery

-- Keys (one row per held key; deleted on release).
CREATE TABLE backend_key (
    id            VARCHAR(255) PRIMARY KEY REFERENCES backend(id),
    namespace     VARCHAR(255) NOT NULL,
    key_name      VARCHAR(255) NOT NULL,
    tag           VARCHAR(255) NOT NULL,
    expires_at    TIMESTAMPTZ NOT NULL,
    fencing_token BIGINT NOT NULL,
    allow_renew   BOOLEAN DEFAULT true NOT NULL
);
CREATE UNIQUE INDEX idx_namespace_name ON backend_key (namespace, key_name);

-- Tokens (one row per minted connection token).
CREATE TABLE token (
    token            VARCHAR(255) PRIMARY KEY,
    backend_id       VARCHAR(255) NOT NULL REFERENCES backend(id),
    expiration_time  TIMESTAMPTZ NOT NULL,
    username         VARCHAR(255),
    auth             JSONB NOT NULL,
    secret_token     VARCHAR(255) NOT NULL
);

-- The event log (LISTEN/NOTIFY surface).
CREATE TABLE event (
    id         SERIAL PRIMARY KEY,
    kind       VARCHAR(255) NOT NULL,
    key        VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT now() NOT NULL,
    data       JSONB NOT NULL
);
CREATE INDEX idx_event_created_at ON event (created_at);
```

#### Plane's keys: unique constraint nuance

`backend_key` has a `UNIQUE (namespace, key_name)` — meaning a key is unique within a namespace, regardless of its `tag`. The `tag` is used only for the **connect-and-match** logic: a connect request with a key that's already held will return the existing backend if and only if its tag matches the request's tag.

### 15.2 Our schema extensions

Migration filename: `20260601000000_add-sandbox-platform.sql` (date-prefixed to sort after all Plane migrations).

```sql
-- Tenants (lightweight; populated by the front-door auth integration).
CREATE TABLE tenant (
    id          VARCHAR(255) PRIMARY KEY,    -- "tenant-..."
    name        TEXT,
    created_at  TIMESTAMPTZ DEFAULT now() NOT NULL,
    deleted_at  TIMESTAMPTZ
);

-- Our wrapper over Plane's backend table. One row per platform sandbox.
CREATE TABLE sandbox (
    id              VARCHAR(255) PRIMARY KEY,    -- "sb-..."
    backend_id      VARCHAR(255) NOT NULL UNIQUE REFERENCES backend(id),
    tenant          VARCHAR(255) NOT NULL REFERENCES tenant(id),

    runtime         VARCHAR(255) NOT NULL,
    trust_tier      VARCHAR(64)  NOT NULL,       -- "trusted" | "untrusted" | "paranoid"
    arch            VARCHAR(16)  NOT NULL,       -- "amd64" | "arm64"
    image           TEXT NOT NULL,               -- resolved OCI ref at create time

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
CREATE INDEX idx_sandbox_tenant ON sandbox (tenant, created_at DESC);
CREATE INDEX idx_sandbox_tags ON sandbox USING gin (tags);
CREATE INDEX idx_sandbox_backend ON sandbox (backend_id);

-- Snapshots.
CREATE TABLE snapshot (
    id            VARCHAR(255) PRIMARY KEY,    -- "snap-..."
    sandbox_id    VARCHAR(255) NOT NULL REFERENCES sandbox(id) ON DELETE CASCADE,
    tenant        VARCHAR(255) NOT NULL REFERENCES tenant(id),

    image_ref     TEXT NOT NULL,
    digest        VARCHAR(128) NOT NULL,       -- sha256:...
    base_image    TEXT NOT NULL,
    size_bytes    BIGINT NOT NULL,
    arch          VARCHAR(16) NOT NULL,
    runtime       VARCHAR(255) NOT NULL,

    name          TEXT,
    tags          JSONB NOT NULL DEFAULT '{}',

    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at    TIMESTAMPTZ NOT NULL
);
CREATE INDEX idx_snapshot_tenant ON snapshot (tenant, created_at DESC);
CREATE INDEX idx_snapshot_sandbox ON snapshot (sandbox_id);
CREATE INDEX idx_snapshot_expires_at ON snapshot (expires_at)
    WHERE expires_at IS NOT NULL;              -- for GC scans

-- Network policies (one row per sandbox).
CREATE TABLE network_policy (
    sandbox_id VARCHAR(255) PRIMARY KEY REFERENCES sandbox(id) ON DELETE CASCADE,
    version    BIGINT NOT NULL DEFAULT 1,       -- monotonic per sandbox
    mode       VARCHAR(64) NOT NULL,            -- "allow-all" | "deny-all" | "rules"
    rules      JSONB NOT NULL DEFAULT '[]',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Secret material (KMS-encrypted at rest).
CREATE TABLE secret (
    id                VARCHAR(255) PRIMARY KEY,  -- "secret-..."
    tenant            VARCHAR(255) NOT NULL REFERENCES tenant(id),
    name              TEXT,
    kms_key_id        TEXT NOT NULL,
    wrapped_data_key  BYTEA NOT NULL,            -- KMS-wrapped data key
    nonce             BYTEA NOT NULL,            -- AES-GCM nonce
    ciphertext        BYTEA NOT NULL,            -- AES-GCM ciphertext
    auth_tag          BYTEA NOT NULL,            -- AES-GCM tag
    tags              JSONB NOT NULL DEFAULT '{}',
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at        TIMESTAMPTZ
);
CREATE INDEX idx_secret_tenant ON secret (tenant) WHERE deleted_at IS NULL;

-- Credential bindings (link a secret to a sandbox + destination + header).
CREATE TABLE credential_binding (
    id            VARCHAR(255) PRIMARY KEY,    -- "cred-..."
    sandbox_id    VARCHAR(255) NOT NULL REFERENCES sandbox(id) ON DELETE CASCADE,
    secret_id     VARCHAR(255) NOT NULL REFERENCES secret(id),
    destination   TEXT NOT NULL,               -- domain or CIDR
    header_name   TEXT NOT NULL,
    header_format TEXT NOT NULL DEFAULT '{value}',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at    TIMESTAMPTZ
);
CREATE INDEX idx_credential_sandbox ON credential_binding (sandbox_id)
    WHERE revoked_at IS NULL;

-- Outbound action queue for our extensions (policy/credential push, snapshot).
-- Mirrors Plane's backend_action pattern: write here, NOTIFY drone, drone acks.
CREATE TABLE extension_action (
    id         VARCHAR(255) PRIMARY KEY,        -- "xak-..."
    kind       VARCHAR(64) NOT NULL,            -- "push_policy" | "push_credential" | "revoke_credential" | "snapshot"
    backend_id VARCHAR(255) REFERENCES backend(id),
    sandbox_id VARCHAR(255) REFERENCES sandbox(id),
    drone_id   INTEGER NOT NULL REFERENCES drone(id),
    payload    JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    acked_at   TIMESTAMPTZ
);
CREATE INDEX idx_extension_action_pending ON extension_action (drone_id, created_at)
    WHERE acked_at IS NULL;

-- Usage metering (hourly buckets, one row per sandbox per hour).
CREATE TABLE usage_record (
    sandbox_id        VARCHAR(255) NOT NULL REFERENCES sandbox(id) ON DELETE CASCADE,
    bucket_start      TIMESTAMPTZ NOT NULL,
    cpu_nsec          BIGINT NOT NULL,
    memory_mb_seconds BIGINT NOT NULL,
    rx_bytes          BIGINT NOT NULL DEFAULT 0,
    tx_bytes          BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (sandbox_id, bucket_start)
);
CREATE INDEX idx_usage_record_bucket ON usage_record (bucket_start);

-- Subdomain-port routing entries (one row per (sandbox, exposed port) pair).
CREATE TABLE port_route (
    sandbox_id      VARCHAR(255) NOT NULL REFERENCES sandbox(id) ON DELETE CASCADE,
    port            INTEGER NOT NULL,
    subdomain_token VARCHAR(255) NOT NULL,       -- the "sb-abc-3000" form
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (sandbox_id, port)
);
CREATE UNIQUE INDEX idx_port_route_subdomain ON port_route (subdomain_token);

-- Audit log: egress proxy decisions, credential injections.
-- High-volume; partitioned by month in production.
CREATE TABLE audit_egress (
    id                SERIAL PRIMARY KEY,
    at                TIMESTAMPTZ NOT NULL,
    sandbox_id        VARCHAR(255) NOT NULL REFERENCES sandbox(id),
    destination_host  TEXT NOT NULL,
    destination_port  INTEGER NOT NULL,
    destination_ip    INET,
    protocol          VARCHAR(16) NOT NULL,      -- "http" | "tls" | "dns"
    action            VARCHAR(32) NOT NULL,      -- "allow" | "allow_inject" | "deny"
    policy_rule_index INTEGER,
    bytes_sent        BIGINT NOT NULL DEFAULT 0,
    bytes_received    BIGINT NOT NULL DEFAULT 0,
    duration_ms       INTEGER NOT NULL DEFAULT 0,
    tls_sni           TEXT,
    http_host         TEXT,
    http_path         TEXT,
    http_status       INTEGER
);
CREATE INDEX idx_audit_egress_sandbox ON audit_egress (sandbox_id, at DESC);
CREATE INDEX idx_audit_egress_at ON audit_egress (at);
```

### 15.3 Relationship diagram

```
Plane tables (inherited)                Our tables (extension)
───────────────────────                 ─────────────────────────

controller ◀── node ◀── drone           tenant
                ▲                        ▲
                │                        │ (sandbox.tenant)
                │                        │
                │                       sandbox ──▶ snapshot (sandbox_id)
                │                        ▲ ▲
                │                        │ │
                │                        │ ├── network_policy (1:1)
                │                        │ │
                │                        │ ├── credential_binding ──▶ secret
                │                        │ │
                │                        │ ├── port_route (1:many on port)
                │                        │ │
                │                        │ ├── usage_record (1:many on hour)
                │                        │ │
                │                        │ └── audit_egress (1:many)
                │                        │
backend ◀──────┼──── sandbox.backend_id (1:1 FK)
   ▲           │
   ├── backend_action (queue)            
   ├── backend_key (current key)         extension_action (queue, mirrors backend_action)
   ├── backend_state (history)
   └── token (connection tokens)

acme_txt_entries (cluster cert state)
event (notification log)
```

### 15.4 Migration strategy

1. Plane's migrations run first on controller startup (via sqlx; Plane handles this in `connect_and_migrate`).
2. Our migrations are appended after; the migration runner is the same sqlx engine, so they apply in the same `_sqlx_migrations` table.
3. Migration files live under `plane/schema/migrations/` in the forked tree, sorted by filename. Our files use `2026MMDDHHMMSS_*.sql` (or later) so they sort after every Plane migration that existed at fork time.
4. Forward-only. Rollbacks are done via new forward migrations.

### 15.5 What we don't add (and why)

- **No `tenants` table in the controller for fine-grained ACLs.** Auth happens at the front door; we just store the tenant string as opaque text. If a future product needs in-controller ACLs, that's an additive change.
- **No separate `drone_capability` table.** Drone capabilities (arch, supported runtimes) are reported in the WebSocket and held in memory; we don't persist them. If the controller restarts, drones reconnect and re-report.
- **No `secret_version` table.** Secret rotation creates a new secret with a new id and updates the bindings; old secrets are deleted. No history.

---

**End of interfaces document.** Any breaking change to any contract here requires a major version bump on the affected interface and a migration plan documented in the PR. Plane-inherited contracts can only be changed by upstream; deviating requires a hard fork and a corresponding change to our internal docs.
