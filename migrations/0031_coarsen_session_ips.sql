-- One-time data-minimization pass: existing sessions still hold precise
-- IPs from before ip_address was coarsened at the application layer.
-- New rows are already coarsened in Rust (see coarsen_ip in auth/handlers.rs)
-- before they're ever bound into an INSERT; this just cleans up what's
-- already on disk.
UPDATE sessions
SET ip_address = regexp_replace(ip_address, '^(\d+\.\d+\.\d+)\.\d+$', '\1.0/24')
WHERE ip_address ~ '^\d+\.\d+\.\d+\.\d+$';

UPDATE sessions
SET ip_address = 'redacted'
WHERE ip_address IS NOT NULL
  AND ip_address !~ '^\d+\.\d+\.\d+\.0/24$';
