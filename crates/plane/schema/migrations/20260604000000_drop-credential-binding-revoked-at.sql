-- Drop the unused credential_binding.revoked_at column.
--
-- The schema advertised a revoke-but-retain state via this column, but no
-- code path ever writes to it: CredentialDatabase::delete is a hard DELETE
-- and SecretDatabase::cleanup_soft_deleted's NOT EXISTS subquery treats the
-- presence of a binding row (regardless of revoked_at) as a hold on the
-- secret. Keeping the column invites future drift where someone soft-revokes
-- and the cleanup query silently starts pinning secrets forever.
--
-- If a revoke-but-retain audit trail is ever needed, model it explicitly
-- (separate revocation_log table) and update the secret-cleanup subquery in
-- the same change.

-- idx_credential_sandbox was a partial index keyed on (sandbox_id) WHERE
-- revoked_at IS NULL; dropping it explicitly since the column it filters on
-- is going away. No current query relies on the sandbox_id index — all
-- credential_binding lookups are by primary key (id) or by secret_id (which
-- has its own idx_credential_secret index).
DROP INDEX IF EXISTS idx_credential_sandbox;

ALTER TABLE credential_binding DROP COLUMN IF EXISTS revoked_at;
