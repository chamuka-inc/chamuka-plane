# Sandbox Platform

A Vercel-Sandbox-like ephemeral compute platform built on Plane + OCI containers with tiered isolation.

**Start here:** [CLAUDE.md](./CLAUDE.md) is the entry point for every session. It imports the engineering discipline and points at the four design docs.

## Project state

This workspace contains:

- `CLAUDE.md` — session entry point; imports the discipline doc
- `docs/` — five mutually-consistent design documents
  - `sandbox-platform-spec.md` — what we build and why
  - `sandbox-platform-sad.md` — how it's structured (C4-style)
  - `sandbox-platform-interfaces.md` — every contract surface, byte-level
  - `sandbox-platform-build-plan.md` — Phase D → Phase 0 → Phases 1–9
  - `sandbox-platform-discipline.md` — TDD, code, source control, session handoff (imported every session)
- `dev-fleet/` — Vagrant-based local fleet for live e2e

## Implementation has not started yet

Currently this is design + discipline only. The first implementation session is Phase D.1 (Plane source deep-read) or Phase 0.1 (workspace scaffolding) depending on whether design verification has been done.

See [docs/sandbox-platform-build-plan.md §8](./docs/sandbox-platform-build-plan.md#8-implementation-order) for the full implementation order.

## First commands after starting implementation

```bash
git init
git config core.hooksPath .githooks
# Phase 0.1 will create .githooks/, scripts/status.sh, and the Cargo workspace
```

Until then, `scripts/status.sh` and `.githooks/` do not exist; the next session is responsible for creating them in Phase 0.1.
