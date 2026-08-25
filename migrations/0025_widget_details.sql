ALTER TABLE profile_widgets
    ADD COLUMN external_image_url TEXT,
    ADD COLUMN description TEXT,
    ADD COLUMN tags TEXT[] NOT NULL DEFAULT '{}';
