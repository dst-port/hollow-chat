ALTER TABLE users ADD COLUMN bio TEXT;
ALTER TABLE users ADD COLUMN pronouns TEXT;
ALTER TABLE users ADD COLUMN status_text TEXT;
ALTER TABLE users ADD COLUMN accent_color TEXT;
ALTER TABLE users ADD COLUMN banner_color TEXT;
ALTER TABLE users ADD COLUMN avatar_attachment_id UUID REFERENCES attachments(id) ON DELETE SET NULL;
ALTER TABLE users ADD COLUMN banner_attachment_id UUID REFERENCES attachments(id) ON DELETE SET NULL;
