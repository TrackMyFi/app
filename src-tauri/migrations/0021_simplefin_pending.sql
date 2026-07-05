-- Pending SimpleFIN transactions, shown for awareness only.
--
-- Pending rows at the bank mutate or vanish (amount, description, even the
-- SimpleFIN id can change when they post), so they never enter `txn` and
-- nothing may reference them. This table is a pure cache: every sync wipes it
-- and re-inserts the currently-pending set for linked accounts. Cash-flow
-- analytics, charts, and balance math never read it.

CREATE TABLE simplefin_pending_txn (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  account_id INTEGER NOT NULL REFERENCES account(id) ON DELETE CASCADE,
  simplefin_id TEXT NOT NULL,
  amount REAL NOT NULL,
  description TEXT NOT NULL,
  date TEXT NOT NULL,
  -- 'income' | 'expense' (sign-derived, same as the posted import path).
  type TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE INDEX idx_simplefin_pending_account ON simplefin_pending_txn(account_id);
