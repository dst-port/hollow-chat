-- Allow a user to place more than one boost on the same server (up to their
-- total premium slots). The old PRIMARY KEY (user_id, server_id) capped it
-- at one per server; replace it with a surrogate id and keep a lookup index.
ALTER TABLE server_boosts DROP CONSTRAINT server_boosts_pkey;
ALTER TABLE server_boosts ADD COLUMN id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY;
CREATE INDEX server_boosts_user_id_idx ON server_boosts (user_id);
