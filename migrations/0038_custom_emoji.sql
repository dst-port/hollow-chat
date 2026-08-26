CREATE TABLE custom_emoji (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    attachment_id UUID NOT NULL REFERENCES attachments(id) ON DELETE CASCADE,
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (server_id, name)
);

CREATE INDEX custom_emoji_server_id_idx ON custom_emoji (server_id);
