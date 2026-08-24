CREATE TABLE server_invites (
    server_id UUID PRIMARY KEY REFERENCES servers(id) ON DELETE CASCADE,
    code TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
