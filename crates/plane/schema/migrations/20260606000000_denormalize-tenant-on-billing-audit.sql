-- Denormalize tenant onto usage_record and audit_egress.
--
-- Both tables currently obtain tenant attribution via JOIN sandbox; that
-- couples their retention timeline to sandbox lifetime. After the snapshot
-- decoupling in 20260605, the next durable artifacts to address are
-- billing (usage_record) and forensics (audit_egress). Adding a local
-- tenant column lets queries attribute rows directly and unblocks a
-- future ON DELETE SET NULL on sandbox_id without losing tenant
-- attribution for orphaned rows.
--
-- Both columns are NOT NULL with FK to tenant(id). No backfill needed —
-- fresh deploys only; this PR adds tenant before any production writes.

ALTER TABLE usage_record
    ADD COLUMN tenant VARCHAR(255) NOT NULL REFERENCES tenant(id);

CREATE INDEX idx_usage_record_tenant_bucket
    ON usage_record (tenant, bucket_start DESC);

ALTER TABLE audit_egress
    ADD COLUMN tenant VARCHAR(255) NOT NULL REFERENCES tenant(id);

CREATE INDEX idx_audit_egress_tenant_at
    ON audit_egress (tenant, at DESC);
