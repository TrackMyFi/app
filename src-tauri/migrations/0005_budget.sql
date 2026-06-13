CREATE TABLE budget_month (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  year           INTEGER NOT NULL,
  month          INTEGER NOT NULL,
  savings_target REAL    NOT NULL,
  UNIQUE(year, month)
);

ALTER TABLE paycheck ADD COLUMN income_account_id INTEGER REFERENCES account(id);
