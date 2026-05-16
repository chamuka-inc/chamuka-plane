# Sandbox Platform — local dev fleet

Bring up the [Phase 1 walking
skeleton](../sandbox-platform-spec.md#phase-1--walking-skeleton-23-weeks)
on a single workstation, using VirtualBox VMs as drones.

Every Plane interface used here is verified against the upstream source
at <https://github.com/jamsocket/plane> — see [verified
interfaces](#verified-interfaces) below.

```
┌────────────────────────────────────────────────────────────────┐
│  YOUR WORKSTATION (the host)                                    │
│                                                                 │
│  ┌─────────────────────────────────────────────┐               │
│  │ Host services (docker compose)              │               │
│  │  - Postgres                       :5432     │               │
│  │  - Plane controller               :8080     │               │
│  │  - Plane proxy                    :9090     │               │
│  │  - Local pull-through registry    :5000     │               │
│  └─────────────────────────────────────────────┘               │
│                       ▲                                         │
│                       │  host-only network 192.168.56.0/24      │
│       ┌───────────────┼───────────────┐                         │
│       ▼               ▼               ▼                         │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐                     │
│  │drone-1  │    │drone-2  │    │drone-3  │   VirtualBox VMs    │
│  │.10      │    │.11      │    │.12      │                     │
│  │ trusted │    │untrusted│    │untrusted│                     │
│  │  runc   │    │  runsc  │    │  runsc  │                     │
│  └─────────┘    └─────────┘    └─────────┘                     │
└────────────────────────────────────────────────────────────────┘
```

## Prerequisites

- VirtualBox 7.0+
- Vagrant 2.4+
- Docker Engine + Docker Compose v2
- 16 GB RAM minimum
- 30 GB free disk
- Ports 8080, 9090, 5432, 5000 free on the host

`make check` validates these before bringing the fleet up.

## Bring it up

```bash
make check     # verify prerequisites
make up        # host services + 3 VMs
make smoke     # end-to-end verification
```

First `make up` takes 5–10 minutes (downloading the Ubuntu box, pulling
images, provisioning). Subsequent runs are under a minute.

## Configuration

Copy `.env.example` to `.env`, or set env vars directly:

```bash
DRONE_COUNT=5 DRONE_VCPUS=4 DRONE_MEM_MB=8192 make up
```

## Common tasks

```bash
make status          # what's running
make cluster-state   # ask the controller who's registered
make smoke           # end-to-end test
make logs            # tail controller logs
make ssh-1           # ssh into drone-1
make down            # halt VMs (host keeps running)
make nuke            # destroy everything including volumes
```

## Verified interfaces

This automation depends on these exact Plane interfaces. If they change
upstream, the files below need updating.

### Plane image

- `plane/plane:latest` — Docker Hub. Same image binary for controller,
  drone, and proxy; the first CLI argument selects the role.
  - Source: `docker/docker-compose.yml`, `plane/src/main.rs`.

### Controller CLI

| Flag | Type | Default | Required |
|---|---|---|---|
| `--db` | Postgres URL | — | yes |
| `--host` | IP | `127.0.0.1` | no |
| `--port` | u16 | `8080` | no |
| `--controller-url` | URL | derived | no |
| `--default-cluster` | cluster name | — | no |
| `--forward-auth` | URL | — | no |
| `--cleanup-min-age-days` | i32 | — | no |
| `--best-of` | i64 | `2` | no |

Source: `plane/src/controller/command.rs`.

### Drone CLI

| Flag | Type | Default | Required |
|---|---|---|---|
| `--name` | drone name | random | no |
| `--controller-url` | WebSocket URL (`ws://...`) | — | yes |
| `--cluster` | cluster name | — | yes |
| `--ip` | IP or hostname | `127.0.0.1` | no |
| `--db` | path | in-memory | no |
| `--docker-runtime` | string (`runc`, `runsc`, ...) | Docker default | no |
| `--pool` | pool name | default pool | no |
| `--mount-base` | path | — | no |
| `--auto-prune-images` | bool | false | no |
| `--auto-prune-containers-older-than-seconds` | i32 | `0` | no |

Source: `plane/src/drone/command.rs`.

**Key detail:** the `--docker-runtime` flag is how a drone instructs
Docker to use gVisor (or any other registered OCI runtime) for spawned
backend containers. The runtime must still be registered with the
Docker daemon's `daemon.json`.

### Proxy CLI

| Flag | Type | Default | Required |
|---|---|---|---|
| `--name` | proxy name | random | no |
| `--controller-url` | WebSocket URL | — | yes |
| `--cluster` | cluster name | — | yes |
| `--https` | bool | false | no |
| `--http-port` | u16 | 9090 (dev) / 80 (prod) | no |
| `--https-port` | u16 | 443 (prod) | no |
| `--cert-path` | path | — | no |
| `--acme-endpoint` | URL | — | no |
| `--acme-email` | string | — | no |
| `--acme-eab-kid` | string | — | no |
| `--acme-eab-hmac-key` | string | — | no |
| `--root-redirect-url` | URL | — | no |

Source: `plane/src/proxy/command.rs`.

### HTTP API routes

Routes are split between `/ctrl/*` (intended to sit behind an auth
proxy) and `/pub/*` (public).

| Method | Path | Purpose |
|---|---|---|
| GET | `/pub/health` | Public liveness check. |
| GET | `/pub/b/{backend}/status` | Current backend status (no auth). |
| GET | `/pub/b/{backend}/status-stream` | SSE stream of status transitions. |
| GET | `/ctrl/status` | Controller status. |
| GET | `/ctrl/c/{cluster}/state` | Cluster state: `{ drones, proxies }`. |
| GET | `/ctrl/c/{cluster}/drone-socket` | Drone WebSocket endpoint. |
| GET | `/ctrl/c/{cluster}/proxy-socket` | Proxy WebSocket endpoint. |
| GET | `/ctrl/dns-socket` | DNS server WebSocket endpoint. |
| POST | `/ctrl/connect` | Issue a connect request (spawn or attach). |
| POST | `/ctrl/c/{cluster}/d/{drone}/drain` | Drain a drone. |
| POST | `/ctrl/b/{backend}/soft-terminate` | Graceful terminate. |
| POST | `/ctrl/b/{backend}/hard-terminate` | Hard terminate. |
| POST | `/ctrl/b/revoke` | Revoke a connection token. |

Source: `plane/src/controller/mod.rs` (router definition) and
`common/src/lib.rs` (`PlaneClient` URL builders).

### `ConnectRequest` body shape

```jsonc
{
  "key": {                          // optional
    "name": "string",
    "namespace": "string",          // optional, default ""
    "tag": "string"                 // optional, default ""
  },
  "spawn_config": {                 // optional; required if key has no running backend
    "id": "backend-name",           // optional; let Plane assign
    "cluster": "host:port",         // optional; uses controller default
    "pool": "pool-name",            // optional; routes to drones in this pool
    "executable": {                 // serialized DockerExecutorConfig
      "image": "ghcr.io/...",
      "pull_policy": "always" | "if-not-present" | "never",  // optional
      "credentials": {...},                                   // optional
      "env": {"KEY": "value"},                                // optional
      "resource_limits": {...},                               // optional
      "mount": {...},                                         // optional
      "network_name": "string"                                // optional
    },
    "lifetime_limit_seconds": 300,  // optional
    "max_idle_seconds": 60,         // optional
    "use_static_token": false,      // optional
    "subdomain": "string"           // optional
  },
  "user": "string",                 // optional; passed as X-Plane-User
  "auth": {}                        // optional; passed as X-Plane-Auth
}
```

Source: `common/src/types/mod.rs` (`ConnectRequest`, `SpawnConfig`,
`KeyConfig`, `DockerExecutorConfig`).

### `ConnectResponse` shape

```jsonc
{
  "backend_id": "string",
  "spawned": true,
  "status": "scheduled" | "loading" | "starting" | "waiting" | "ready" | "terminating" | "hard-terminating" | "terminated",
  "token": "string",
  "url": "http://cluster/token/",
  "subdomain": "string",            // optional
  "secret_token": "string",         // optional
  "status_url": "http://controller/pub/b/.../status",
  "drone": "drone-name"             // optional
}
```

Source: `common/src/types/mod.rs` (`ConnectResponse`).

### Cluster name format

A cluster name is either:
- `host:port` (dev) — proxy serves HTTP, no ACME.
- `host` (prod) — proxy serves HTTPS, needs ACME setup.

Detection: presence of a colon in the name. Source:
`plane/src/proxy/command.rs` and quickstart docs.

## What's running where

| Component | Where | How |
|---|---|---|
| Postgres | host container | docker compose |
| Plane controller | host container | docker compose |
| Plane proxy | host container | docker compose |
| Pull-through registry | host container | docker compose |
| Plane drone | inside each VM, as a container | systemd unit |
| Sandbox containers | inside each VM, on the VM's Docker | spawned by drone |

The proxy reaches drones at their host-only IPs (192.168.56.10+i). The
drone advertises this IP via `--ip` so the proxy knows where to route.

## Architecture notes

This setup uses upstream Plane unchanged. The
[spec document](../sandbox-platform-spec.md) describes a number of
extensions on top of Plane — extended controller, `agentd`, egress
proxy, snapshot/restore. None of those are present here yet. This
walking skeleton exists to validate:

- Multi-host scheduling (Plane's job).
- Drone lifecycle and registration.
- Image distribution via the pull-through cache.
- Cross-component networking on the host-only network.
- Trust-tier routing via the `--pool` mechanism.

Phase 2+ deliverables that slot in here without restructuring:
- Replace the stock controller image with a fork that speaks our SDK.
- Add `30-egress-proxy.sh` provisioning to install Envoy alongside the drone.
- Build sandbox runtime images with `agentd` baked in.

## Troubleshooting

### `make up` hangs at "Waiting for controller /pub/health"

```bash
make logs
```

Most common causes: Postgres bad volume state (`make nuke && make up`),
port 8080 already in use on the host (`ss -ltn | grep 8080`), or the
controller image failing to pull.

### Drones don't appear in `make cluster-state`

```bash
make ssh-1
sudo journalctl -u plane-drone -n 100 --no-pager
```

Common causes:
- Drone can't reach controller. From inside the VM:
  ```bash
  curl -v http://192.168.56.1:8080/pub/health
  ```
- The drone container exited because of a bad CLI flag. The journal
  will show the Plane error message.
- Cluster name mismatch between controller (`--default-cluster`) and
  drone (`--cluster`). Both must be identical strings.

### Smoke test fails at `/ctrl/connect`

```bash
curl -v -X POST http://127.0.0.1:8080/ctrl/connect \
  -H 'Content-Type: application/json' \
  -d '{"spawn_config":{"executable":{"image":"ghcr.io/jamsocket/demo-image-drop-four"}}}'
```

Look at the response body for a Plane API error code (`NoDroneAvailable`,
`NoClusterProvided`, etc.). Each is documented in
`plane/src/controller/connect.rs`.

### `runsc` runtime not available

```bash
make ssh-2   # an untrusted-tier drone
docker info | grep -i runtime
docker run --rm --runtime=runsc hello-world
```

If runsc is missing, re-run provisioning: `vagrant reload --provision drone-2`.

### Performance is sluggish

- Use linked clones (already enabled in the Vagrantfile).
- Reduce `DRONE_COUNT`.
- Use NVMe storage on the host.
- On Apple Silicon, use Lima or Tart instead — VirtualBox's M-series
  support is still beta.

## File layout

```
sandbox-platform-dev/
├── Vagrantfile               # drone fleet definition
├── Makefile                  # orchestration
├── README.md                 # this file
├── .env.example              # documented config
├── host/
│   └── docker-compose.yml    # control plane services
├── provision/
│   ├── 00-common.sh          # base setup
│   ├── 10-drone.sh           # Plane drone systemd unit
│   └── 20-runsc.sh           # gVisor (untrusted tier only)
└── scripts/
    ├── check-prereqs.sh
    └── smoke-test.sh
```
