ALTER TABLE messages ADD COLUMN edited_at TIMESTAMPTZ;
ALTER TABLE messages ADD COLUMN pinned BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE messages ADD COLUMN reply_to_id UUID REFERENCES messages(id) ON DELETE SET NULL;

ALTER TABLE dm_messages ADD COLUMN edited_at TIMESTAMPTZ;
ALTER TABLE dm_messages ADD COLUMN pinned BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE dm_messages ADD COLUMN reply_to_id UUID REFERENCES dm_messages(id) ON DELETE SET NULL;

CREATE TABLE message_reactions (
    message_id UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    emoji TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (message_id, user_id, emoji)
);

CREATE TABLE dm_message_reactions (
    dm_message_id UUID NOT NULL REFERENCES dm_messages(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    emoji TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (dm_message_id, user_id, emoji)
);

CREATE INDEX messages_pinned_idx ON messages (channel_id) WHERE pinned = true;
CREATE INDEX dm_messages_pinned_idx ON dm_messages (dm_channel_id) WHERE pinned = true;
