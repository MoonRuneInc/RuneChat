CREATE TABLE messages (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id          UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    author_id           UUID NOT NULL REFERENCES users(id),
    content             TEXT NOT NULL,
    compromised_at_send BOOLEAN NOT NULL DEFAULT false,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    edited_at           TIMESTAMPTZ
);

-- Primary query pattern: fetch last N messages in a channel ordered by time
CREATE INDEX idx_messages_channel_created ON messages (channel_id, created_at DESC);
