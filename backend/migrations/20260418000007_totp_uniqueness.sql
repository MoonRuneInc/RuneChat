-- A user should have at most one TOTP secret enrolled at a time.
-- This prevents duplicate enrollment and simplifies the unlock flow.
ALTER TABLE totp_secrets ADD CONSTRAINT uniq_totp_secret_per_user UNIQUE (user_id);
