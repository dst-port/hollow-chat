-- No connection metadata is captured for sessions at all any more (see
-- auth/handlers.rs) - drop the columns outright rather than leaving them
-- around unused, so there's no schema surface left to accidentally start
-- writing to again later.
ALTER TABLE sessions DROP COLUMN user_agent;
ALTER TABLE sessions DROP COLUMN ip_address;
