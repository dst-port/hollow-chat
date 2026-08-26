ALTER TABLE servers ADD COLUMN icon_attachment_id UUID REFERENCES attachments(id);
