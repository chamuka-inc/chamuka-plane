# Sandbox Platform — Build Plan

**Status:** v0.3 (TDD-first, Rust-default, solo implementer)
**Implementer:** Me.
**Companions:** [spec](./sandbox-platform-spec.md), [SAD](./sandbox-platform-sad.md), [interfaces](./sandbox-platform-interfaces.md)

This document is the plan for me to implement the platform one binary at a time, test-first, with contracts as the substrate that makes "one binary at a time" actually work.

Three constraints drive every decision:

1. **I lose context between sessions.** The artifacts on disk are the only durable state.
2. **No tests exist yet.** The fixtures, schemas, stubs, and test harnesses are themselves a deliverable. Until that's in place, no binary can be built test-first because there's nothing to be red against.
3. **TDD is the workflow.** For each feature: red → green-impl → e2e (against stubs) → live e2e (against real systems and the contract fixtures). No code lands without going through all four.

This document tells future me: what to build first, what discipline to follow, and how to leave the project in a state any next session can pick up.

---

## Table of contents

1. [Build philosophy](#1-build-philosophy)
2. [Language choices](#2-language-choices)
3. [Repo layout](#3-repo-layout)
4. [Shared types via the `common` crate](#4-shared-types-via-the-common-crate)
5. [Contract fixtures — the integration surface](#5-contract-fixtures--the-integration-surface)
6. [Walking skeleton](#6-walking-skeleton)
7. [Per-binary spec sheets](#7-per-binary-spec-sheets)
   - [7.1 agentd-headless](#71-agentd-headless)
   - [7.2 sandbox-runtime images](#72-sandbox-runtime-images)
   - [7.3 plane (controller mode)](#73-plane-controller-mode)
   - [7.4 plane (drone mode)](#74-plane-drone-mode)
   - [7.5 plane (proxy mode)](#75-plane-proxy-mode)
   - [7.6 egress-proxy](#76-egress-proxy)
   - [7.7 agentd-desktop](#77-agentd-desktop)
   - [7.8 ops-cli](#78-ops-cli)
   - [7.9 sdk-typescript](#79-sdk-typescript)
   - [7.10 sdk-python](#710-sdk-python)
8. [Implementation order](#8-implementation-order)
9. [Fork management — living with upstream Plane](#9-fork-management--living-with-upstream-plane)
10. [CI/CD scaffold](#10-cicd-scaffold)

---

## 1. Build philosophy

Four project-specific rules. Cross-cutting engineering discipline (TDD, code shape, source control, session handoff) lives in the [discipline doc](./sandbox-platform-discipline.md), imported in every session via [CLAUDE.md](../CLAUDE.md).

**Rule 1: Discovery before scaffolding.** Three discovery spikes (Phase D in §8) precede any test infrastructure or binary work. They validate that the design holds by running it, not by reading it. Spikes are disposable; their findings — corrections to the docs, verified facts about Plane's behavior, one working live-e2e test — are not. If a spike fails, the docs are wrong; update them before proceeding.

**Rule 2: Test infrastructure before binary code.** Before any binary implementation, we need a working `contracts/` directory, the `common` crate, fixtures derived from verified Plane source, the `common::stubs` mocks, schema generation, and CI workflows that can actually run something. This is [Phase 0](#8-implementation-order) and it's not optional — without it, "test-first" is just words.

**Rule 3: Rust by default, exceptions only for SDKs.** Everything in one Cargo workspace except the TS and Python SDKs (their consumers live in those ecosystems).

**Rule 4: The walking skeleton always works.** Every session ends with the walking skeleton green or with an explicit note in the session-ending commit body saying "broke it, fix this first next session." The walking skeleton is the canary across time.

Beyond these, the [discipline doc](./sandbox-platform-discipline.md) governs how code arrives: TDD per feature (red → green-impl → e2e → live e2e), the propagation rule, code-shape prohibitions enforced by clippy, source-control conventions, and git-as-status-tracker via commit trailers.

---

## 2. Language choices

| Binary | Language | Reason |
|---|---|---|
| agentd-headless | **Rust** | Same workspace as plane fork; share types via `common`. |
| agentd-desktop | **Rust** | Cargo `desktop` feature on agentd, not a separate binary. |
| plane (controller / drone / proxy) | **Rust** | Already Rust upstream. |
| egress-proxy | **Rust** | `tonic` for the xDS gRPC server; `envoy-types` proto crate. |
| ops-cli | **Rust** | `clap`. Reuses `common` and plane types directly. |
| sandbox-runtime images | **Dockerfiles** | Not a language. |
| sdk-typescript | **TypeScript** | Consumers are in npm. |
| sdk-python | **Python** | Consumers are in PyPI. |

Six Rust binaries in one Cargo workspace, plus two SDKs in their native ecosystems. Three toolchains total: rustup, pnpm, uv.

---

## 3. Repo layout

```
sandbox-platform/
├── crates/                          # single flat Cargo workspace
│   ├── .cargo/config.toml           # rustflags (tracing_unstable)
│   ├── Cargo.toml                   # workspace root
│   ├── common/                      # shared types + test stubs (feature-gated)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── protocol.rs          # MessageFromDrone/ToDrone + our ext variants
│   │   │   ├── types/               # BackendState, etc.
│   │   │   ├── ext/                 # our extension types
│   │   │   │   ├── network_policy.rs
│   │   │   │   ├── credential.rs
│   │   │   │   ├── snapshot.rs
│   │   │   │   └── extension_action.rs
│   │   │   ├── stubs/               # behind feature = "test-stubs"
│   │   │   │   ├── mock_controller.rs
│   │   │   │   ├── mock_drone.rs
│   │   │   │   ├── mock_proxy.rs
│   │   │   │   └── mock_docker.rs
│   │   │   └── test_helpers/        # load_fixture, etc.
│   │   └── tests/                   # tests for the helpers themselves
│   │
│   ├── plane/                       # hard fork of jamsocket/plane (modify freely)
│   │   ├── Cargo.toml               # depends on common, plane-common, plane-dynamic-proxy
│   │   ├── .sqlx/                   # offline query metadata (SQLX_OFFLINE=true)
│   │   ├── src/
│   │   │   ├── controller/          # controller mode
│   │   │   ├── drone/               # drone mode
│   │   │   ├── proxy/               # proxy mode
│   │   │   └── ext/                 # sandbox platform extensions
│   │   │       ├── mod.rs
│   │   │       ├── api_v1.rs        # /pub/v1/* handlers
│   │   │       ├── dispatch.rs
│   │   │       └── types.rs
│   │   └── schema/migrations/       # ours date-prefixed after Plane's
│   │
│   ├── plane-common/                # Plane's shared library (upstream origin)
│   │   ├── Cargo.toml
│   │   └── src/
│   │
│   ├── plane-dynamic-proxy/         # Plane's reverse proxy library (upstream origin)
│   │   ├── Cargo.toml
│   │   └── src/
│   │
│   ├── plane-tests/                 # Plane integration tests (require Docker)
│   │   ├── Cargo.toml
│   │   └── tests/
│   │
│   ├── plane-test-macro/            # proc macro for Plane test harness
│   │   ├── Cargo.toml
│   │   └── src/
│   │
│   ├── agentd/                      # in-sandbox HTTP daemon
│   │   ├── Cargo.toml               # has "desktop" feature
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── http/
│   │   │   ├── exec/
│   │   │   ├── fs/
│   │   │   ├── ports/
│   │   │   └── computer/            # #[cfg(feature = "desktop")]
│   │   ├── tests/
│   │   └── README.md
│   │
│   ├── egress-proxy/                # xDS publisher for local Envoy
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   └── tests/
│   │
│   ├── ops-cli/                     # operator tooling (sbctl binary)
│   │   ├── Cargo.toml
│   │   └── src/
│   │
│   └── tools/                       # workspace-internal binaries
│       └── generate-schemas/        # emits contracts/schemas/ from common types
│           ├── Cargo.toml
│           └── src/main.rs
│
├── contracts/                       # the cross-language integration surface
│   ├── schemas/                     # generated; committed; CI fails if stale
│   │   └── *.schema.json
│   ├── fixtures/                    # hand-written initially; tracked carefully
│   │   ├── api/
│   │   ├── agentd/
│   │   ├── extension-action/
│   │   ├── network/
│   │   ├── sandbox/
│   │   ├── snapshot/
│   │   └── usage/
│   └── README.md
│
├── runtime-images/                  # OCI images
│   ├── base/Dockerfile
│   ├── node24/Dockerfile
│   ├── python313/Dockerfile
│   ├── bun1/Dockerfile
│   ├── desktop/Dockerfile
│   └── build.sh
│
├── sdks/
│   ├── typescript/                  # @sandbox-platform/sdk
│   └── python/                      # sandbox-platform-sdk
│
├── dev-fleet/                       # Vagrant-based local fleet (already shipped)
│
├── walking-skeleton/                # the Layer-4 integration test
│   ├── test.sh
│   ├── docker-compose.yml
│   └── README.md
│
├── docs/                            # spec, SAD, interfaces, this file
├── scripts/
│   └── status.sh                    # prints current Phase, Next, walking-skeleton status
├── Makefile                         # bootstrap, test-all, walking-skeleton
└── README.md
```

One Cargo workspace; all crates at the same level under `crates/`. The plane fork (`plane/`, `plane-common/`, `plane-dynamic-proxy/`, `plane-tests/`, `plane-test-macro/`) lives alongside our custom crates (`common`, `agentd`, `egress-proxy`, `ops-cli`). SDKs sit outside the workspace because their tooling is per-language. `runtime-images/`, `walking-skeleton/`, `contracts/`, `docs/` are language-agnostic siblings.

**Build note:** Always set `SQLX_OFFLINE=true` when building without a live Postgres. The `.sqlx/` directory in `crates/plane/` contains cached query metadata.

---

## 4. Shared types via the `common` crate

Within the Rust workspace, two binaries that exchange a message agree by importing the type from `common`. No codegen, no manual sync — the types are the contract.

The `common` crate has four sub-modules:

- `common::protocol` — Plane's wire types, re-exported. (Our forked plane crate re-exports the same types from its own `common`; we just unify the path so other crates import from one place.)
- `common::types` — Plane's data types (`BackendStatus`, `BackendState`, etc.).
- `common::ext` — types we add: `PushNetworkPolicyMessage`, `PushCredentialMessage`, `SnapshotCommand`, etc. These extend Plane's enums as new variants in our fork.
- `common::stubs` — test doubles behind `feature = "test-stubs"`. `MockController`, `MockDrone`, `MockProxy`, `MockDockerSocket`, etc.

Schemas are generated from the Rust types:

```rust
// crates/tools/generate-schemas/src/main.rs
fn main() {
    write_schema::<common::protocol::MessageFromDrone>("MessageFromDrone");
    write_schema::<common::protocol::MessageToDrone>("MessageToDrone");
    write_schema::<common::ext::PushNetworkPolicyMessage>("PushNetworkPolicyMessage");
    // ... one call per top-level type
}
```

`cargo run --bin generate-schemas` regenerates everything under `contracts/schemas/`. CI runs this and fails if `git diff` is non-empty.

SDKs generate their language types from the schemas:
- TypeScript: `quicktype` or `json-schema-to-typescript`
- Python: `datamodel-code-generator` → Pydantic

The propagation chain:

```
common (Rust types, hand-written)
    └─▶ schemars              → JSON Schema (committed)
            └─▶ quicktype     → TypeScript types
            └─▶ datamodel-cg  → Pydantic models
```

One source of truth.

---

## 5. Contract fixtures — the integration surface

Fixtures pin the wire shapes as concrete examples. They're the gold-standard inputs and expected outputs that every Layer 2 contract test loads.

### 5.1 Variants per type

For every message type in [interfaces §5](./sandbox-platform-interfaces.md#5-drone--controller-websocket-plane--our-additions), §6, §10, and §3, three fixture variants:

- `*.minimal.json` — only required fields
- `*.typical.json` — common shape
- `*.full.json` — every optional field populated

Plus `negative/*.json` for invalid examples.

### 5.2 Example: `backendEvent.ready.typical.json`

```json
{
  "BackendEvent": {
    "event_id": 42,
    "backend_id": "be-7f3d1a",
    "state": {
      "status": "ready",
      "address": "10.0.0.5:32768"
    },
    "timestamp": "2026-05-16T12:34:56.789Z"
  }
}
```

### 5.3 How fixtures are born

The first fixtures (Phase 0 work) are written by hand, derived from Plane's source. Process:

1. Read the verified type in interfaces.md.
2. Write a fixture file as a plausible instance.
3. Run it through schema validation (CI's `ajv` check).
4. Where possible, validate it against real upstream Plane (Layer 4 live e2e) — send it as a real wire message and confirm Plane accepts it.

Step 4 is critical for any fixture that travels to or from Plane. It's the only check that the fixture isn't subtly wrong in a way schema validation misses.

### 5.4 How fixtures change

When a contract changes, the [propagation rule (discipline §1.8)](./sandbox-platform-discipline.md#18-what-this-means-for-new-features) applies. Any binary's Layer 2 tests that used the old fixture will fail until that binary is updated.

---

## 6. Walking skeleton

The minimum end-to-end path. The project-wide Layer 4 test.

### 6.1 What it does

```ts
const sandbox = await Sandbox.create({ runtime: 'node24' });
const result = await sandbox.runCommand('node', ['-e', 'console.log("hi")']);
expect(result.exitCode).toBe(0);
expect(await result.stdout()).toBe('hi\n');
await sandbox.stop();
```

Stages:

1. SDK posts to a controller's `/ctrl/connect`.
2. Controller picks a drone from `untrusted` pool.
3. Drone pulls `sandbox-runtime:node24-skel` (minimal image with `agentd-headless`).
4. Drone starts the container; `agentd` boots, listens on `:8080`.
5. Drone emits `BackendEvent::Ready`.
6. SDK opens HTTPS to `<cluster>/<token>/v1/exec` via the proxy.
7. Proxy resolves the token via `RouteInfoRequest`, byte-pipes to `agentd`.
8. `agentd` runs the command, returns the result.
9. SDK calls `/ctrl/b/<id>/soft-terminate`; everything shuts down.

### 6.2 What it uses

- **Stock Plane** (`plane/plane:latest`). Not our fork yet.
- **Stock Postgres** (container).
- **Our `agentd-headless`** binary.
- **Our minimal sandbox-runtime image** (base + node 24 + agentd).
- **A minimal `sdk-typescript`** (just `create`, `runCommand`, `stop`).
- **The dev fleet** for hosting.

### 6.3 What it does NOT use

- No `/pub/v1/*` wrapper.
- No egress proxy, no network policy, no credentials.
- No snapshots.
- No desktop variant.
- No metering aggregation.

### 6.4 When it merges and why it matters

The walking skeleton is the *first* Layer 4 test that runs in CI. It merges after Phase 0 (test infrastructure) and the first two binaries (agentd-headless and the base sandbox-runtime image). From then on, every PR runs it. If a PR breaks the canary, it doesn't merge.

The walking skeleton is also how I detect that a contract has drifted from reality. If the controller fork stops accepting a fixture that real Plane accepts, the walking skeleton (which uses real Plane) keeps passing while our isolated tests would also need a fix.

---

## 7. Per-binary spec sheets

Format per binary: purpose, contracts in/out, what can be stubbed, the TDD layers and what each contains, build & test commands, and a done definition.

### 7.1 agentd-headless

**Purpose.** HTTP daemon that runs as PID 1 (via `tini`) inside every sandbox container. Exposes exec, filesystem, ports, processes.

**Contracts consumed.** None at the network layer (server only). POSIX, cgroup v2 (read), inotify, `/proc`.

**Contracts produced.** HTTP API on `:8080` per [interfaces §10](./sandbox-platform-interfaces.md#10-agentd-http-api-our-extension): `/health`, `/info`, `/v1/exec*`, `/v1/fs/*`, `/v1/ports`, `/v1/processes`. Not `/v1/computer` (that's the desktop variant, §8.7).

**Dependencies.** `axum`, `tokio`, `tower-http`, `serde`, `nix`, `tracing`. The `common` crate.

**What can be stubbed.** Nothing in production code. For tests, `tokio::process` lets us spawn child processes deterministically; `tempfile::tempdir` for filesystem isolation.

**TDD layers.**

- **Layer 1 (unit).** Per-handler tests: `parse_exec_request_strips_invalid_paths`, `pty_resize_message_routed_correctly`, etc. Many small tests, no I/O.
- **Layer 2 (contract).** Each endpoint has a test that loads a request fixture from `contracts/fixtures/agentd/`, calls the handler, asserts the response matches the response fixture.
  - `exec.simple.{request,response}.json`
  - `exec.with_pty.{request,response}.json`
  - `exec.detached.{request,response}.json`
  - `fs.write.{request,response}.json`
  - `fs.list.{request,response}.json`
  - `ports.{request,response}.json`
  - `health.response.json`
- **Layer 3 (e2e stubbed).** Bring up the real `axum` server on an ephemeral port; hit it with `reqwest`. No real container; runs on host. Covers the WebSocket streaming endpoints which are awkward in unit tests.
- **Layer 4 (live e2e).** Build the Docker image, run `agentd` inside a real container in the dev fleet (or `docker run` locally), `curl` from outside. Compare responses to fixtures byte-for-byte.

**Build & test.**
```bash
cd crates/agentd
cargo test                  # layers 1 and 2
cargo test --test e2e_stubbed   # layer 3
make live-e2e              # layer 4 (requires docker)
```

**Acceptance criteria.**
- Layer 1 has > 80% coverage of branches.
- Every endpoint in interfaces §10.1 (except `/v1/computer`) has a Layer 2 contract test.
- Exec covers: sync vs detached, PTY vs pipes, stdin, signal forwarding, output truncation at 10 MB.
- WebSocket streaming emits frames matching `LogFrame` and `FsEvent`.
- Negative-test suite: invalid JSON, missing fields → 400 with our error envelope.
- Binary size < 15 MB stripped. Cold start (process start → `/health` 200) < 100 ms.

**Done definition.**
- All four layers green.
- Walking skeleton uses this binary and passes.
- `cargo bench --bench cold_start` reports < 100 ms p95.

---

### 7.2 sandbox-runtime images

**Purpose.** OCI images the drone pulls when spawning. Embeds `agentd` as PID 1, plus one runtime (Node 24, Python 3.13, Bun 1, desktop).

**Contracts consumed.** `agentd` binary (§8.1). The `runtime` enum values in interfaces §3.2.

**Contracts produced.** Images at `<registry>/sandbox-runtime/<name>:<version>`.

**Dependencies.** Docker, `buildx`.

**What can be stubbed.** Nothing. Pure builds.

**TDD layers.**

There's no Layer 1 here (these aren't testable as code). The relevant layers:

- **Layer 2 (image smoke).** Each `Dockerfile` has a corresponding test in `runtime-images/tests/<name>.sh` that:
  - Builds the image.
  - Runs it.
  - Curls `/health` from outside.
  - Asserts the response.
- **Layer 3 (in-runtime).** Image smoke test extends to running a one-liner in the runtime's language via `/v1/exec` and asserting output. Verifies the runtime is actually installed and works.
- **Layer 4 (live).** The walking skeleton uses these images. Passes there = good.

**Build & test.**
```bash
cd runtime-images
./build.sh node24 v0.1.0          # buildx for amd64 + arm64
./tests/node24.sh                  # layer 2
./tests/node24-runtime.sh          # layer 3
```

**Acceptance criteria.**
- Image sizes: base < 50 MB, node24 < 200 MB, python313 < 250 MB, bun1 < 100 MB.
- Multi-arch (amd64 + arm64).
- Cold start container → agentd ready: < 500 ms with image cached, runsc runtime.
- Per-runtime smoke + in-runtime tests pass.

**Done definition.**
- `./build.sh all v0.1.0` produces all images for both arches.
- All Layer 2/3 tests pass.
- Walking skeleton uses `node24` and passes.

---

### 7.3 plane (controller mode)

**Purpose.** Forked Plane controller. Adds `/pub/v1/*` SDK routes, our extension actions, our Postgres tables, our notification kinds. Plane's controller behavior intact.

**Contracts consumed.**
- HTTP from SDK: interfaces §3.
- `MessageFromDrone` (Plane + our `AckExtensionAction`).
- `MessageFromProxy` (Plane native).
- KMS.

**Contracts produced.**
- HTTP responses to SDK.
- `MessageToDrone` (Plane + our extension variants).
- `MessageToProxy` (Plane + our `SubdomainPortRouteUpdate`).
- Postgres writes per interfaces §15.2.
- LISTEN/NOTIFY emissions per §13.1.

**Dependencies.** Plane's set, plus `aws-sdk-kms`.

**What can be stubbed.**
- **Drone:** `common::stubs::MockDrone`.
- **Proxy:** `common::stubs::MockProxy`.
- **KMS:** `MockKms` for tests; `local-kms` container for dev.
- **Postgres:** **not stubbed.** Real Postgres in testcontainers. Plane's behavior depends on Postgres specifics.

**TDD layers, phased.**

**Phase A — schema migration.**
- *Layer 2:* migration applies on a fresh Plane-migrated DB; tables exist; FKs hold.
- *Layer 3:* upstream Plane tests still pass after adding our migration directory.
- *Layer 4:* run our forked controller (with our migrations) against the walking-skeleton; passes.

**Phase B — `/pub/v1/*` routes.**
- *Layer 1:* per-handler unit tests on request validation, idempotency-key caching, error mapping.
- *Layer 2:* every `/pub/v1/*` endpoint has a request/response fixture pair; contract test loads request, asserts response.
- *Layer 3:* e2e with `MockDrone` — `POST /pub/v1/sandboxes` triggers a fake spawn; observe state transitions through the mock.
- *Layer 4:* live e2e — send our `createSandbox.request.full.json` to real upstream Plane (translated to a Plane `ConnectRequest`); verify Plane accepts it. This catches "our translation is subtly wrong."

**Phase C — extension actions.**
- *Layer 1:* unit tests on the queue mechanism, the redelivery logic, the notification fanout.
- *Layer 2:* fixture tests for `PushNetworkPolicyMessage`, `PushCredentialMessage`, `RevokeCredentialMessage`, `SnapshotCommand`.
- *Layer 3:* e2e with `MockDrone` that ACKs extension actions; verify `extension_action.acked_at` is set; verify redelivery on mock-reconnect.
- *Layer 4:* live e2e — run controller + real drone (our fork, Phase B); push a network policy; verify it lands in the drone's xDS feeder.

**Phase D — production hardening.**
- *Layer 1/2:* tests for cleanup logic (our extension tables match Plane's cleanup semantics).
- *Layer 3:* multi-replica e2e — three controller replicas, stubbed drone reconnects across replicas; verify state is consistent.
- *Layer 4:* three-replica live e2e behind a load balancer; walking skeleton against each individually.

**Build & test.**
```bash
cd crates/plane
cargo test -p plane --lib                  # layers 1+2
cargo test -p plane --test controller_e2e  # layer 3 (uses MockDrone, real Postgres)
make live-e2e-controller                   # layer 4 (real Plane image side-by-side)
```

**Done definition.**
- All four phases × four layers green.
- Walking skeleton passes against forked controller (substituting it for `plane/plane:latest`).

---

### 7.4 plane (drone mode)

**Purpose.** Forked Plane drone. Handles Plane's `Spawn`/`Terminate` plus our extension actions. Runs metering, manages containers via Docker, installs sandbox netns rules, forwards xDS config to egress-proxy.

**Contracts consumed.**
- `MessageToDrone` (Plane + ours).
- Docker Engine API per interfaces §8.
- cgroup v2 files.
- Local OCI registry.

**Contracts produced.**
- `MessageFromDrone` (Plane + our `AckExtensionAction`).
- Drone → egress-proxy UDS messages.
- Docker API calls.
- iptables rules in sandbox netns.

**Dependencies.** `bollard` (Docker client), `iptables` crate or `nix`, Plane's set.

**What can be stubbed.**
- **Controller:** `common::stubs::MockController`.
- **egress-proxy:** `common::stubs::MockEgressPublisher` (records xDS pushes).
- **Docker:** for unit tests, `bollard`'s test support. For integration tests, real Docker (dind in CI).

**TDD layers, phased.**

**Phase A — Plane-native protocol.**
- *Layer 1:* unit tests for action dispatch, dedup by `action_id`, heartbeat cadence.
- *Layer 2:* fixture tests for every `MessageToDrone::Action` variant; assert correct Docker API call sequence and emitted `MessageFromDrone`.
- *Layer 3:* e2e with `MockController` + real `bollard` against a real Docker socket. Drone receives a fixture-derived `Spawn`; verify real container started; verify state events back to mock.
- *Layer 4:* live e2e — our drone connected to real upstream Plane controller image. Plane sends a real `Spawn`. Our drone runs a real container. Plane sees real state events. This is the biggest validation step.

**Phase B — our extensions.**
- *Layer 1:* unit tests on `PushNetworkPolicy` translation to xDS config (golden files).
- *Layer 2:* contract tests for each extension action.
- *Layer 3:* e2e with `MockController` pushing extension actions, `MockEgressPublisher` recording the resulting xDS pushes.
- *Layer 4:* live e2e — our forked controller pushing real extension actions to our drone; drone applies to real egress-proxy.

**Phase C — metering.**
- *Layer 1:* unit tests on cgroup file parsing, unit conversions (microsecs → nanosecs).
- *Layer 2:* fixture tests for `BackendMetricsMessage` shape.
- *Layer 3:* e2e — drone running real containers; verify metering messages match.
- *Layer 4:* validate against real upstream Plane consuming our metering messages.

**Build & test.**
```bash
cd crates/plane
cargo test -p plane --test drone_unit         # layer 1+2
cargo test -p plane --test drone_e2e          # layer 3 (real Docker)
make live-e2e-drone                            # layer 4 (real upstream Plane)
```

**Done definition.**
- All three phases × four layers green.
- Walking skeleton runs against forked drone.

---

### 7.5 plane (proxy mode)

**Purpose.** Forked Plane proxy. Adds subdomain-port routing.

**Contracts consumed.**
- HTTPS/WSS from end-users.
- `MessageToProxy` (Plane + our `SubdomainPortRouteUpdate`).

**Contracts produced.**
- `MessageFromProxy` (Plane native).
- Reverse-proxied HTTPS/WSS to `agentd` via drones.
- ACME challenge responses.

**What can be stubbed.**
- **Controller:** `common::stubs::MockController` answers `RouteInfoRequest` from fixtures.
- **Backend (agentd):** an HTTP echo for unit/integration; real agentd in live e2e.

**TDD layers, phased.**

**Phase A — Plane-native.** Upstream tests pass in our fork; Layer 4 = walking skeleton.

**Phase B — subdomain-port routing.**
- *Layer 1:* unit tests on subdomain parser.
- *Layer 2:* contract tests using `SubdomainPortRouteUpdate` fixtures.
- *Layer 3:* e2e — proxy + `MockController` + HTTP echo agentd; subdomain URL routes correctly.
- *Layer 4:* live e2e in the dev fleet.

**Phase C — ACME hardening.**
- *Layer 3:* test against Let's Encrypt **staging** environment.
- *Layer 4:* test against real domain certs (manual ops; not in regular CI).

**Build & test.**
```bash
cd crates/plane
cargo test -p plane --test proxy_e2e
```

**Done definition.**
- All phases × four layers green.
- Walking skeleton runs against forked proxy.

---

### 7.6 egress-proxy

**Purpose.** Daemon on each drone host that runs alongside Envoy. Receives policy updates from the local drone over UDS, translates them into Envoy xDS resources, serves them to Envoy. Also relays Envoy access logs back to the drone.

**Contracts consumed.**
- Policy push from drone over UDS (interfaces §7).
- Envoy ALS (access log) gRPC stream.

**Contracts produced.**
- xDS gRPC to Envoy (LDS, RDS, CDS, EDS).
- Access log batches to drone.

**Dependencies.** `tonic`, `envoy-types`, `tokio`, `serde`. The `common` crate.

**What can be stubbed.**
- **Drone (policy source):** unit tests feed policy fixtures directly through the trait abstraction over UDS.
- **Envoy:** for Layer 3, a recording xDS client stub. For Layer 4, real Envoy in a container.

**TDD layers.**

- **Layer 1.** Per-translation unit tests: `NetworkPolicy` with N rules → expected xDS LDS/RDS/CDS structure (golden file comparison).
- **Layer 2.** Contract fixtures for `NetworkPolicy`, `CredentialBinding`; assert that a given policy fixture produces a given xDS config fixture.
- **Layer 3.** Real `tonic` server against a recording xDS client stub; verify resource version increments correctly across updates.
- **Layer 4.** Real Envoy in a container; push fixture-derived policies; send real curl from a sandbox netns; assert allow/deny/inject behavior.

**Build & test.**
```bash
cd crates/egress-proxy
cargo test
make test-with-envoy   # layer 4
```

**Acceptance criteria.**
- Four scenarios (allow, deny, allow-inject, MITM) pass in Layer 4 integration.
- Per-sandbox identification via loopback IP scheme (interfaces §7.2) correctly attributes traffic in access logs.
- TLS MITM with per-sandbox internal CA: agentd installs CA at boot, proxy presents signed cert, sandbox stack verifies.
- Latency: passthrough < 5 ms p95; MITM < 25 ms p95.
- 100 concurrent sandboxes per host.

**Done definition.**
- All four layers green.

---

### 7.7 agentd-desktop

**Purpose.** Adds `/v1/computer` to agentd. Same binary, `desktop` Cargo feature enables additional handlers and X11 deps.

**Contracts consumed.** X11 via Xvfb. `xdotool`, `scrot`, `gnome-screenshot`.

**Contracts produced.** `/v1/computer` per interfaces §10.8.

**What can be stubbed.** For Layer 1, the X11 calls can be mocked; for Layer 3+, real Xvfb in a headless container.

**TDD layers.**

- **Layer 1.** Per-action parsing tests; coordinate validation; `xdotool` arg generation tests (golden strings).
- **Layer 2.** Contract tests against `contracts/fixtures/agentd/computer.*.json` for every action.
- **Layer 3.** Real Xvfb in CI; assert behavior end-to-end (screenshot bytes change after click).
- **Layer 4.** Run the desktop image in the dev fleet; have an AI agent loop drive it. Manual but reproducible.

**Build & test.**
```bash
cd crates/agentd
cargo test --features desktop
docker build -f Dockerfile.desktop -t agentd-desktop:dev .
```

**Acceptance criteria.**
- Every action in interfaces §10.8 implemented; names and param shapes match Anthropic `computer_20251124` exactly (verified against the quickstarts repo).
- Latencies: screenshot < 80 ms, click < 30 ms, type 100 chars < 200 ms.
- Round-trip e2e: screenshot → click → screenshot, pixel change near click point.

**Done definition.**
- All layers green.
- Desktop runtime image builds and smoke-tests.

---

### 7.8 ops-cli

**Purpose.** Operator tooling: snapshot GC, tenant management, drone draining, audit queries.

**Contracts consumed.**
- `/ctrl/*` admin routes.
- Postgres for direct queries.

**Contracts produced.** CLI output (table or JSON).

**Dependencies.** `clap`, `reqwest`, `sqlx`, `tabled`. The `common` crate.

**What can be stubbed.** `common::stubs::MockController` HTTP fake.

**TDD layers.**

- **Layer 1.** Per-subcommand: arg parsing, output formatting (table vs JSON).
- **Layer 2.** Each subcommand has a contract test: load a controller response fixture, run the CLI against a `MockController`, assert stdout.
- **Layer 3.** Real controller (our fork) in a container; CLI runs against it; verifies commands actually do what they say.
- **Layer 4.** CLI against the live dev fleet.

**Build & test.**
```bash
cd crates/ops-cli
cargo test
./target/release/sbctl drones list --cluster localhost:9090
```

**Done definition.**
- All subcommands (`drones`, `sandboxes`, `snapshots`, `tenants`, `secrets`, `audit`) implemented and tested at all four layers.

---

### 7.9 sdk-typescript

**Purpose.** TypeScript client for `/pub/v1/*`.

**Contracts consumed.**
- HTTP API per interfaces §3.
- SSE per §3.5.
- Data-plane HTTPS/WSS to agentd via proxy per §9.

**Contracts produced.** Public `Sandbox` class.

**Type generation.** `contracts/schemas/*.schema.json` → TypeScript types via `quicktype`. Committed.

**What can be stubbed.**
- `MockServer` (`msw`) serves `/pub/v1/*` from fixtures.
- `MockProxy` byte-pipes to `MockAgentd`.

**TDD layers.**

- **Layer 1.** Per-method unit tests with `msw` intercepts; SSE consumer state-machine tests.
- **Layer 2.** Contract tests against `contracts/fixtures/http/*`: every endpoint exercised, request/response fixture pair.
- **Layer 3.** Real `axum` server (an embedded test server backed by fixtures) plus an embedded WS echo for agentd; SDK end-to-end against this.
- **Layer 4.** The walking skeleton.

**Build & test.**
```bash
cd sdks/typescript
pnpm install
pnpm build
pnpm test                       # layers 1+2
pnpm test:e2e                   # layer 3
pnpm test:walking-skeleton      # layer 4
```

**Acceptance criteria.**
- API surface mirrors `@vercel/sandbox` where overlapping (verified against `vercel/sandbox` source).
- Our extensions (`trust`, `bindCredential`, `updateNetworkPolicy`) marked in JSDoc.
- SSE consumer: keepalives, `Last-Event-ID` resume with status-string semantics.
- Retries: 5xx + 429 with backoff; 4xx → no retry.
- Generated types: pass `tsc --noEmit` strict; cover every type in `contracts/schemas/`.

**Done definition.**
- All four layers green.
- Published as `@sandbox-platform/sdk@0.1.0`.

---

### 7.10 sdk-python

**Purpose.** Python client. Parallels TypeScript, idiomatic Python.

**Contracts consumed.** Same as §7.9.

**Contracts produced.** `Sandbox` class with async methods. Pydantic models.

**Type generation.** Schemas → Pydantic via `datamodel-code-generator`.

**TDD layers.** Same shape as §7.9, with `pytest` + `respx` instead of `vitest` + `msw`.

**Build & test.**
```bash
cd sdks/python
uv venv && source .venv/bin/activate
uv pip install -e ".[dev]"
pytest tests/                    # layers 1+2
pytest tests/e2e/                # layer 3
pytest tests/walking_skeleton/   # layer 4
```

**Done definition.**
- All four layers green.
- Published as `sandbox-platform-sdk==0.1.0`.

---

## 8. Implementation order

The order I actually do this in. Each item is a session or small handful of sessions. Phase D comes first, then Phase 0, then numbered phases.

### Phase D — Discovery (before any commitment to scaffolding)

Design verification by reading is not verification by running. Before sinking weeks into test infrastructure and binary code, three discovery spikes ground every later assumption in something we executed, not something we read. Spikes are disposable; their findings are not.

**If any spike fails, stop.** Update the docs to reflect what's actually true, then re-run the spike. Do not proceed to Phase 0 until all three spikes pass and the docs reflect what was learned.

```
D.1  Plane source deep-read
     ├─ Read every file under crates/plane/src/database/
     │  in detail (it's small — a few thousand lines).
     ├─ Trace: sqlx query patterns, transaction boundaries,
     │  connection-pool config, LISTEN/NOTIFY emission paths,
     │  cleanup loop integration points.
     ├─ Verify the unknowns currently hand-waved in the docs:
     │  - BackendMetricsMessage.cpu_used semantics: does it include
     │    runsc overhead or just the sandbox process's CPU time?
     │  - idle-timeout sweeper interaction with backend_action queue
     │    under contention
     │  - exact wire serialization of DurationMicroSeconds /
     │    DurationSeconds (number vs string)
     │  - event-table cleanup behavior with --cleanup-min-age-days
     │  - Plane's transaction isolation defaults
     ├─ Verify: that our ext/ extension table FKs into Plane tables
     │  won't break upstream cleanup queries.
     └─ Output: corrections and additions to interfaces.md
        (especially §13 LISTEN/NOTIFY, §15 schema) and any new
        anti-patterns added to [discipline §2.3](./sandbox-platform-discipline.md#23-project-specific-prohibitions). Update doc
        version numbers and date stamps.

D.2  First Layer-4 contract verification
     ├─ Bring up real upstream Plane in a docker-compose stack
     │  (postgres + plane controller + plane drone, all stock).
     ├─ Manually write contracts/fixtures/http/connectRequest.full.json
     │  and contracts/fixtures/http/connectResponse.full.json based
     │  on what interfaces.md §3 says.
     ├─ Send the request fixture as a real HTTP POST to /ctrl/connect.
     │  Inspect the response. Does it match the response fixture?
     ├─ Open a real drone-socket WebSocket; send our
     │  contracts/fixtures/messageFromDrone/backendEvent.ready.json.
     │  Does Plane's controller accept it? What rows change in the DB?
     ├─ For every discrepancy: fix the fixture and update interfaces.md
     │  (the fixture is wrong until proven otherwise). Re-run until
     │  the test passes.
     ├─ Verify at least 3 fixture pairs end-to-end this way:
     │  ConnectRequest/Response, BackendStateMessage (Ready),
     │  BackendActionMessage (Spawn).
     └─ Output: one or more passing test scripts under a scratch
        directory (will be cleaned up; the verified fixtures persist).
        This is the seed of Phase 0's Layer-4 infrastructure.

D.3  Walking-skeleton spike
     ├─ Disposable end-to-end implementation. No TDD, no per-binary
     │  spec sheets, no contract-test layer beyond reuse of D.2 —
     │  just enough code to run a real sandbox lifecycle.
     ├─ Quick-and-dirty Rust agentd: single file, /health and /v1/exec
     │  (sync only), maybe 200 lines.
     ├─ Minimal node24 Dockerfile that wraps agentd.
     ├─ Stock Plane (controller + drone + proxy) from the public image.
     ├─ 100-line TypeScript script that talks to Plane's /ctrl/connect
     │  directly, hits agentd via the proxy, runs `echo hi`, asserts
     │  the output, then soft-terminates.
     ├─ Run inside the existing dev-fleet (Vagrant VM).
     ├─ Output: a green spike run, captured as a screen recording or
     │  log file for future reference. The spike code lives on a
     │  branch named `spike/walking-skeleton`; it is NOT merged to
     │  main. Phase 1 will reimplement these binaries under full TDD
     │  discipline.
     └─ If the spike fails: identify why (network? Plane behavior we
        misunderstood? our image misconfigured?), update the docs
        with what we learned, retry the spike. Do not enter Phase 0
        with a failing or skipped spike.
```

**End of Phase D:** the docs reflect verified reality, at least one Layer-4 contract test is known to work, a disposable but green walking skeleton has run. Confidence in the design is no longer based on careful reading; it's based on having executed it. From here, Phase 0's investment makes sense.

Sessions estimate: Phase D is 3–6 sessions total. D.1 is one session (reading + doc updates). D.2 is 1–2 sessions (infrastructure setup, then per-fixture verification). D.3 is 1–3 sessions depending on what reality reveals.

### Phase 0 — Test infrastructure

This must complete before any binary work. Without it, "test-first" is not possible.

```
0.1  Workspace scaffolding
     ├─ Cargo workspace at crates/
     ├─ Empty crates with Cargo.toml: common, plane, agentd, egress-proxy,
     │  ops-cli, tools/generate-schemas
     ├─ Top-level Makefile, .gitignore, README
     ├─ .githooks/{pre-commit,pre-push} (see [discipline §3.6](./sandbox-platform-discipline.md#36-pre-commit-hooks))
     ├─ scripts/status.sh skeleton (see [discipline §4.2](./sandbox-platform-discipline.md#42-reading-current-state))
     └─ Vagrantfile etc. from existing dev-fleet/ pulled in

0.2  Fork Plane
     ├─ Clone jamsocket/plane into crates/plane/ (preserving git history)
     ├─ Set up upstream tracking branch
     ├─ Confirm cargo build --release green
     ├─ Confirm cargo test green on upstream tests
     └─ UPSTREAM.md documents the rebase strategy

0.3  Build common types
     ├─ common::protocol re-exports from plane::common
     ├─ common::types re-exports
     ├─ common::ext skeleton (empty types matching interfaces.md §14)
     ├─ Derive Serialize/Deserialize/JsonSchema on everything
     └─ cargo test passes (only re-export tests at this point)

0.4  Schema generation
     ├─ crates/tools/generate-schemas/ binary
     ├─ Calls write_schema::<T> for every top-level type
     ├─ Writes to contracts/schemas/*.schema.json
     └─ Run it, commit the generated schemas

0.5  Initial fixtures
     ├─ Hand-write minimal/typical/full variants for every contract type
     ├─ Source the values from verified Plane source (the *.rs files we read)
     ├─ Validate each against its schema (one-off script)
     └─ Commit. CI will validate on every change going forward.

0.6  Test helpers
     ├─ common::test_helpers::load_fixture(path) -> serde_json::Value
     ├─ common::test_helpers::load_fixture_as<T>(path) -> T
     └─ Tests for the helpers themselves

0.7  Mock stubs
     ├─ common::stubs::MockController (behind feature = "test-stubs")
     ├─ common::stubs::MockDrone
     ├─ common::stubs::MockProxy
     ├─ common::stubs::MockDockerSocket
     ├─ Minimal: each can play canned fixture responses and record what
     │  it received
     └─ Tests for the stubs themselves

0.8  CI workflows
     ├─ rust-workspace.yml: cargo build / test / clippy / fmt
     ├─ contracts.yml: regenerate schemas, fail if git diff non-empty;
     │  validate every fixture against its schema
     ├─ walking-skeleton.yml: not yet runnable; skeleton script written
     │  but will fail until §1.1 completes
     └─ All three jobs green or appropriately red (skeleton is allowed red
        only until binaries exist)
```

End of Phase 0: workspace builds, types compile, schemas committed, fixtures validate, stubs work, CI runs. No binary is implemented yet, but the *infrastructure for implementing binaries test-first* is in place.

### Phase 1 — Walking skeleton

```
1.1  agentd-headless to passing walking-skeleton level
     ├─ TDD per endpoint: /health, /info, /v1/exec (sync only), /v1/fs/read,
     │  /v1/fs/write
     ├─ Layer 1 + 2 + 3 green
     └─ Layer 4 deferred to 1.3

1.2  Minimal sandbox-runtime/node24 image
     ├─ Dockerfile that embeds agentd-headless
     ├─ Smoke tests (Layer 2 + 3)
     └─ Push to local registry

1.3  Walking skeleton green
     ├─ A 50-line sdk-typescript talks to stock Plane /ctrl/connect
     ├─ End-to-end: create, exec, stop
     ├─ This is also agentd-headless Layer 4
     └─ Walking-skeleton CI workflow turns from "allowed red" to "must green"
```

End of Phase 1: the canary flies. The simplest possible end-to-end works. Any future change that breaks this fails CI.

### Phase 2 — agentd completion + SDK v0

```
2.1  agentd-headless feature-complete
     ├─ All remaining endpoints from interfaces §10
     ├─ WebSocket streaming endpoints
     ├─ inotify watch
     └─ All four layers green for every endpoint

2.2  sdk-typescript v0.1
     ├─ Talks to stock Plane /ctrl/connect (still no /pub/v1/* wrapper)
     ├─ Full method surface (runCommand, writeFiles, etc.)
     ├─ Layers 1, 2, 3 green; Layer 4 = walking skeleton
     └─ Publish to private registry
```

### Phase 3 — Plane fork: controller

```
3.1  Controller Phase A — schema migration
     ├─ Our migration file in plane/schema/migrations/
     ├─ Layer 2: migration applies; Plane's tests still pass
     └─ Layer 4: walking skeleton against our controller (with no /pub/v1/*
        yet, so this only validates "our migration didn't break Plane")

3.2  Controller Phase B — /pub/v1/* routes
     ├─ TDD per endpoint
     ├─ Each endpoint goes through layers 1-4
     ├─ Update sdk-typescript to use /pub/v1/* (still pre-extensions)
     └─ Walking skeleton routes through our wrapper

3.3  Controller Phase C — extension action queue
     ├─ extension_action table, NOTIFY infra
     ├─ MessageToDrone::PushNetworkPolicy variant added to common::ext
     ├─ Regenerate schemas, add fixtures
     ├─ Controller-side dispatch: Layers 1-3 green
     └─ Layer 4 deferred to drone Phase B (4.2 below)
```

### Phase 4 — Plane fork: drone

```
4.1  Drone Phase A — Plane-native
     ├─ Upstream tests green in our fork (mostly nothing to do; we just
     │  haven't broken Plane)
     └─ Layer 4: walking skeleton uses our drone

4.2  Drone Phase B — extension handlers
     ├─ Handler for each MessageToDrone extension variant
     ├─ Layers 1-3 green using MockController + MockEgressPublisher
     ├─ Layer 4 completes controller Phase C end-to-end: our controller
     │  + our drone, real network policies push and apply
     └─ Snapshot path also lands here (most complex extension)

4.3  Drone Phase C — metering
     ├─ cgroup reader, BackendMetrics emission
     ├─ Layers 1-3 green
     └─ Layer 4: real metering flows through real Plane controller
```

### Phase 5 — egress-proxy

```
5.1  egress-proxy
     ├─ xDS publisher, UDS receiver
     ├─ Layers 1-3 green
     └─ Layer 4: with real Envoy in dev fleet
```

### Phase 6 — Plane fork: proxy

```
6.1  Proxy Phase A — Plane-native
6.2  Proxy Phase B — subdomain-port routing
6.3  Proxy Phase C — ACME hardening
```

### Phase 7 — Production hardening

```
7.1  Controller Phase D — cleanup, multi-replica
7.2  Comprehensive walking skeleton variants (snapshot, network policy)
```

### Phase 8 — Full SDK + ops

```
8.1  sdk-typescript completion
     ├─ network policy, credentials, snapshots, persistent sandboxes
8.2  sdk-python (mirrors TS)
8.3  ops-cli
```

### Phase 9 — Desktop + paranoid

```
9.1  agentd-desktop
9.2  desktop runtime image
9.3  paranoid tier (Kata + Firecracker drone pool)
```

### 8.1 Estimating

Each numbered item above is roughly one or a small handful of implementation sessions. Phases 0–2 are foundation; everything after is feature work that the infrastructure now supports.

### 8.2 Recovery if a session ends mid-task

The session-ending commit's body carries forward. Example for a session that ended halfway through Phase 10.3-B (controller `/pub/v1/sandboxes` POST):

```
plane: red: failing test for /pub/v1/sandboxes happy path

Wrote the contract test loading createSandbox.request.full.json
and asserting the response matches createSandbox.response.full.json.
Test currently fails because build_connect_request() returns
todo!() — handler scaffold is in place but the conversion logic
isn't there yet.

File: crates/plane/src/ext/api_v1.rs line 87
Layer 1 (unit) tests for request validation: green
Layer 2 (contract) tests: 1 red, the rest not yet written

Phase: 10.3-B
Next: 1. Read interfaces.md §3.2 (CreateSandboxRequest -> ConnectRequest
         mapping).
      2. Implement build_connect_request() in api_v1.rs.
      3. Run: cargo test -p plane --test api_v1 --
            create_sandbox_happy_path until green.
      4. Once green, run walking-skeleton; should still pass
         (this endpoint isn't on its path yet).
      5. Commit (green), then write the error-case tests.
```

The next session runs `./scripts/status.sh` (or `git log -1 --format=%B`), sees this commit's trailers, runs the test (confirms it's still red for the same reason), implements, gets green, commits with the new `Next:` trailer pointing at the error-case work. Zero archaeology.

---

## 9. Fork management — hard fork

### 9.1 Strategy

`crates/plane/` is a **hard fork** of `jamsocket/plane`. There is no upstream tracking branch, no rebase cadence, no `ext/` restriction. We modify any file freely — controller, drone, proxy, database, schema — as needed for the sandbox platform.

Our sandbox-specific code lives primarily in:

- `crates/plane/src/ext/` — API handlers, dispatch, types
- `crates/plane/schema/migrations/2026*_*.sql` — date-prefixed after upstream migrations

But we also fix, refactor, and extend upstream code directly (boy scout principle).

### 9.2 Cherry-picking upstream fixes

If `jamsocket/plane` ships a bug fix or feature we want, cherry-pick it manually:

```bash
# Add upstream as a remote (one-time)
git remote add upstream-plane https://github.com/jamsocket/plane.git
git fetch upstream-plane
# Cherry-pick the specific commit(s)
git cherry-pick <commit-hash>
# Resolve conflicts, test, commit
SQLX_OFFLINE=true cargo test --workspace --exclude plane-tests
```

### 9.3 No contributing back

This is a divergent fork. We don't maintain PR compatibility with upstream.

---

## 10. CI/CD scaffold

```
.github/workflows/
├── rust-workspace.yml       # cargo build / test / clippy / fmt on every PR
├── contracts.yml            # regenerate schemas, fail if drift; validate fixtures
├── sdk-typescript.yml       # build, test, type-check
├── sdk-python.yml           # build, test
├── runtime-images.yml       # docker buildx; only on tag or main
└── walking-skeleton.yml     # the canary; on every PR
```

### 10.1 The contracts job (most critical)

```yaml
- run: cargo run --bin generate-schemas
- run: git diff --exit-code contracts/schemas/
- run: ./scripts/validate-fixtures.sh    # ajv-based
```

If you changed a Rust type without regenerating schemas, CI fails. If you changed a schema without committing it, CI fails. This is the propagation rule made executable.

### 10.2 The walking-skeleton job

Brings up the dev fleet, builds the latest of every binary, runs the end-to-end test. If red, PR doesn't merge.

### 10.3 Per-binary Layer 4 jobs

For binaries with expensive live e2e (real Plane, real Envoy, real Docker), each has its own job that runs only on PRs touching that binary:

```yaml
on:
  pull_request:
    paths:
      - 'crates/plane/**'
      - 'crates/common/**'
      - 'contracts/**'
```

This keeps CI time bounded.

---

**End of build plan.** Phase D first (verify the design by running it), then Phase 0 (build the test infrastructure), then phases 1–9 with TDD discipline per the [discipline doc](./sandbox-platform-discipline.md). The four-layer testing pyramid, the propagation rule, and the git history (with `Phase:` and `Next:` trailers) are the spine. Everything else falls out.