CREATE TABLE IF NOT EXISTS storage_config (
  id            INTEGER PRIMARY KEY CHECK (id = 1),
  provider      TEXT NOT NULL DEFAULT 'local',
  bucket_name   TEXT,
  r2_account_id TEXT,
  s3_region     TEXT
);
-- Seed the single-row so commands can always UPDATE rather than INSERT-or-ignore.
INSERT OR IGNORE INTO storage_config (id, provider) VALUES (1, 'local');
