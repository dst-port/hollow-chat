CREATE TABLE badges (
    slug TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    description TEXT NOT NULL,
    sort_order INT NOT NULL DEFAULT 0
);

CREATE TABLE user_badges (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    badge_slug TEXT NOT NULL REFERENCES badges(slug) ON DELETE CASCADE,
    awarded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, badge_slug)
);

CREATE INDEX user_badges_user_id_idx ON user_badges (user_id);

INSERT INTO badges (slug, label, description, sort_order) VALUES
    ('owner', 'Owner', 'Owns HollowChat', 10),
    ('staff', 'Hollow Staff', 'Works at HollowChat', 20),
    ('developer', 'Developer', 'Verified app or bot developer', 30),
    ('dev-contributor', 'Hollow Chat Development', 'Contributed to building HollowChat', 40),
    ('supporter', 'Supporter', 'Subscribed to Hollow Chatter', 50);
