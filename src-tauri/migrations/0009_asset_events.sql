CREATE TABLE asset_event (
  id                    INTEGER PRIMARY KEY AUTOINCREMENT,
  account_id            INTEGER REFERENCES account(id) ON DELETE SET NULL,
  asset_label           TEXT,
  date                  TEXT NOT NULL,
  description           TEXT NOT NULL,
  kind                  TEXT NOT NULL DEFAULT 'maintenance',
  cost                  REAL NOT NULL,
  vendor                TEXT,
  notes                 TEXT,
  linked_transaction_id INTEGER REFERENCES txn(id) ON DELETE SET NULL,
  created_at            TEXT NOT NULL,
  updated_at            TEXT NOT NULL
);

CREATE INDEX idx_asset_event_account ON asset_event(account_id, date);
CREATE INDEX idx_asset_event_date ON asset_event(date);
