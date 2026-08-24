ALTER TABLE users ADD COLUMN tier TEXT NOT NULL DEFAULT 'free';
ALTER TABLE users ADD COLUMN stripe_customer_id TEXT;
ALTER TABLE users ADD COLUMN stripe_subscription_id TEXT;
ALTER TABLE users ADD COLUMN subscription_status TEXT;
ALTER TABLE users ADD COLUMN current_period_end TIMESTAMPTZ;

CREATE UNIQUE INDEX users_stripe_customer_id_idx ON users (stripe_customer_id) WHERE stripe_customer_id IS NOT NULL;

CREATE TABLE attachments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    uploader_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    filename TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    storage_key TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE messages ADD COLUMN attachment_id UUID REFERENCES attachments(id) ON DELETE SET NULL;
ALTER TABLE dm_messages ADD COLUMN attachment_id UUID REFERENCES attachments(id) ON DELETE SET NULL;

ALTER TABLE messages ALTER COLUMN encrypted_blob DROP NOT NULL;
ALTER TABLE dm_messages ALTER COLUMN encrypted_blob DROP NOT NULL;

ALTER TABLE messages ADD CONSTRAINT messages_content_or_attachment
    CHECK (encrypted_blob IS NOT NULL OR attachment_id IS NOT NULL);
ALTER TABLE dm_messages ADD CONSTRAINT dm_messages_content_or_attachment
    CHECK (encrypted_blob IS NOT NULL OR attachment_id IS NOT NULL);
