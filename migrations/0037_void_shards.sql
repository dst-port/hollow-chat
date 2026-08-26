CREATE TABLE server_boosts (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, server_id)
);

CREATE INDEX server_boosts_server_id_idx ON server_boosts (server_id);
