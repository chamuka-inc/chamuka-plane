-- Fix FK cascade gaps so Plane backend cleanup can delete terminated backends
-- that still have sandbox rows, and drop the unused network_policy table
-- (policy is stored on sandbox.network_policy JSONB).

-- Policy lives on sandbox.network_policy; the separate table was never wired up.
DROP TABLE IF EXISTS network_policy;

-- Plane backend cleanup deletes backend rows; sandbox must cascade.
ALTER TABLE sandbox DROP CONSTRAINT IF EXISTS sandbox_backend_id_fkey;
ALTER TABLE sandbox
    ADD CONSTRAINT sandbox_backend_id_fkey
    FOREIGN KEY (backend_id) REFERENCES backend(id) ON DELETE CASCADE;

-- Child rows must not block sandbox deletion during backend cleanup.
ALTER TABLE audit_egress DROP CONSTRAINT IF EXISTS audit_egress_sandbox_id_fkey;
ALTER TABLE audit_egress
    ADD CONSTRAINT audit_egress_sandbox_id_fkey
    FOREIGN KEY (sandbox_id) REFERENCES sandbox(id) ON DELETE CASCADE;

ALTER TABLE extension_action DROP CONSTRAINT IF EXISTS extension_action_backend_id_fkey;
ALTER TABLE extension_action
    ADD CONSTRAINT extension_action_backend_id_fkey
    FOREIGN KEY (backend_id) REFERENCES backend(id) ON DELETE CASCADE;

ALTER TABLE extension_action DROP CONSTRAINT IF EXISTS extension_action_sandbox_id_fkey;
ALTER TABLE extension_action
    ADD CONSTRAINT extension_action_sandbox_id_fkey
    FOREIGN KEY (sandbox_id) REFERENCES sandbox(id) ON DELETE CASCADE;

-- Dev-mode (encrypted_value) and production (KMS envelope) are mutually exclusive
-- storage shapes; at least one must be present.
ALTER TABLE secret DROP CONSTRAINT IF EXISTS secret_storage_present;
ALTER TABLE secret
    ADD CONSTRAINT secret_storage_present CHECK (
        encrypted_value IS NOT NULL
        OR (
            kms_key_id IS NOT NULL
            AND wrapped_data_key IS NOT NULL
            AND nonce IS NOT NULL
            AND ciphertext IS NOT NULL
            AND auth_tag IS NOT NULL
        )
    );
