CREATE TABLE hsa_expense (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  account_id      INTEGER REFERENCES account(id) ON DELETE SET NULL,
  date            TEXT NOT NULL,
  description     TEXT NOT NULL,
  category        TEXT NOT NULL DEFAULT 'medical',
  amount          REAL NOT NULL,
  person          TEXT,
  provider        TEXT,
  notes           TEXT,
  reimbursed      INTEGER NOT NULL DEFAULT 0,
  reimbursed_date TEXT,
  created_at      TEXT NOT NULL,
  updated_at      TEXT NOT NULL
);

CREATE INDEX idx_hsa_expense_account ON hsa_expense(account_id, date);
CREATE INDEX idx_hsa_expense_date ON hsa_expense(date);

CREATE TABLE hsa_attachment (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  hsa_expense_id  INTEGER NOT NULL REFERENCES hsa_expense(id) ON DELETE CASCADE,
  object_key      TEXT NOT NULL,
  original_name   TEXT NOT NULL,
  provider        TEXT NOT NULL DEFAULT 'local',
  byte_size       INTEGER,
  created_at      TEXT NOT NULL
);

CREATE INDEX idx_hsa_attachment_expense ON hsa_attachment(hsa_expense_id);
