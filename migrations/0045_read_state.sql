-- Per-user "last read" marker for unread counts. Timestamp-based: when a
-- user opens a channel we store the newest message's timestamp (or now()),
-- and unread = messages after that point not authored by them.
CREATE TABLE channel_read_state (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    last_read_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, channel_id)
);

CREATE TABLE dm_read_state (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    dm_channel_id UUID NOT NULL REFERENCES dm_channels(id) ON DELETE CASCADE,
    last_read_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, dm_channel_id)
);
