# Dev Fleet — Ops Guide

Operational reference for running and testing the Docker Compose multi-drone fleet.
Maintained by the AI agent; updated as the fleet evolves.

---

## Images

Two images, two roles. Never mix them.

| Image | Contents | Role |
|---|---|---|
| `chamuka-plane:latest` | `/bin/plane` + `/bin/wget` (no shell) | Controller, proxy, drones |
| `sandbox-runtime:node24-latest` | agentd + Node 24 + tini | Sandbox backends |

Build `chamuka-plane` with `SQLX_OFFLINE=true`. The image is distroless —
no shell, no curl. `/bin/wget` is copied from busybox for healthchecks.
Use `CMD` format (not `CMD-SHELL`) since there is no shell.

Other runtime variants: `sandbox-runtime:agent-latest` (node24 + git + jq + curl + wget + procps),
`sandbox-runtime:desktop-latest` (with X11/VNC),
`sandbox-runtime:ysweet-latest` (y-sweet 0.6.3 pre-installed on port 3000).

## Starting the fleet

### Canonical (Vagrant)

```bash
make fleet-images
make fleet-up
make smoke
```

Host services run via `dev-fleet/host/docker-compose.yml`; drones run in
VirtualBox VMs. Cluster name: `192.168.56.1:9090`.

### Docker fallback

```bash
make fleet-images
make fleet-up-docker
make smoke-docker
```

```bash
cd dev-fleet && docker compose up -d   # equivalent to make fleet-up-docker
```

Seven containers: `fleet-postgres`, `fleet-controller`, `fleet-proxy`,
`fleet-drone-1` (trusted), `fleet-drone-2` (untrusted), `fleet-drone-3` (untrusted),
`fleet-registry`.

Verify all registered:

```bash
curl -s http://localhost:8080/ctrl/c/localhost%3A9090/state | python3 -m json.tool
```

Expected: 3 drones (`drone-trusted-1`, `drone-untrusted-1`, `drone-untrusted-2`),
1 proxy.

## Stopping the fleet

```bash
cd dev-fleet && docker compose down
```

Add `-v` to also wipe Postgres data and registry contents.

## Fleet status

```bash
make fleet-status
```

Shows the canonical Vagrant fleet status. For Docker-only fallback status, use
`make fleet-status-docker`.

## Running the smoke test

```bash
./dev-fleet/scripts/smoke-test.sh
```

Exercises both pools end-to-end: spawn, proxy routing, agentd health/exec/fs,
cross-pool isolation, cleanup. 12 assertions, takes ~10s.

Environment overrides: `CONTROLLER`, `PROXY`, `CLUSTER`, `IMAGE`.

---

## Network topology

```
Host machine
├── fleet-controller  :8080  ← API + WebSocket coordination
├── fleet-proxy       :9090  ← routes /{token}/... to backend containers
├── fleet-drone-1..3         ← schedule containers via Docker socket
├── fleet-postgres    :5432  ← controller state
├── fleet-registry    :5000  ← local image registry (unused in Docker mode)
│
└── plane-{backend-id}       ← sandbox containers (spawned as siblings)
    └── agentd :8080         ← mapped to ephemeral host port
```

Drones mount `/var/run/docker.sock` and advertise `host.docker.internal` as their IP.
The proxy (also in Docker) reaches sandbox containers via `host.docker.internal:{port}`.

## Port map

| Port | Service | Bind |
|---|---|---|
| 8080 | Controller API | 127.0.0.1 |
| 9090 | Proxy (HTTP) | 127.0.0.1 |
| 5432 | Postgres | 127.0.0.1 |
| 5000 | Registry | 127.0.0.1 |

---

## Controller API

All routes defined in `crates/plane/src/controller/mod.rs`.

### Public routes (`/pub/...`)

| Method | Path | Purpose |
|---|---|---|
| GET | `/pub/health` | Health check (returns empty 200) |
| GET | `/pub/b/{backend}/status` | Backend status |
| GET | `/pub/b/{backend}/status-stream` | SSE status stream |

### Control routes (`/ctrl/...`)

| Method | Path | Purpose |
|---|---|---|
| GET | `/ctrl/status` | Controller status |
| GET | `/ctrl/c/{cluster}/state` | Cluster state (drones, proxies) |
| POST | `/ctrl/connect` | Spawn or reuse backend |
| POST | `/ctrl/b/{backend}/soft-terminate` | Graceful terminate |
| POST | `/ctrl/b/{backend}/hard-terminate` | Forced terminate |
| POST | `/ctrl/c/{cluster}/d/{drone}/drain` | Drain a drone |

Cluster name in URLs must be URL-encoded: `localhost:9090` → `localhost%3A9090`.

### ConnectRequest (POST /ctrl/connect)

Source: `ConnectRequest` in `crates/plane-common/src/types/mod.rs`.

```json
{
  "key": {
    "name": "my-key",
    "namespace": "",
    "tag": ""
  },
  "spawn_config": {
    "cluster": "localhost:9090",
    "pool": "trusted",
    "executable": {
      "image": "sandbox-runtime:node24-latest",
      "env": {}
    },
    "lifetime_limit_seconds": 300,
    "max_idle_seconds": 60
  }
}
```

**Critical fields:**

- `key` is a `KeyConfig` object, not a string. Always required when spawning.
  `name` is the reuse key; same name+namespace+tag reuses an existing backend.
- `spawn_config` (not ~~`spawn`~~). Contains the spawn parameters.
- `spawn_config.pool` must match a drone's `--pool` flag. Default is `""` (empty string),
  which matches no named pool. Use `"trusted"` or `"untrusted"`.
- `spawn_config.executable` is `serde_json::Value` deserialized as `DockerExecutorConfig`.
- `spawn_config.id` is optional. If provided, it becomes the `BackendName`.

**ConnectResponse:**

```json
{
  "backend_id": "my-backend-id",
  "spawned": true,
  "status": "scheduled",
  "token": "abc123...",
  "url": "http://localhost:9090/abc123.../",
  "status_url": "http://0.0.0.0:8080/pub/b/my-backend-id/status",
  "drone": "drone-trusted-1"
}
```

### BackendName validation

Source: `crates/plane-common/src/names.rs`.

- Max 45 characters.
- Lowercase alphanumeric and hyphens only: `[a-z0-9-]`.
- No uppercase, no underscores, no dots.
- macOS `date +%s%N` outputs literal `N` (not nanoseconds) — never use `%N` in backend IDs.

### Backend lifecycle

`scheduled` → `loading` → `starting` → `ready` → `terminating` → `terminated`

Poll status via `GET /pub/b/{backend}/status`. Typical time to `ready`: 1–3s
for cached images.

---

## Proxy routing

The proxy strips the token prefix and forwards to the backend container.

**Host header must end in the cluster name.** Use `localhost:9090`, not `127.0.0.1:9090`.

```bash
# correct
curl http://localhost:9090/{token}/health -H "Host: localhost:9090"

# wrong — returns 403 "Host header does not end in cluster name"
curl http://127.0.0.1:9090/{token}/health
```

The proxy adds response headers:
- `x-plane-backend-id: {backend_id}`
- `server: Plane/0.5.5-`

---

## agentd API

All routes defined in `crates/agentd/src/server.rs`. agentd listens on port 8080 inside
the container.

### Routes

| Method | Path | Purpose |
|---|---|---|
| GET | `/health` | Health (no `/v1` prefix) |
| GET | `/info` | Runtime info |
| POST | `/v1/exec` | Execute command |
| GET | `/v1/exec/{cmd_id}` | Exec status |
| POST | `/v1/exec/{cmd_id}/stdin` | Send stdin |
| POST | `/v1/exec/{cmd_id}/signal` | Send signal |
| GET | `/v1/exec/{cmd_id}/stdout` | Stdout stream (SSE) |
| GET | `/v1/exec/{cmd_id}/stderr` | Stderr stream (SSE) |
| GET | `/v1/exec/{cmd_id}/logs` | Combined logs |
| GET | `/v1/fs/read?path=...` | Read file |
| PUT | `/v1/fs/write` | Write file |
| POST | `/v1/fs/mkdir` | Create directory |
| DELETE | `/v1/fs/rm` | Remove file/dir |
| GET | `/v1/fs/list?path=...` | List directory |
| POST | `/v1/fs/move` | Move/rename |
| GET | `/v1/fs/watch` | Watch directory (SSE) |
| GET | `/v1/ports` | Listening ports |
| GET | `/v1/processes` | Process list |
| POST | `/v1/computer` | Desktop automation |

### ExecRequest (POST /v1/exec)

Source: `ExecRequest` in `crates/common/src/ext/agentd.rs`.

`cmd` is a binary path passed to `Command::new()`, not a shell command string.
To run shell commands, use `/bin/sh -c`:

```json
{
  "cmd": "/bin/sh",
  "args": ["-c", "node --version && uname -a"],
  "env": {},
  "timeout_ms": 5000
}
```

**Wrong** (will fail with "No such file or directory"):
```json
{"cmd": "node --version"}
```

**Sync response** (when `detached: false`, the default):

```json
{
  "cmd_id": "uuid",
  "exit_code": 0,
  "stdout": "v24.15.0\n",
  "stderr": "",
  "duration_ms": 6
}
```

**Detached response** (when `detached: true`):

```json
{
  "cmd_id": "uuid",
  "status": "started"
}
```

For detached processes, use `/v1/exec/{cmd_id}` to poll status,
`/v1/exec/{cmd_id}/stdout` for streaming output, and
`/v1/exec/{cmd_id}/signal` to send signals.

### Filesystem operations

```bash
# Read file
curl "http://localhost:9090/{token}/v1/fs/read?path=/etc/hostname" -H "Host: localhost:9090"

# List directory
curl "http://localhost:9090/{token}/v1/fs/list?path=/" -H "Host: localhost:9090"

# Write file (PUT, body is raw file content)
curl -X PUT "http://localhost:9090/{token}/v1/fs/write?path=/tmp/test.txt" \
  -H "Host: localhost:9090" -d "hello"

# Mkdir
curl -X POST "http://localhost:9090/{token}/v1/fs/mkdir" \
  -H "Host: localhost:9090" -H "Content-Type: application/json" \
  -d '{"path":"/tmp/mydir","recursive":true}'
```

---

## Troubleshooting

### Logs

```bash
# All fleet containers
docker compose -f dev-fleet/docker-compose.yml logs -f

# Specific service
docker compose -f dev-fleet/docker-compose.yml logs drone-1 --tail 50

# Sandbox container
docker logs plane-{backend-id}
```

### Common issues

**"NoDroneAvailable"** on connect:
- `pool` not specified → defaults to `""`, matches no named pool.
- Fix: add `"pool": "trusted"` or `"pool": "untrusted"` to `spawn_config`.

**403 "Host header does not end in cluster name"** from proxy:
- Using `127.0.0.1:9090` instead of `localhost:9090`.
- Fix: set `-H "Host: localhost:9090"` or use `localhost` in the URL.

**"Failed to decode backend name"** on status check:
- Backend ID contains uppercase or invalid characters.
- Fix: IDs must be `[a-z0-9-]` only, max 45 chars.

**Backend stays in "starting", never reaches "ready"**:
- Image exits immediately (e.g. `node:22-slim` with no server process).
- The drone's `wait_for_backend` polls TCP for up to 5 minutes. It also
  checks container liveness every 5 iterations (~500ms) — if the container
  exited, it returns `ContainerExited` immediately and the backend transitions
  to `terminated` with the exit code.
- Fix: use `sandbox-runtime:node24-latest` (runs agentd, stays alive).

**Healthcheck failures on controller/proxy**:
- `chamuka-plane:latest` is distroless — no shell. `CMD-SHELL` healthchecks fail.
- Fix: use `CMD` format with `/bin/wget`: `["CMD", "/bin/wget", "-q", "--spider", "http://localhost:8080/pub/health"]`.

**Backend terminates instantly after spawn (deserialization error)**:
- `pull_policy` values are PascalCase: `IfNotPresent`, `Always`, `Never`.
- Kebab-case (`if-not-present`) or lowercase (`always`) causes a serde error
  and immediate termination.
- Fix: use `"pull_policy": "IfNotPresent"` in the executable block.

**"KeyUnheldNoSpawnConfig"**:
- `spawn_config` missing from the connect request body.
- Fix: include `spawn_config` with at least `executable.image`.

**Container pruned before you could test**:
- Plane prunes idle containers. Default `lifetime_limit_seconds` controls this.
- Fix: set `lifetime_limit_seconds: 300` (5 min) or higher.

### Inspecting sandbox containers

```bash
# List running sandbox containers
docker ps --filter "name=plane-"

# Exec into a sandbox (if it has a shell)
docker exec -it plane-{backend-id} /bin/sh

# Direct agentd access (bypass proxy)
PORT=$(docker port plane-{backend-id} 8080 | cut -d: -f2)
curl http://localhost:$PORT/health
```

### Resetting the fleet

```bash
# Stop everything including sandbox containers
docker compose -f dev-fleet/docker-compose.yml down
docker ps --filter "name=plane-" -q | xargs -r docker rm -f

# Full reset (wipe DB, registry)
docker compose -f dev-fleet/docker-compose.yml down -v
```

---

## Quick-reference: full lifecycle in one shell

```bash
# 1. Spawn
RESP=$(curl -sf -X POST http://localhost:8080/ctrl/connect \
  -H 'Content-Type: application/json' \
  -d '{
    "key": {"name": "test-1", "namespace": "dev", "tag": ""},
    "spawn_config": {
      "cluster": "localhost:9090",
      "pool": "trusted",
      "executable": {"image": "sandbox-runtime:node24-latest", "env": {}},
      "lifetime_limit_seconds": 300
    }
  }')

TOKEN=$(echo "$RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])")
BID=$(echo "$RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['backend_id'])")

# 2. Wait for ready
while true; do
  STATUS=$(curl -sf "http://localhost:8080/pub/b/$BID/status" \
    | python3 -c "import sys,json; print(json.load(sys.stdin)['status'])")
  [ "$STATUS" = "ready" ] && break
  sleep 1
done

# 3. Use via proxy
curl -sf "http://localhost:9090/$TOKEN/health" -H "Host: localhost:9090"
curl -sf "http://localhost:9090/$TOKEN/v1/exec" \
  -H "Host: localhost:9090" -H "Content-Type: application/json" \
  -d '{"cmd":"/bin/sh","args":["-c","node --version"]}'

# 4. Terminate
curl -sf -X POST "http://localhost:8080/ctrl/b/$BID/soft-terminate"
```

---

## Source of truth

Before making API calls, check the struct definitions:

| What | Where |
|---|---|
| ConnectRequest, SpawnConfig, KeyConfig | `crates/plane-common/src/types/mod.rs` |
| BackendName validation | `crates/plane-common/src/names.rs` |
| Controller routes | `crates/plane/src/controller/mod.rs` |
| agentd routes | `crates/agentd/src/server.rs` |
| ExecRequest and all agentd types | `crates/common/src/ext/agentd.rs` |
| Docker Compose fleet | `dev-fleet/docker-compose.yml` |
| Smoke test | `dev-fleet/scripts/smoke-test.sh` |
