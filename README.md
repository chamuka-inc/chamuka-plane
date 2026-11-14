# Chamuka Plane

[![CI](https://github.com/chamuka-labs/chamuka-plane/actions/workflows/rust-workspace.yml/badge.svg)](https://github.com/chamuka-labs/chamuka-plane/actions/workflows/rust-workspace.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Ephemeral compute platform built on Plane and OCI containers, with tiered isolation.

This repository is a hard fork of [jamsocket/plane](https://github.com/jamsocket/plane).

## Quick start

```bash
git config core.hooksPath .githooks
make build                              # build the Rust workspace
cd dev-fleet && docker compose up -d    # start the local fleet
./dev-fleet/scripts/smoke-test.sh       # verify everything works
```

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

dev-fleet/                Docker Compose multi-drone fleet
├── docker-compose.yml    Controller, proxy, 3 drones, Postgres, registry
├── scripts/              Fleet status, smoke test, prereq checks
└── OPS.md                API reference, troubleshooting, quick-start recipes

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
| `make images` | Build all runtime images |
| `make schemas` | Regenerate JSON schemas from Rust types |
| `./scripts/status.sh` | Current phase, next step, walking-skeleton status |

## Current state

Run `./scripts/status.sh` — it reads the latest commit trailers to show the current phase and next concrete step. See [dev-fleet/OPS.md](./dev-fleet/OPS.md) for operational reference.

## Design documents

All design decisions live in `docs/`. The [discipline doc](./docs/sandbox-platform-discipline.md) governs how code arrives in the tree (TDD layers, code rules, commit discipline, session handoff). It is imported every session via [CLAUDE.md](./CLAUDE.md).

## Contributing

Issues and pull requests are welcome.

## Security

Report vulnerabilities to [security@chamuka.ai](mailto:security@chamuka.ai).

## License

Chamuka's original work in this repository is licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).

This repository is a hard fork of [jamsocket/plane](https://github.com/jamsocket/plane). The crates `plane`, `plane-common`, and `plane-dynamic-proxy` (and the accompanying `plane-tests` and `plane-test-macro` crates) derive from that upstream MIT-licensed project (Copyright (c) 2023 Drifting in Space Corp., plus the included hyperium/hyper-util notice). The full upstream MIT text is preserved in [LICENSE-MIT-upstream](LICENSE-MIT-upstream). Those forked crates also contain Chamuka modifications licensed under Apache-2.0 (`MIT AND Apache-2.0`).

---

Part of [Chamuka Labs](https://chamuka.ai), an applied AI lab.
