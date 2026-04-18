CREATE TABLE channels (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id    UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    display_name TEXT NOT NULL,
    slug         TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (server_id, slug)
);

-- Supports listing and lookup by server
CREATE INDEX idx_channels_server_slug ON channels (server_id, slug);
