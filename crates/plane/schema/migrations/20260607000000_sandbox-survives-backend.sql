-- Decouple sandbox lifetime from backend lifetime.
--
-- Backend cleanup (BackendDatabase::cleanup, default 7 days post-termination)
-- currently cascades through sandbox → snapshot/port_route/credential_binding/
-- usage_record/audit_egress/extension_action via the chain established in
-- 20260603. That wipes durable artifacts (billing rows, audit trail) on a
-- timeline tied to Plane internals, not to our retention policy.
--
-- After this migration:
--   - sandbox.backend_id is NULL-able
--   - sandbox.backend_id FK is ON DELETE SET NULL
--   - Backend cleanup leaves the sandbox row intact with backend_id = NULL.
--     Sandbox-level retention (terminated_at + tenant policy) drives the
--     eventual hard-delete that cascades to child tables.
--   - The UNIQUE constraint on backend_id is preserved (Postgres treats NULLs
--     as distinct, so multiple terminated-and-orphaned sandboxes can coexist).

ALTER TABLE sandbox ALTER COLUMN backend_id DROP NOT NULL;

ALTER TABLE sandbox DROP CONSTRAINT IF EXISTS sandbox_backend_id_fkey;
ALTER TABLE sandbox
    ADD CONSTRAINT sandbox_backend_id_fkey
    FOREIGN KEY (backend_id) REFERENCES backend(id) ON DELETE SET NULL;
