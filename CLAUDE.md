# Sandbox Platform

A Vercel-Sandbox-like ephemeral compute platform built on Plane (jamsocket/plane) + OCI containers with tiered isolation (runc / runsc / Kata + Firecracker).

Solo implementer (me). Five mutually-consistent design docs + git history are the durable substrate; nothing else persists between sessions.

---

## Engineering discipline (imported every session)

The rules below govern how code arrives in this tree. TDD, code quality, source control, session handoff. Read every session, follow without exception.

@./docs/sandbox-platform-discipline.md

---

## Project documents

- [Build plan](./docs/sandbox-platform-build-plan.md) — Phase D (discovery) → Phase 0 (test infrastructure) → TDD per binary
- [Spec](./docs/sandbox-platform-spec.md) — what we build and why
- [SAD](./docs/sandbox-platform-sad.md) — how it's structured (C4-style, with verified Plane message types)
- [Interfaces](./docs/sandbox-platform-interfaces.md) — every contract surface, byte-level, with Postgres schema

---

## First command every session

```bash
./scripts/status.sh
```

Shows current Phase, Next concrete step, walking-skeleton status, and recent phase tags. The session's resumption point is in the most recent commit's `Phase:` and `Next:` trailers (see [discipline doc §4](./docs/sandbox-platform-discipline.md#4-session-handoff-discipline)).

If `scripts/status.sh` doesn't exist yet, we're before Phase 0.1 (workspace scaffolding) and the first session creates it.

---

## Hard rules (the short version)

1. **Phase D before Phase 0.** Don't start scaffolding until the design has been verified by running, not just by reading. See [build plan §1](./docs/sandbox-platform-build-plan.md#1-build-philosophy) Rule 1.
2. **Red before green.** Every feature begins with a failing test. No exceptions.
3. **Propagation rule.** interfaces.md → Rust type → schema → fixture → test → code. Never code-first.
4. **One source of truth.** Types live in `common`. Schemas are generated from types. Fixtures are pinned wire examples. If two of these disagree, fix all of them — never paper over.
5. **`ext/` only.** Inside `crates/plane/`, modifications happen only in `plane/src/ext/` and date-prefixed migrations. Anywhere else is a rebase landmine.
6. **End every session with a commit.** That commit's body carries `Phase:` and `Next:` trailers. No separate STATUS.md file.

Full versions of every rule above are in the [discipline doc](./docs/sandbox-platform-discipline.md).

---

## Key paths

- `crates/` — single Cargo workspace for all Rust binaries (common, plane, agentd, egress-proxy, ops-cli, tools/generate-schemas)
- `contracts/schemas/` — JSON Schemas generated from `common` Rust types; CI fails if stale
- `contracts/fixtures/` — canonical wire examples; the integration surface between Rust and the SDKs
- `sdks/typescript/`, `sdks/python/` — SDKs in consumers' ecosystems
- `runtime-images/` — OCI image Dockerfiles (base, node24, python313, bun1, desktop)
- `dev-fleet/` — Vagrant-based local fleet for live e2e
- `walking-skeleton/` — the project-wide canary test, runs on every PR
- `.githooks/` — pre-commit + pre-push hooks (run `git config core.hooksPath .githooks` after cloning)
- `scripts/status.sh` — print current state for session start
