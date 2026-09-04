-- A TOTP code stays valid for roughly 90 seconds across the drift window, and
-- nothing recorded which step had already been spent - so an observed code
-- could be redeemed repeatedly inside that window. Track the last step we
-- accepted per user and refuse anything that isn't newer (RFC 6238 5.2).
ALTER TABLE users ADD COLUMN totp_last_counter BIGINT;
