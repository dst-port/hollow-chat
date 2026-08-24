CREATE TABLE servers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE server_members (
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (server_id, user_id)
);

CREATE INDEX server_members_user_id_idx ON server_members (user_id);

ALTER TABLE channels ADD COLUMN server_id UUID REFERENCES servers(id) ON DELETE CASCADE;
ALTER TABLE channels ADD COLUMN channel_type TEXT NOT NULL DEFAULT 'text';
ALTER TABLE channels ADD COLUMN category TEXT;
ALTER TABLE channels ADD COLUMN position INT NOT NULL DEFAULT 0;

CREATE INDEX channels_server_id_idx ON channels (server_id);

ALTER TABLE messages ADD COLUMN author_id UUID REFERENCES users(id) ON DELETE SET NULL;
