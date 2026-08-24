ALTER TABLE channels ADD COLUMN slowmode_seconds INT NOT NULL DEFAULT 0;

CREATE TABLE threads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    parent_message_id UUID REFERENCES messages(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    archived BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX threads_channel_id_idx ON threads (channel_id);

ALTER TABLE messages ADD COLUMN thread_id UUID REFERENCES threads(id) ON DELETE CASCADE;

CREATE INDEX messages_thread_id_timestamp_idx ON messages (thread_id, "timestamp") WHERE thread_id IS NOT NULL;
