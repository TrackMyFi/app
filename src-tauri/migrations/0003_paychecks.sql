CREATE TABLE paycheck (
  id                  INTEGER PRIMARY KEY AUTOINCREMENT,
  pay_date            TEXT NOT NULL,
  employer            TEXT NOT NULL,
  pay_period          TEXT NOT NULL,
  gross_amount        REAL NOT NULL,
  net_amount          REAL NOT NULL,
  federal_tax         REAL NOT NULL DEFAULT 0,
  state_tax           REAL NOT NULL DEFAULT 0,
  local_tax           REAL NOT NULL DEFAULT 0,
  social_security_tax REAL NOT NULL DEFAULT 0,
  medicare_tax        REAL NOT NULL DEFAULT 0,
  deductions          TEXT NOT NULL DEFAULT '[]',
  employer_match      TEXT NOT NULL DEFAULT '[]',
  import_source       TEXT NOT NULL DEFAULT 'manual',
  created_at          TEXT NOT NULL,
  updated_at          TEXT NOT NULL
);

CREATE INDEX idx_paycheck_date ON paycheck(pay_date);

ALTER TABLE txn ADD COLUMN paycheck_id INTEGER REFERENCES paycheck(id) ON DELETE CASCADE;
