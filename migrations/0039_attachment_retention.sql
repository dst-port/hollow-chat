ALTER TABLE attachments ADD COLUMN purged_at TIMESTAMPTZ;

-- Sweeps only ever look at attachments that aren't already known to be
-- long-lived (avatars, banners, server icons, emoji, widget images), so this
-- keeps that scan cheap regardless of how large the table gets.
CREATE INDEX attachments_sweep_idx ON attachments (created_at) WHERE purged_at IS NULL;
