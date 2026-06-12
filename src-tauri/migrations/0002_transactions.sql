CREATE TABLE txn (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  account_id INTEGER NOT NULL REFERENCES account(id) ON DELETE CASCADE,
  transfer_account_id INTEGER REFERENCES account(id) ON DELETE CASCADE,
  amount REAL NOT NULL,
  description TEXT NOT NULL,
  date TEXT NOT NULL,
  type TEXT NOT NULL,
  category TEXT NOT NULL DEFAULT 'uncategorized',
  is_contribution INTEGER NOT NULL DEFAULT 0,
  import_source TEXT NOT NULL DEFAULT 'manual',
  generated_balance_id INTEGER REFERENCES account_balance(id) ON DELETE SET NULL,
  generated_balance_to_id INTEGER REFERENCES account_balance(id) ON DELETE SET NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX idx_txn_account ON txn(account_id, date);
CREATE INDEX idx_txn_date ON txn(date);

CREATE TABLE import_mapping (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE,
  config TEXT NOT NULL,
  created_at TEXT NOT NULL
);
