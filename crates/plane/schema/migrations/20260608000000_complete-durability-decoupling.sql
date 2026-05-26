-- Close the remaining gaps in the durability-decoupling stack.
--
-- 20260605 made snapshot.sandbox_id SET NULL.
-- 20260606 denormalized tenant onto usage_record + audit_egress.
-- 20260607 made sandbox.backend_id SET NULL.
--
-- Still cascading-on-sandbox-delete (which is now the authoritative GC at
-- 83-day retention): audit_egress, usage_record, port_route,
-- credential_binding, extension_action. The intent for billing/forensics
-- (usage_record + audit_egress) was for them to survive the sandbox row
-- and follow their own retention. Cycle 3 added tenant for attribution
-- but the FK switch was deferred behind a usage_record PK redesign (the
-- composite PK includes sandbox_id, so it cannot become nullable yet).
--
-- This migration:
--   1. Switches audit_egress.sandbox_id to nullable + SET NULL FK.
--      No PK conflict (audit_egress has its own SERIAL id PK).
--   2. Switches fk_sandbox_snapshot_source to SET NULL. Without this,
--      cleanup_expired's DELETE on a snapshot raises foreign_key_violation
--      whenever any sandbox row's snapshot_source points at it.
--
-- usage_record stays CASCADE for now (PK redesign tracked separately).

ALTER TABLE audit_egress ALTER COLUMN sandbox_id DROP NOT NULL;

ALTER TABLE audit_egress DROP CONSTRAINT IF EXISTS audit_egress_sandbox_id_fkey;
ALTER TABLE audit_egress
    ADD CONSTRAINT audit_egress_sandbox_id_fkey
    FOREIGN KEY (sandbox_id) REFERENCES sandbox(id) ON DELETE SET NULL;

ALTER TABLE sandbox DROP CONSTRAINT IF EXISTS fk_sandbox_snapshot_source;
ALTER TABLE sandbox
    ADD CONSTRAINT fk_sandbox_snapshot_source
    FOREIGN KEY (snapshot_source) REFERENCES snapshot(id) ON DELETE SET NULL;
