# Sandbox Platform

Ephemeral compute platform built on [Plane](https://github.com/jamsocket/plane) + OCI containers with tiered isolation.

## Quick start

```bash
git config core.hooksPath .githooks
make clean
make build
make fleet-images    # chamuka-plane:latest + sandbox-runtime:node24-latest
make fleet-up        # canonical Vagrant fleet (host services + VM drones)
make smoke           # verify end-to-end
make console-install # first time only
make console-dev     # Next.js UI on :3000
```

### Docker fallback

If VirtualBox/Vagrant is unavailable, use the all-Docker fallback fleet:

```bash
make fleet-images
make fleet-up-docker
make smoke-docker
make fleet-down-docker
```

See [dev-fleet/README.md](./dev-fleet/README.md) and [dev-fleet/OPS.md](./dev-fleet/OPS.md) for details.

## Repository layout

```
crates/                   Cargo workspace (flat, all crates at same level)
├── plane/                Hard fork of jamsocket/plane (controller, drone, proxy)
├── plane-common/         Plane's shared library (upstream origin)
├── plane-dynamic-proxy/  Plane's reverse proxy library (upstream origin)
├── agentd/               In-sandbox HTTP daemon
├── common/               Our shared types, extension types, test stubs
├── egress-proxy/         xDS publisher for local Envoy
├── ops-cli/              Operator tooling (sbctl binary)
├── tools/                Schema generation from Rust types
├── plane-tests/          Integration tests (require Docker + Postgres)
└── plane-test-macro/     Proc macro for Plane test harness

sdks/
├── typescript/           @sandbox-platform/sdk
└── python/               sandbox-platform-sdk

runtime-images/           OCI image Dockerfiles
├── node24/               Node 24 + agentd (base sandbox image)
├── agent/                Node 24 + git + jq + curl + full coreutils + agentd
├── ysweet/               Node 24 + y-sweet 0.6.3 + agentd
├── desktop/              X11/VNC + agentd
└── tests/                Smoke tests for each image (shared lib.sh harness)

dev-fleet/                Local cluster (Vagrant canonical, Docker fallback)
├── host/                 Host services for canonical Vagrant fleet
├── docker-compose.yml    Docker-only fallback fleet
├── scripts/              Fleet status, smoke test, prereq checks
└── OPS.md                API reference, troubleshooting, quick-start recipes

console/                  Next.js operator console + ws-relay

contracts/
├── schemas/              JSON Schemas generated from Rust types (CI-checked)
└── fixtures/             Canonical wire examples

docs/                     Design documents
├── sandbox-platform-spec.md        What we build and why
├── sandbox-platform-sad.md         C4-style architecture
├── sandbox-platform-interfaces.md  Every contract surface, byte-level
├── sandbox-platform-build-plan.md  Phased implementation order
└── sandbox-platform-discipline.md  TDD, code rules, source control
```

## Key commands

| Command | Purpose |
|---|---|
| `make ci` | Format check + clippy + tests (no Docker) |
| `make ci-full` | Full CI including Docker integration tests |
| `make fleet-images` | Build images required for local cluster |
| `make fleet-up` | Start canonical Vagrant dev fleet |
| `make fleet-up-docker` | Start Docker-only fallback fleet |
| `make smoke` | End-to-end test (Vagrant fleet) |
| `make console-dev` | Start console (Next.js + ws-relay) |
| `make images` | Build all runtime images |
| `make schemas` | Regenerate JSON schemas from Rust types |
| `./scripts/status.sh` | Current phase, next step, walking-skeleton status |

## Current state

Run `./scripts/status.sh` — it reads the latest commit trailers to show the current phase and next concrete step. See [dev-fleet/OPS.md](./dev-fleet/OPS.md) for operational reference.

## Design documents

All design decisions live in `docs/`. The [discipline doc](./docs/sandbox-platform-discipline.md) governs how code arrives in the tree (TDD layers, code rules, commit discipline, session handoff). It is imported every session via [CLAUDE.md](./CLAUDE.md).

- [Build conversation](https://claude.ai/chat/fc9f54d0-62bc-48d6-a5d0-0c9936e7c1fa) — the Claude Code session that scaffolded this project

## License

Not yet determined.
