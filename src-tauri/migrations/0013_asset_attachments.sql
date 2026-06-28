CREATE TABLE asset_attachment (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  asset_event_id  INTEGER NOT NULL REFERENCES asset_event(id) ON DELETE CASCADE,
  object_key      TEXT NOT NULL,
  original_name   TEXT NOT NULL,
  provider        TEXT NOT NULL DEFAULT 'local',
  byte_size       INTEGER,
  created_at      TEXT NOT NULL
);

CREATE INDEX idx_asset_attachment_event ON asset_attachment(asset_event_id);
