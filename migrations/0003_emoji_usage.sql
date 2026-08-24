CREATE TABLE emoji_usage (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    emoji TEXT NOT NULL,
    count INTEGER NOT NULL DEFAULT 0,
    last_used_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, emoji)
);

CREATE INDEX emoji_usage_user_frequency_idx ON emoji_usage (user_id, count DESC, last_used_at DESC);
