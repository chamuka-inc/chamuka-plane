-- Dev-mode secret storage: adds a plaintext column for use without KMS.
-- Production deployments use the KMS envelope columns (kms_key_id, wrapped_data_key, etc.).
-- This column is nullable so existing rows (if any) are unaffected.
ALTER TABLE secret ADD COLUMN IF NOT EXISTS encrypted_value TEXT;

-- Make the KMS columns nullable for dev mode (they were NOT NULL).
ALTER TABLE secret ALTER COLUMN kms_key_id DROP NOT NULL;
ALTER TABLE secret ALTER COLUMN wrapped_data_key DROP NOT NULL;
ALTER TABLE secret ALTER COLUMN nonce DROP NOT NULL;
ALTER TABLE secret ALTER COLUMN ciphertext DROP NOT NULL;
ALTER TABLE secret ALTER COLUMN auth_tag DROP NOT NULL;
