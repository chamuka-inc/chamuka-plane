# Fork Provenance

Hard fork of [jamsocket/plane](https://github.com/jamsocket/plane).

- **Fork point:** commit `abca802` (2025-05-16, "drop log level to debug (#882)")
- **Strategy:** Hard fork. No upstream tracking. Modify freely.
- **Rationale:** Tight integration with our workspace; upstream's design decisions don't constrain ours.

## Workspace integration

Plane is its own Cargo workspace at `crates/plane/`, excluded from the parent workspace via `exclude = ["plane"]` in `crates/Cargo.toml`. Parent crates depend on plane crates via path:

```toml
plane = { path = "plane/plane" }
plane-common = { path = "plane/common" }
```

Build requires `SQLX_OFFLINE=true` (offline query cache in `plane/.sqlx/`).
