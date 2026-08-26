CREATE TABLE reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    reporter_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reported_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    context_kind TEXT NOT NULL,
    context_id UUID NOT NULL,
    server_id UUID REFERENCES servers(id) ON DELETE SET NULL,
    status TEXT NOT NULL DEFAULT 'open',
    sealed_key_ephemeral_public BYTEA NOT NULL,
    sealed_key_nonce BYTEA NOT NULL,
    sealed_key_ciphertext BYTEA NOT NULL,
    payload_nonce BYTEA NOT NULL,
    payload_ciphertext BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX reports_reported_user_idx ON reports (reported_user_id);
CREATE INDEX reports_status_idx ON reports (status);
CREATE INDEX reports_reporter_idx ON reports (reporter_id);
