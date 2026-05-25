# Sandbox Platform

A Vercel-Sandbox-like ephemeral compute platform built on Plane (jamsocket/plane) + OCI containers with tiered isolation (runc / runsc / Kata + Firecracker).

Solo implementer (me). Five mutually-consistent design docs + git history are the durable substrate; nothing else persists between sessions.

---

## Autonomous operation mode

This project runs autonomously. Execute the build plan phase by phase (Phase 0 → Phase 1 → ... → completion) without waiting for human approval between steps. Follow the discipline doc, TDD layers, and propagation rule as written — they are the guardrails.

**Only ask for human input when:**
- A critical blocker that cannot be resolved from the docs, code, or upstream source (e.g., upstream Plane has a breaking change that invalidates our architecture)
- A design decision not covered by the spec/interfaces/SAD that would be expensive to reverse
- Infrastructure access or credentials are required that you don't have
- The walking skeleton is red for reasons traceable to a fundamental design flaw rather than an implementation bug

**At the end of each coding phase**, run `/simplify` to review changed code for reuse, quality, and efficiency before tagging the phase complete.

**Do not ask for human input for:**
- Routine TDD cycles, refactoring, or phase transitions
- Choosing between implementation alternatives when both satisfy the spec
- Cherry-picking upstream Plane fixes when beneficial (see build plan §9)
- Fixing failing tests or CI — just fix them
- Any decision the design docs already answer

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
5. **Hard fork.** `crates/plane/` is a hard fork of jamsocket/plane. Modify anything freely — no upstream tracking, no ext/ restriction.
6. **End every session with a commit.** That commit's body carries `Phase:` and `Next:` trailers. No separate STATUS.md file.

Full versions of every rule above are in the [discipline doc](./docs/sandbox-platform-discipline.md).

---

## Key paths

- `crates/` — single flat Cargo workspace (all crates at same level)
  - `common/` — our shared types, extension types, test stubs
  - `plane/` — hard fork of jamsocket/plane (controller + drone + proxy binary)
  - `plane-common/` — Plane's shared library (upstream origin)
  - `plane-dynamic-proxy/` — Plane's reverse proxy library (upstream origin)
  - `plane-tests/` — Plane integration tests (require Docker + Postgres)
  - `plane-test-macro/` — proc macro for Plane test harness
  - `agentd/` — in-sandbox HTTP daemon
  - `egress-proxy/` — xDS publisher for local Envoy
  - `ops-cli/` — operator tooling (`sbctl` binary)
  - `tools/generate-schemas/` — emits contracts/schemas/ from common types
  - `.cargo/config.toml` — workspace rustflags (tracing_unstable)
- `contracts/schemas/` — JSON Schemas generated from `common` Rust types; CI fails if stale
- `contracts/fixtures/` — canonical wire examples; the integration surface between Rust and the SDKs
- `sdks/typescript/`, `sdks/python/` — SDKs in consumers' ecosystems
- `runtime-images/` — OCI image Dockerfiles (base, node24, python313, bun1, desktop)
- `dev-fleet/` — Vagrant-based local fleet for live e2e
- `walking-skeleton/` — the project-wide canary test, runs on every PR
- `.githooks/` — pre-commit + pre-push hooks (run `git config core.hooksPath .githooks` after cloning)
- `scripts/status.sh` — print current state for session start

---

## Makefile targets

Use `make <target>` for all build/test/image operations. Run `make help` for the full list. Key targets:

| Target | Purpose |
|---|---|
| `make build` | Build the whole Cargo workspace |
| `make check` | Type-check without codegen (fast) |
| `make test` | Run workspace tests (excludes Docker-dependent) |
| `make test-docker` | Run Docker-dependent integration tests |
| `make ci` | CI pipeline: fmt-check + lint + test |
| `make ci-full` | CI pipeline including Docker integration tests |
| `make images` | Build all runtime images (base, node24, python313, agent, desktop) |
| `make image-base` | Build sandbox-runtime:base-latest |
| `make image-node24` | Build sandbox-runtime:node24-latest |
| `make image-python313` | Build sandbox-runtime:python313-latest |
| `make image-agent` | Build sandbox-runtime:agent-latest |
| `make image-desktop` | Build sandbox-runtime:desktop-latest |
| `make image-plane` | Build chamuka-plane:latest |
| `make fleet-images` | Build images required for local cluster |
| `make fleet-up` | Start canonical Vagrant dev fleet |
| `make fleet-up-docker` | Start Docker-only fallback fleet |
| `make fleet-down` | Stop Vagrant drones |
| `make fleet-nuke` | Destroy Vagrant fleet (VMs + host services + volumes) |
| `make smoke` | End-to-end smoke test (Vagrant fleet) |
| `make smoke-docker` | End-to-end smoke test (Docker fallback) |
| `make schemas` | Regenerate JSON schemas from Rust types |
| `make console-install` | Install console npm dependencies |
| `make console-dev` | Start console dev server (Next.js + ws-relay) |
| `make console-test` | Run console test suite |

**Always use Makefile targets instead of calling build scripts directly.** When adding new images or build steps, add a corresponding Makefile target.
