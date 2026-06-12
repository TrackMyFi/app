CREATE TABLE fire_profile (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  current_age INTEGER NOT NULL,
  target_retirement_age INTEGER NOT NULL,
  annual_expenses_target REAL NOT NULL,
  lean_fire_annual_expenses REAL,
  fat_fire_annual_expenses REAL,
  annual_income REAL NOT NULL,
  expected_return_rate REAL NOT NULL,
  inflation_rate REAL NOT NULL
);

CREATE TABLE account (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  type TEXT NOT NULL,
  institution TEXT,
  is_active INTEGER NOT NULL DEFAULT 1,
  include_in_fire_calculations INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL
);

CREATE TABLE account_balance (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  account_id INTEGER NOT NULL REFERENCES account(id) ON DELETE CASCADE,
  balance REAL NOT NULL,
  recorded_at TEXT NOT NULL
);

CREATE INDEX idx_balance_account ON account_balance(account_id, recorded_at);

INSERT INTO fire_profile
  (id, current_age, target_retirement_age, annual_expenses_target,
   annual_income, expected_return_rate, inflation_rate)
VALUES (1, 30, 50, 40000, 80000, 0.07, 0.03);
