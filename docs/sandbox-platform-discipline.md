# Sandbox Platform — Engineering Discipline

**Imported in every session via [CLAUDE.md](../CLAUDE.md).** These rules apply to every code change regardless of which binary or phase I'm working on. For project-specific structure (binaries, phases, repo layout), see the [build plan](./sandbox-platform-build-plan.md).

Three reasons this document exists separately from the build plan:

1. **It's cross-cutting.** TDD, code shape, source control, and session handoff apply to every line I write. They're not phase-specific.
2. **It belongs in every session's context.** The build plan tells me *what* to build next; this tells me *how* to build anything.
3. **It's stable.** The build plan evolves as phases complete; these rules don't change.

---

## Table of contents

1. [Testing strategy — the TDD discipline](#1-testing-strategy--the-tdd-discipline)
2. [Code discipline — what's forbidden](#2-code-discipline--whats-forbidden)
3. [Source control discipline](#3-source-control-discipline)
4. [Session-handoff discipline](#4-session-handoff-discipline)
5. [Anti-patterns to avoid](#5-anti-patterns-to-avoid)

---

## 1. Testing strategy — the TDD discipline

This section is the contract between me and myself: this is how new code arrives in the tree. There are no exceptions, including for "small" changes, because small changes are exactly what TDD catches early.

### 1.1 The four layers

```
                  Layer 4: Live e2e against contracts
                  ▲     real Plane, real Docker, real Envoy, real Postgres
                  │     fixtures sent as actual wire bytes; assertions match real behavior
                  │
                  Layer 3: e2e against stubs
                  ▲     full binary running, but counterpart components
                  │     replaced by common::stubs::* (MockDrone, MockController, …)
                  │     fast, deterministic, runnable on a laptop in seconds
                  │
                  Layer 2: Component / integration tests
                  ▲     larger units inside one binary; uses contracts/fixtures
                  │     e.g., "controller handles this fixture, writes expected DB row"
                  │
                  Layer 1: Unit tests (TDD inner loop)
                        red → green-impl, one small test at a time
                        runs in milliseconds; no I/O; no fixtures
```

Each layer catches different bugs. Skipping any layer means those bugs land in production.

### 1.2 The inner loop (Layer 1)

The classic TDD cycle, applied per feature, per public method, per branch of logic.

```
1. Write the smallest test that captures the next bit of desired behavior.
2. Run it. Confirm it fails for the right reason (red).
3. Write the minimum production code that makes it pass (green).
4. Refactor — both test and production code — keeping green.
5. Commit.
6. Repeat.
```

The commit at the end of each cycle is small (often <50 lines). Multiple cycles compose into a feature. The session ends at a checkpoint where the tree is green.

### 1.3 Contract tests (Layer 2)

For every type defined in [interfaces.md §14](./sandbox-platform-interfaces.md#14-shared-type-reference) and every endpoint in interfaces §3, §4, §10, there's at least one contract test per binary that touches it. The contract test:

- Loads a fixture from `contracts/fixtures/`.
- Asserts that the binary produces output matching another fixture, or consumes input from a fixture and updates state correctly.
- Runs without network, without Docker, without other binaries.

This is the layer where my interfaces doc gets validated against my implementation. If a fixture and a binary disagree, one of them is wrong; I pick one truth and fix the other.

### 1.4 e2e against stubs (Layer 3)

Run the full binary, but every counterpart is a stub from `common::stubs`. For the controller, this means `MockDrone` and `MockProxy`. For the drone, `MockController` and a mock Docker socket.

The point: exercise the binary's actual main loop, async runtime, real serialization — but without the operational burden of bringing up real Plane, Docker, Envoy. Layer 3 tests run in seconds and are deterministic.

### 1.5 Live e2e against contracts (Layer 4)

The truth check. Run the binary against real systems (real Plane image, real Docker daemon, real Envoy, real Postgres) and exercise it with contract fixtures as the input or comparison material.

Examples:

- **Plane fork (controller).** Run our forked controller and stock Plane controller side-by-side against the same fixtures. Compare responses. Differences are bugs in our fork (or intentional extensions we mark in a baseline file).
- **Plane fork (drone).** Run our drone connected to real upstream Plane controller. Send fixture-derived spawn actions. Verify a real container actually starts and reports back over the real wire protocol.
- **agentd.** Run agentd in a real container in the dev fleet. Hit it with HTTP from outside. Compare responses to fixtures.
- **egress-proxy.** Stand up real Envoy. Push fixture-derived policies. Send real curl traffic from inside a sandbox netns. Verify Envoy's behavior matches.

The dev-fleet smoke canary ([build plan §6](./sandbox-platform-build-plan.md#6-dev-fleet-smoke-canary)) is the maintained project-wide Layer 4 path.

### 1.6 What "live e2e against contracts" prevents

This is the layer that would have caught my own mistakes in earlier doc revisions. The interfaces doc claimed `MessageToDrone::Spawn` was a top-level command; reading Plane source showed it's actually `MessageToDrone::Action(BackendActionMessage)` with `Spawn` nested. If I had a live e2e test sending a `Spawn` to real Plane upstream, it would have failed on day one and I'd have learned the right shape.

Concretely, live e2e tests do one of:

1. **Compatibility check** — Send a fixture against real upstream; verify it accepts (for inputs we send) or matches (for outputs we expect). This validates the fixture itself.
2. **Behavior check** — Run our binary in the system; verify side effects (DB rows, container state, network calls) match what the contract says should happen.

When a live e2e fails, the order of suspicion is: (a) fixture is wrong, (b) our binary is wrong, (c) the upstream system changed underneath us. In that order, because that's how often each is the cause.

### 1.7 Cadence and ownership

| Layer | Runs on every commit? | Runs in CI? | Time budget |
|---|---|---|---|
| 1 Unit | Yes (locally) | Yes | < 5 s per binary |
| 2 Contract | Yes (locally) | Yes | < 30 s per binary |
| 3 e2e against stubs | Often (locally) | Yes | < 2 min per binary |
| 4 Live e2e | Sometimes (locally; expensive) | Yes, with infra | < 10 min per binary |

The `agentd-smoke` job runs on every PR. Per-binary Layer 4 tests run on every PR that touches that binary where CI coverage exists; otherwise use `make smoke` or `make smoke-docker` locally.

### 1.8 What this means for new features

For a new endpoint, new message variant, or new behavior, the order is:

```
1. Update interfaces.md   (the spec change)
2. Update the Rust type in common::ext   (the implementation contract)
3. cargo run --bin generate-schemas   (the JSON Schema)
4. Regenerate SDK types from schemas   (the SDK contract)
     TS:  cargo run --bin generate-sdk-types -- --lang ts
     Py:  cargo run --bin generate-sdk-types -- --lang py
5. Add or update fixtures in contracts/fixtures/   (the wire examples)
6. Write a red Layer 2 contract test   (the verification target)
7. TDD inner loop: red → green-impl   (the code)
8. Add a Layer 3 e2e test with stubs   (the integration)
9. Add a Layer 4 live e2e test where it makes sense   (the reality check)
10. Commit with `Phase:` and `Next:` trailers (the handoff)
```

The full propagation chain is:

```
interfaces.md → Rust type → JSON Schema → SDK types (TS + Py) → fixtures → test → code
```

Steps 3 and 4 are mechanically enforced: CI fails if schemas or SDK types are stale relative to Rust types. The `generated/` directory in each SDK is truly generated — never hand-edited. If a generated type doesn't fit the SDK's ergonomic needs, the fix goes in the generator, not in the output.

Skipping any step is technical debt that compounds. Especially: do not write code in step 7 without having a red test in step 6 first.

---

## 2. Code discipline — what's forbidden

The TDD discipline (§1) says how code arrives in the tree. This section says what shape it must be in. Both are gates: passing tests doesn't ship if the code violates these rules. Most are enforced by clippy in CI; the rest fall to self-review with flagging in the session-ending commit body (see §4).

### 2.1 Rust prohibitions

**No `unwrap()` or `expect()` in production code.** Production code returns `Result` and propagates with `?`. Exceptions, each with a comment: `Mutex::lock()` on a poisoned mutex (unrecoverable), `OnceCell::get` after explicit initialization, startup errors in `main()` that should crash. Test code may use `expect("descriptive reason")` because test failures are loud.

**No `clone()` to dodge the borrow checker.** Cloning a `String` when `&str` works, cloning a `Vec` when an iterator works — each is a small smell. Reach for borrows first. Cheap clones (`Arc::clone`, `Copy` types) and clones that genuinely encode fan-out are fine.

**No `unsafe` without a `// SAFETY:` comment.** Every `unsafe` block documents why the invariants hold. Without the comment, it doesn't compile (clippy `undocumented_unsafe_blocks` set to deny).

**No stringly-typed APIs.** Finite sets are enums; structured data is a struct. `fn handle(kind: &str)` with a match on `"spawn" | "terminate"` is wrong; the parameter is the enum.

**No primitive obsession on IDs.** `BackendName`, `DroneName`, `SandboxId`, etc. are distinct newtype wrappers. Functions take `&BackendName`, not `&str`. The type-checker catches "I passed a tenant ID where a backend ID was expected" at compile time. Plane already does this; we follow.

**No `Box<dyn Error>` as the error type.** Each crate defines its own error enum using `thiserror`. The error type encodes the actual failure modes; downstream code matches on variants. `Box<dyn Error>` defers the error design and hides bugs.

**No holding locks across `.await`.** Take the lock, read or write, drop, then await. Use `tokio::sync::Mutex` only when the lock genuinely must span an `.await`, and that should be rare. Clippy's `await_holding_lock` set to deny.

**No silent error swallowing.** `let _ = something_returning_result();` is forbidden without a `// intentionally ignored: <reason>` comment. Same for `if let Err(_) = ... {}` with an empty block.

**No god modules.** A single `.rs` file or `mod.rs` over ~500 lines is a smell. Split it. Exceptions for generated code, marked explicitly.

**No premature abstraction.** A trait with one implementation is suspect. Wait until there are two real users. Exception: `common::stubs` exists exactly to enable two implementations (real and mock); traits there are correct on day one.

**No premature optimization.** `Arc` everywhere, `RwLock` where `Mutex` would do, custom allocators, `unsafe` for performance — none of it lands until a profile says it has to.

### 2.2 General code smells

**No god functions.** A function over ~50 lines is a smell. A function with more than five parameters is a smell (use a struct). Nesting deeper than three levels is a smell.

**No magic numbers.** `Duration::from_secs(180)` is fine only if 180 has a documented name (e.g. it's Plane's heartbeat interval); otherwise `const HEARTBEAT_INTERVAL_SECS: u64 = 180;` with the source of the value in a comment.

**No dead code, no commented-out code.** Delete it. Git has the history.

**No TODO/FIXME without an issue reference.** Format: `// TODO(#123): description`. Untracked TODOs are promises we forget.

**No misleading names.** `data`, `info`, `manager`, `helper`, `process_thing()` — these are placeholders, not names. The name reflects what the thing represents.

**No comments that say *what*.** Comments explain *why*, not *what*. `// increment counter` over `counter += 1` is noise; `// rate-limit reset boundary` is signal. If the *what* isn't obvious from the code, the code is wrong; rewrite it.

**No duplicated code across modules.** Two functions doing the same thing in different files will drift. Extract.

### 2.3 Project-specific prohibitions

**No bypassing `common`.** If you need a `BackendName`, you import it from `common`. You do not define a local one. Two definitions = future drift.

**No hand-rolled JSON.** Derive macros only. `serde_json::Value` for genuinely ad-hoc cases is acceptable; manual `Serialize`/`Deserialize` implementations need explicit justification in a comment.

**No test fixtures generated in test code.** Fixtures come from `contracts/fixtures/` via `load_fixture()`. Test code that builds expected JSON inline by hand is fixture drift waiting to happen.

**No direct Postgres outside the database layer.** Each crate has at most one module that holds `sqlx` queries. Business logic doesn't reach into the database from arbitrary places.

**No `tokio::spawn` without managing the `JoinHandle`.** Fire-and-forget tasks leak. Either await the handle, store it for shutdown, or use a `JoinSet`.

**No assuming exactly-once delivery.** Plane is at-least-once. Every action handler is idempotent by `action_id` (or our `extension_action.id`). Processing an action without first checking "have I seen this `action_id`?" is wrong.

**No request_id correlation when Plane uses action_id.** I made this mistake in an early doc revision. The pattern is durable queue + per-message id + ack. We follow it; we don't invent a new correlation scheme.

**No bypassing the propagation rule.** Adding an optional field "just for testing" goes through interfaces.md → type → schema → SDK types → fixture → test → code, like everything else.

**No hand-written SDK types in `generated/` directories.** Files under `sdks/typescript/src/generated/` and `sdks/python/src/sandbox_platform/generated/` are produced by `generate-sdk-types` from `contracts/schemas/`. Editing them by hand is how drift starts — the edit looks correct today and silently diverges from the contract tomorrow. If a generated type has the wrong shape, the bug is in one of: (a) interfaces.md, (b) the Rust type, (c) the generator. Fix the source; regenerate.

**No SDK-only types that shadow contract types.** If the SDK needs a type that represents a contract surface (request, response, entity), that type comes from the generator. SDK-specific convenience types (e.g., options bags, builder patterns) are fine as long as they delegate to generated types internally, never redefine the wire shape.

**No legacy API paths in public SDK surface.** The SDKs expose one creation path: `SandboxClient` → `/pub/v1/sandboxes`. Internal Plane endpoints (`/ctrl/connect`, `/ctrl/status`) are not part of the SDK contract and must not be reachable from public SDK methods. If legacy paths exist for migration, they are `@deprecated`/`@internal` with a removal date.

**Boy-scout the Plane fork.** `crates/plane/` is a hard fork — modify any file freely. Apply the boy-scout principle: leave code cleaner than you found it (fix warnings, improve naming, simplify logic) whenever you're working in an area.

### 2.4 Process prohibitions

**No skipping clippy or rustfmt.** CI runs `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check`. Local pre-commit should run both.

**No merge without green CI.** All workflows green: rust-workspace, contracts, agentd-smoke, and any relevant per-binary live e2e.

**No `#[allow(...)]` without justification comment.** Same shape as `unsafe`: a comment explaining why this lint is being silenced.

**No PRs that change types without regenerating schemas and SDK types.** The contracts workflow catches this; the discipline is to run `cargo run --bin generate-schemas && cargo run --bin generate-sdk-types` before pushing, not let CI tell me. A type change that updates schemas but not SDK types (or vice versa) is an incomplete propagation.

### 2.5 Enforcement

Three layers.

**Layer 1 — Workspace lints.** In `crates/Cargo.toml`:

```toml
[workspace.lints.rust]
unused_imports = "deny"
dead_code = "deny"
unreachable_code = "deny"

[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
todo = "deny"
unimplemented = "deny"
dbg_macro = "deny"
print_stdout = "deny"
print_stderr = "deny"
undocumented_unsafe_blocks = "deny"
await_holding_lock = "deny"
await_holding_refcell_ref = "deny"
clone_on_ref_ptr = "warn"
inefficient_to_string = "warn"
needless_collect = "warn"
manual_async_fn = "warn"
```

Each binary crate inherits via `lints.workspace = true`.

**Layer 2 — CI.** The rust-workspace workflow runs:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features --exclude plane-tests
```

Any violation fails the build.

**Layer 3 — Self-review.** Things clippy can't catch: naming, magic numbers, god modules, comments that say *what*, duplicated logic across crates, premature abstraction. When I notice these post-commit, I flag them in the next commit's `Next:` trailer and fix them before the session ends. If I catch them in the same session, fix immediately.

When in doubt, the rules are upstream of cleverness. A "clever" exception that breaks a rule is almost always wrong on a longer timeline.

---

## 3. Source control discipline

Git is the audit trail. For solo TDD work it matters more than for a team: the commit history is the only record of how decisions accumulated, what was tried and reverted, and which changes paired with which test additions. A messy history means future-me has no way to understand why current-me did something.

### 3.1 Branch model

**Trunk-based with two long-running branches** (only in `crates/plane/`).

- `main` is the only long-lived branch.
- Feature branches are short-lived (one session, sometimes two). They merge into `main` via fast-forward or rebase-merge, never via merge commits. Once merged, they're deleted.
- `main` always has the maintained smoke canary green. Always.
- `crates/plane/` is a hard fork — no upstream-tracking branch, no rebase cadence. See [build plan §9](./sandbox-platform-build-plan.md#9-fork-management).

**No long-running feature branches.** A branch alive for more than a week is a smell. Either it should have merged a long time ago, or it's actually a different thing trying to be a single change.

### 3.2 Commit discipline

**Commits are TDD-aligned.** The natural rhythm produces this commit pattern per feature:

```
agentd: red: failing test for PTY resize handler
agentd: green: implement PTY resize
agentd: refactor: extract resize logic into pty::session module
```

Three commits, three logical steps. `git log --oneline` reads as the history of how the code came to be.

Variations are fine:
- A small feature might be one `red:` and one `green:` (no refactor).
- A bug fix has the same pattern: red test reproducing the bug, then the fix.
- Pure refactors land as `refactor:` commits with no red predecessor — they're behavior-preserving, so tests stay green throughout.

**Atomicity.** Each commit is a logical unit that:
- Compiles on its own (except the deliberate `red:` commits, which must compile but have failing tests).
- Doesn't break unrelated tests.
- Can be reverted without cascading failures.

**Frequency.** A session producing one commit is a smell. Five to fifteen is normal. The instinct to "I'll squash this all together at the end" loses the rhythm of TDD in the history.

**The session-ending commit.** Every session ends with a commit. That commit's body carries the session-handoff state via the `Phase:` and `Next:` trailers (see §4). If the session ended mid-task, the trailers tell the next session exactly where to resume.

### 3.3 Commit message format

```
<scope>: <type>: <imperative summary, ≤72 chars>

<optional body explaining WHY, not what>
<wrap at 72 cols>

<optional footer with issue refs or breaking-change notes>
```

**Scopes** (the binary or area being touched):
- `agentd`, `plane`, `common`, `egress-proxy`, `ops-cli`
- `sdk-ts`, `sdk-py`
- `contracts` (schemas + fixtures), `docs`, `ci`, `dev-fleet`
- `workspace` (Cargo.toml, top-level Makefile, .gitignore, .githooks)

**Types** (the kind of change):
- `red` — adding a failing test (TDD).
- `green` — making a red test pass (TDD).
- `refactor` — behavior-preserving cleanup.
- `feat` — new functionality where the test was added in a same-PR earlier commit, or where the change is too small to split.
- `fix` — bug fix (usually paired with a `red:` commit reproducing it).
- `chore` — dependency bumps, formatting, build infra.
- `docs` — documentation only.
- `revert` — reverts an earlier commit.

**Examples.**

```
agentd: red: failing test for /v1/fs/watch backpressure

The endpoint should drop frames with a BACKPRESSURE marker
when the consumer can't keep up beyond 1 MB buffered. Current
behavior is to OOM.
```

```
plane: green: implement /pub/v1/sandboxes POST handler

Builds Plane ConnectRequest from CreateSandboxRequest per
interfaces.md §3.2. Idempotency key cached for 24h.

Phase: 3.2
```

```
contracts: regenerate schemas for PushNetworkPolicyMessage

Triggered by adding the `version` field on the message type
in commit abcd1234.
```

**What doesn't go in commit messages:**
- "WIP" or "checkpoint" — every commit should be a coherent step.
- "Updates" or "fixes" without scope.
- Long descriptions of what code changed (`git diff` shows that).

### 3.4 What never gets committed

**Never, under any circumstance:**

- Secrets, API keys, tokens, credentials, passwords. Not in code, not in tests, not in fixtures, not in `.env` files, not in commit messages, not even in deleted files (git history retains them). If a secret was ever committed, treat it as compromised: rotate it immediately, and use `git filter-repo` or BFG to scrub history before pushing further.
- `.env`, `.env.local`, `*.pem`, `*.key`, `id_rsa*`, `*.kubeconfig`, AWS credential files.
- Personal data, customer data, real sandbox content.
- Generated artifacts that can be reproduced: `target/`, `node_modules/`, `dist/`, `__pycache__/`, `.venv/`.
- IDE-specific config: `.idea/`, `.vscode/settings.json` (workspace recommendations file is OK).
- OS junk: `.DS_Store`, `Thumbs.db`.
- Large binaries. If genuinely needed, use git-lfs (configured per-extension in `.gitattributes`).

**Generated-but-committed exceptions:**
- `Cargo.lock` — committed for binary crates (this whole workspace is binaries).
- `contracts/schemas/*.schema.json` — generated from Rust types but committed; CI fails if they're stale (§3.8).
- `sdks/typescript/src/generated/types.ts` and `sdks/python/src/sandbox_platform/generated/`— generated from schemas but committed for the same reason.

### 3.5 .gitignore

Comprehensive from day one. The full text lives at the repo root:

```gitignore
# Rust
target/
**/*.rs.bk
*.pdb

# Node / TypeScript
node_modules/
dist/
build/
*.tsbuildinfo
.pnpm-store/
.npm/

# Python
__pycache__/
*.py[cod]
*$py.class
.venv/
venv/
.pytest_cache/
.ruff_cache/
.mypy_cache/
*.egg-info/
.tox/

# Secrets / env
.env
.env.local
.env.*.local
*.pem
*.key
*.crt
*.kubeconfig
secrets/

# OS
.DS_Store
Thumbs.db
desktop.ini

# Editor
.idea/
.vscode/*
!.vscode/extensions.json
*.swp
*.swo
*~

# Vagrant / dev-fleet
.vagrant/
*.box

# Logs
*.log
logs/

# Coverage / profiling
*.profraw
*.profdata
coverage/
lcov.info

# Misc local
.local/
tmp/
```

When a tool generates a new directory or file pattern, add it to `.gitignore` in the same commit, not later.

### 3.6 Pre-commit hooks

Hooks live in `.githooks/` and are version-controlled. After cloning, the first command run is:

```bash
git config core.hooksPath .githooks
```

`.githooks/pre-commit`:

```bash
#!/usr/bin/env bash
set -euo pipefail

# Format check (fast, runs in milliseconds for unchanged files)
cargo fmt --all -- --check

# Lint (only against staged files where possible; clippy isn't naturally
# incremental but it's still under 10s on this workspace)
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Schemas in sync with Rust types
cargo run --quiet --bin generate-schemas
if ! git diff --quiet contracts/schemas/; then
    echo "ERROR: contracts/schemas/ is stale. Run 'cargo run --bin generate-schemas' and commit."
    exit 1
fi

# SDK types in sync with schemas
cargo run --quiet --bin generate-sdk-types
if ! git diff --quiet sdks/typescript/src/generated/ sdks/python/src/sandbox_platform/generated/; then
    echo "ERROR: SDK generated types are stale. Run 'cargo run --bin generate-sdk-types' and commit."
    exit 1
fi

# Basic secret scan (very rough; primary defense is .gitignore + discipline)
if git diff --cached | grep -E '(AKIA[0-9A-Z]{16}|-----BEGIN [A-Z ]+ PRIVATE KEY-----)' > /dev/null; then
    echo "ERROR: Possible secret detected in staged changes."
    exit 1
fi
```

`.githooks/pre-push`:

```bash
#!/usr/bin/env bash
set -euo pipefail

# Full test suite before any push
cargo test --workspace --all-features
```

The hooks are advisory in spirit (CI is the source of truth), but they catch problems before push and save a CI cycle.

### 3.7 Tags and releases

**Semantic versioning per binary**, tagged with binary prefix:

- `agentd-v0.1.0`
- `plane-v0.1.0` (covers all three modes — controller, drone, proxy — since they share a binary)
- `egress-proxy-v0.1.0`
- `ops-cli-v0.1.0`
- `sdk-ts-v0.1.0`
- `sdk-py-v0.1.0`
- `runtime-images-v0.1.0` (covers all image variants for a release)

**Tag discipline.** A tag is created when CI is green on `main` and I've decided this commit is release-worthy. Tagging triggers the release workflow: build artifacts, push images, publish packages.

**Tags are immutable.** Never re-tag. If a release was wrong, cut a new patch version (`v0.1.1`) with the fix.

**Version bumping rules:**
- Breaking change to a public contract → major bump.
- New feature, backward-compatible → minor.
- Bug fix only → patch.

For pre-1.0 (which we'll be in for a long time), the rules apply with one shift: breaking changes are minor, features and fixes are patch. Don't go to 1.0 until the platform is genuinely stable.

### 3.8 Self-PR workflow (recommended, not required)

Solo doesn't mean skipping the audit trail. For any non-trivial change (more than a few commits, or anything touching contracts, schemas, or fork code), push to a branch and open a self-PR. Then:

1. Read the diff on the GitHub web UI. The web view catches things the terminal doesn't — naming inconsistency across files, misplaced new files, accidental whitespace changes.
2. CI runs against the PR; the `agentd-smoke` job's result is on record.
3. Merge via "Squash and merge" only if the branch has chatty commits that aren't worth keeping (mostly `wip:` or `try:` ones, which shouldn't have happened in the first place). Default: "Rebase and merge" to preserve the TDD commit rhythm.
4. Delete the branch immediately after merge.

For tiny changes (a typo fix in a doc, a one-line README), direct commit to `main` is fine.

### 3.9 Conflict resolution

Conflicts are rare in solo work — they may happen when cherry-picking upstream Plane fixes ([build plan §9](./sandbox-platform-build-plan.md#9-fork-management)). When one appears:

1. **Understand both sides.** Read the upstream change and our local change in full. Don't just pick one.
2. **Resolve in the smallest diff possible.** Often the right answer combines both.
3. **Add a regression test if applicable.** A conflict that resolves with non-obvious semantics gets a test that pins the resolved behavior.

### 3.10 Enforcement

Three layers, parallel to §2.5:

**Layer 1 — Pre-commit hooks.** `.githooks/pre-commit` runs fmt, clippy, schema-sync, and basic secret scan. Bypassable with `--no-verify` only in emergencies (and the next commit fixes whatever the bypass was for).

**Layer 2 — CI.** Branch protection on `main`:
- Required status checks: rust-workspace, contracts, agentd-smoke.
- Linear history required (no merge commits).
- No force-pushes to `main`.
- Even self-merges go through PR if the change is non-trivial.
- The `sdk-types` check regenerates SDK types from schemas and fails if the output differs from what's committed. This closes the Rust → schema → SDK type pipeline end-to-end.

**Layer 3 — Self-review.** Things tooling can't catch:
- Commit message quality: subject is imperative, body explains *why*.
- Logical atomicity: does this commit do one thing?
- TDD alignment: are red and green in separate commits?
- Caught post-commit: flag in the next commit's `Next:` trailer; amend or follow-up commit before session ends.

When in doubt, the rules are upstream of cleverness. A single bad commit is fine; a habit of bad commits silently ruins the history's value over months.

---

## 4. Session-handoff discipline

Source control is the status tracker. Every session ends with a commit; that commit's body carries the same information a `STATUS.md` file would have. The handoff state can't drift from the tree because it IS the tree.

### 4.1 Commit-message trailers

Standard Git trailer format. After the body, separated by a blank line:

```
<scope>: <type>: <imperative summary>

<body explaining why, not what>

Phase: <implementation-order number from build plan §8>
Next: <concrete next step; multi-line OK with continuation indent>
Blocked: <optional; only if something genuinely prevents progress>
```

**`Phase:`** — the position in [build plan §8](./sandbox-platform-build-plan.md#8-implementation-order) the commit belongs to. Format: `<top-level>.<sub>` or `<top-level>.<sub>-<phase-letter>` (e.g., `Phase: 7.3-B` for controller Phase B work).

**`Next:`** — the very next concrete action. Equivalent to what a STATUS.md would have under "Next concrete step." Indent continuation lines:

```
Next: Read interfaces.md §3.2 (CreateSandboxRequest -> ConnectRequest
      mapping). Implement build_connect_request() in api_v1.rs.
      Run: cargo test -p plane --test api_v1 -- create_sandbox_happy_path
      until green. Then run the relevant smoke target; should pass.
```

**`Blocked:`** — only present if something genuinely prevents the next step (e.g., upstream dependency is broken in a way I don't yet know how to fix). A TDD `red:` commit is not blocked — it's the expected mid-cycle state where the next commit makes it green.

### 4.2 Reading current state

Single command:

```bash
git log -1 --pretty=format:"%B" | git interpret-trailers --parse --only-trailers
```

Or, the helper at `scripts/status.sh`:

```bash
$ ./scripts/status.sh
HEAD: abc1234 — plane: red: failing test for /pub/v1/sandboxes happy path
Phase: 7.3-B
Next: Read interfaces.md §3.2 (CreateSandboxRequest -> ConnectRequest
      mapping). Implement build_connect_request() in api_v1.rs.
      Run: cargo test -p plane --test api_v1 -- create_sandbox_happy_path
      until green.
Tree state: clean
Recent phase tags:
  phase-0.8-done    (3 days ago)
  phase-6.1-done    (yesterday)
  phase-7.1-done    (this morning)
```

The script is the first thing I run at session start.

### 4.3 Phase-completion tags

When an implementation-order item ([build plan §8](./sandbox-platform-build-plan.md#8-implementation-order)) completes, tag the commit with an annotated tag:

```bash
git tag -a phase-7.1-done -m "Phase 7.1 complete: agentd-headless

All four layers green:
  Layer 1: 47 unit tests, 84% coverage
  Layer 2: contract tests for all 11 endpoints
  Layer 3: e2e stubbed pass
  Layer 4: smoke canary green

Cold start: 87ms p95 (target 100ms).
Binary size: 12.4 MB stripped (target 15 MB).

Next phase: 7.2 sandbox-runtime images."
```

`git tag --sort=creatordate -l "phase-*"` shows the progression of completed phases. These tags are the milestones; `scripts/status.sh` lists them.

Tags are immutable. If a phase later turns out incomplete, do not re-tag — instead, the next session opens with a `red:` commit reproducing the missing behavior and proceeds normally. The git history will show "phase X.Y-done tag, then later red commit reopening it" which is the right shape: the original phase was claimed done, reality disagreed, here's the proof.

### 4.4 End-of-session rules

Every session ends with:

1. **A commit.** Always. Even just a doc tweak.
2. **`Phase:` and `Next:` trailers populated.** `Blocked:` only if applicable.
3. **`git push`** — or at minimum, local main at a known state with no uncommitted changes.
4. **Smoke run** if runtime behavior changed. If it broke, the commit body says so explicitly and `Next:` starts with "fix smoke before anything else."
5. **Phase-completion tag** if this session finished an implementation-order item.

### 4.5 Mid-task recovery example

Session 1 ends halfway through Phase 7.3-B (controller `/pub/v1/sandboxes` POST). The test is red because the conversion logic isn't there yet. The session's last commit:

```
plane: red: failing test for /pub/v1/sandboxes happy path

Wrote the contract test loading createSandbox.request.full.json
and asserting the response matches createSandbox.response.full.json.
Test currently fails because build_connect_request() returns
todo!() — handler scaffold is in place but the conversion logic
isn't implemented yet.

File: crates/plane/src/ext/api_v1.rs line 87

Phase: 7.3-B
Next: Read interfaces.md §3.2 (CreateSandboxRequest -> ConnectRequest
      mapping). Implement build_connect_request() in api_v1.rs.
      Run: cargo test -p plane --test api_v1 -- create_sandbox_happy_path
      until green. Then run the relevant smoke target; should pass.
      Then commit and continue with error-case tests.
```

Session 2 starts. First command: `./scripts/status.sh`. Output shows the resumption point. The next session resumes inside the same TDD cycle (red commit already there; implement to green is the next step). Zero archaeology.

### 4.6 Why git instead of a separate STATUS.md

- **Atomic.** State and code update in one commit.
- **No drift.** State can't lie about the tree; it IS the tree.
- **Time-indexed.** `git log --grep "^Phase: 7.3"` finds every commit in a given phase.
- **Tooling-friendly.** Trailers parse with `git interpret-trailers`; tags filter with `git tag`.
- **Forgetting-proof.** I don't need to remember to update a separate file — I just have to write a commit message, which I'd write anyway.
- **Authentic history.** Every claim about "where the project was on date X" is verifiable from `git log` and `git tag`. No file with "last updated 3 weeks ago" lies.

### 4.7 Trust the docs over recollection

In a session, when I think I remember how something works and the docs say otherwise, the docs win. If I find evidence a doc is wrong, the propagation rule (§1.8) kicks in: fix the doc first, with citation to upstream source, then schemas, then fixtures, then tests, then code. Never code-first.

This rule and the git-as-tracker rule reinforce each other: the tree is the current state, the docs are the contract, both are durable. My memory is neither.

---

## 5. Anti-patterns to avoid

**Anti-pattern 1: writing code before a red test.** If the test isn't red, the test isn't actually testing anything. Always: red first, then green-impl.

**Anti-pattern 2: skipping a TDD layer.** Layer 1 alone doesn't catch wire-format bugs. Layer 2 alone doesn't catch async-runtime bugs. Layer 3 alone doesn't catch real-Plane-protocol bugs. All four layers exist for a reason.

**Anti-pattern 3: importing across binary crates other than via `common`.** Two binary crates shouldn't depend on each other directly. They depend on `common`.

**Anti-pattern 4: drifting fixtures.** If a binary emits something slightly different from a fixture, pick one truth. Never "make the test accept both shapes."

**Anti-pattern 5: leaving upstream mess untouched.** When working in forked Plane code, apply the boy-scout principle. Fix warnings, improve naming, simplify logic — don't preserve upstream quirks out of false respect for an origin we no longer track.

**Anti-pattern 6: skipping the propagation rule.** Code-first changes leave types, schemas, fixtures, and tests behind. They'll be wrong in three different ways and you won't catch it until much later.

**Anti-pattern 7: leaving the smoke canary red.** If a session breaks the canary, fixing it is the first thing in the next session. The session-ending commit's `Next:` trailer says so.

**Anti-pattern 8: trying to do too much in one session.** Better to over-commit small green checkpoints, end at a known state with tight `Phase:` and `Next:` trailers, than push into uncertain territory and end mid-work.

**Anti-pattern 9: pre-optimizing binary boundaries.** If `agentd-headless` and `agentd-desktop` end up 95% shared, they're one binary with a Cargo feature. Reality > the plan.

**Anti-pattern 10: skipping Layer 4 (live e2e) "because Layer 3 passes."** Layer 4 catches the things stubs can't: real wire byte sequences, real Plane behavior, real Docker quirks, real timing. The whole point of Phase 0 is to make Layer 4 cheap enough that you do run it.

**Anti-pattern 11: squashing TDD commits into one.** Squash-on-merge erases the red-then-green rhythm that's the whole point of TDD-aligned commits. The history loses its meaning. Default merge strategy is rebase, not squash.

**Anti-pattern 12: bypassing pre-commit hooks with `--no-verify`.** The hooks exist because mistakes are cheap to catch pre-commit and expensive later. Every `--no-verify` is a future debugging session. If the hook is wrong, fix the hook.

**Anti-pattern 13: committing secrets "just for testing."** No. Use `.env.example` with placeholder values, a local-kms container for KMS dev, and never `git add` a real credential. A committed secret in git history is treated as compromised, period.

**Anti-pattern 14: hand-writing types in a `generated/` directory.** The directory name is a promise: its contents are machine-produced from a single source of truth. Hand-writing types there — even "just this once, I'll wire up codegen later" — creates immediate drift. The SDK type divergence of May 2026 started exactly this way: types were hand-transcribed into `sdks/typescript/src/generated/types.ts`, the transcription introduced field-name errors (`env_var` vs `destination`/`header_name`, `repo` vs `url`), and the Python SDK copied the already-wrong TS types. No CI check existed to catch it because the codegen pipeline was never built. The fix is structural: the generator is the only writer, CI is the only verifier, humans never edit generated output.

**Anti-pattern 15: exposing internal Plane endpoints through the public SDK.** The SDK's public surface covers `/pub/v1/*` (our platform API) and `/v1/*` (agentd data plane). Plane-internal endpoints like `/ctrl/connect`, `/ctrl/status`, `/ctrl/drain` are implementation details — they bypass tenant auth, skip resource validation, and use raw Plane types that don't match our contracts. An SDK method that calls an internal endpoint will work today and break silently when the platform layer changes the contract it was supposed to enforce.

---

**End of discipline doc.** TDD per feature → code shape enforced by clippy → git as audit trail and status tracker. These are the rules every session follows.
