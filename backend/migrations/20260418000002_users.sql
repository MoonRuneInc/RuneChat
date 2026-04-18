CREATE TABLE users (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username                CITEXT NOT NULL UNIQUE,
    email                   TEXT NOT NULL UNIQUE,
    password_hash           TEXT NOT NULL,
    account_status          TEXT NOT NULL DEFAULT 'active'
                                CHECK (account_status IN ('active', 'compromised', 'suspended')),
    compromise_detected_at  TIMESTAMPTZ,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE totp_secrets (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id          UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    secret_encrypted TEXT NOT NULL,
    enrolled_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    verified_at      TIMESTAMPTZ
);

CREATE TABLE refresh_tokens (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  TEXT NOT NULL UNIQUE,
    expires_at  TIMESTAMPTZ NOT NULL,
    revoked_at  TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Replay detection: token exists + revoked_at IS NOT NULL = replay attack
-- Cleanup query target: revoked_at IS NOT NULL AND expires_at < now() - interval '7 days'
CREATE INDEX idx_refresh_tokens_user_expiry ON refresh_tokens (user_id, expires_at);
