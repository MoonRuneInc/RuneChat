ALTER TABLE users ADD COLUMN locked_until TIMESTAMPTZ;

CREATE TABLE audit_log (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type  TEXT NOT NULL,
    user_id     UUID REFERENCES users(id) ON DELETE SET NULL,
    ip          TEXT,
    details     JSONB,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_audit_log_user_id    ON audit_log (user_id);
CREATE INDEX idx_audit_log_created_at ON audit_log (created_at DESC);
CREATE INDEX idx_audit_log_event_type ON audit_log (event_type);
