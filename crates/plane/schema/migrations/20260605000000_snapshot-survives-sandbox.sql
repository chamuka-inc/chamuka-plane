-- Decouple snapshot lifetime from sandbox lifetime.
--
-- Spec §11.4: snapshots are durable artifacts with their own retention
-- (default 14 days via expires_at). A persistent sandbox writes its
-- snapshot id to sandbox.snapshot_source and resume reads back through
-- the snapshot catalog row, not the sandbox row. Snapshots must therefore
-- outlive the sandbox they originated from — otherwise routine backend
-- cleanup wipes them via the sandbox→snapshot CASCADE introduced in
-- 20260601, even though the snapshot's own retention TTL has not elapsed.
--
-- After this migration:
--   - snapshot.sandbox_id is NULL-able
--   - snapshot.sandbox_id FK uses ON DELETE SET NULL
--   - cleanup_expired (driven by expires_at) is the sole snapshot GC path

ALTER TABLE snapshot ALTER COLUMN sandbox_id DROP NOT NULL;

ALTER TABLE snapshot DROP CONSTRAINT IF EXISTS snapshot_sandbox_id_fkey;
ALTER TABLE snapshot
    ADD CONSTRAINT snapshot_sandbox_id_fkey
    FOREIGN KEY (sandbox_id) REFERENCES sandbox(id) ON DELETE SET NULL;
