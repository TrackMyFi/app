-- Suppress rules: hide bank-sync noise from cash-flow analytics.
--
-- Investment accounts emit internal activity ("Fees", "Realizedgainloss",
-- dividend sweeps) that is real money movement inside the account but not
-- income or spending in any monthly-cash-flow sense. A suppress rule matches
-- transaction descriptions (case-insensitive substring, like category rules)
-- and stamps txn.suppressed_as with a kind:
--
--   'investment_activity' | 'fee' | 'interest'
--
-- Suppressed transactions are kept in the database (they still count toward
-- balance math and remain visible behind a toggle) but are excluded from
-- income/expense totals, charts, and the default transactions list. The kind
-- keeps the door open for surfacing e.g. per-account fee drag later.
--
-- account_id scopes a rule to one account (NULL = all accounts). Scoping
-- matters: "fees" on a Fidelity 401(k) is noise, but a fee on checking is a
-- real expense.
--
-- txn.suppressed_as is purely rule-derived: any rule change triggers a full
-- re-derivation (apply_suppress_rules), so there is no per-transaction manual
-- override to preserve.

CREATE TABLE suppress_rules (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  keyword TEXT NOT NULL,
  kind TEXT NOT NULL,
  account_id INTEGER REFERENCES account(id) ON DELETE CASCADE,
  created_at TEXT NOT NULL
);

ALTER TABLE txn ADD COLUMN suppressed_as TEXT;
