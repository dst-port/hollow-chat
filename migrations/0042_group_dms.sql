ALTER TABLE dm_channels ALTER COLUMN user_a DROP NOT NULL;
ALTER TABLE dm_channels ALTER COLUMN user_b DROP NOT NULL;
ALTER TABLE dm_channels DROP CONSTRAINT IF EXISTS dm_channels_ordered;
ALTER TABLE dm_channels DROP CONSTRAINT IF EXISTS dm_channels_unique_pair;
ALTER TABLE dm_channels ADD CONSTRAINT dm_channels_ordered
    CHECK (user_a IS NULL OR user_b IS NULL OR user_a < user_b);
ALTER TABLE dm_channels ADD COLUMN is_group BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE dm_channels ADD COLUMN name TEXT;
ALTER TABLE dm_channels ADD COLUMN owner_id UUID REFERENCES users(id) ON DELETE SET NULL;

CREATE UNIQUE INDEX dm_channels_unique_pair ON dm_channels (user_a, user_b) WHERE is_group = false;

CREATE TABLE dm_channel_members (
    dm_channel_id UUID NOT NULL REFERENCES dm_channels(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    added_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (dm_channel_id, user_id)
);

CREATE INDEX dm_channel_members_user_idx ON dm_channel_members (user_id);

INSERT INTO dm_channel_members (dm_channel_id, user_id)
SELECT id, user_a FROM dm_channels WHERE user_a IS NOT NULL
UNION
SELECT id, user_b FROM dm_channels WHERE user_b IS NOT NULL;

CREATE TABLE dm_channel_sender_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    dm_channel_id UUID NOT NULL REFERENCES dm_channels(id) ON DELETE CASCADE,
    sender_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recipient_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    ciphertext TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (dm_channel_id, sender_id, recipient_id)
);

CREATE INDEX dm_channel_sender_keys_recipient_idx ON dm_channel_sender_keys (dm_channel_id, recipient_id);
