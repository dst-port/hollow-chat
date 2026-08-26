CREATE TABLE devicelink_deltas (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    payload TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX devicelink_deltas_user_id_id_idx ON devicelink_deltas (user_id, id);
