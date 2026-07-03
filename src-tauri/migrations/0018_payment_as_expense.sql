-- Count transfers INTO an account as spending.
--
-- Transfers are normally cash-flow neutral: counting a checking → credit card
-- payment as an expense would double-count every purchase already recorded
-- against the card. That reasoning breaks for loan-style accounts (a mortgage,
-- a car loan): no purchases are ever recorded against them, so the payment is
-- the only footprint the expense will ever leave. When this flag is set on the
-- destination account, classifyFlow (and its Rust mirror in period_stats)
-- counts the full transfer amount as fixed spending.
--
-- The flag lives on the account, and classification happens at read time —
-- flipping it instantly reclassifies history, and manual/CSV transfers get the
-- same treatment as synced ones. Mortgages default on; credit cards and other
-- liabilities default off.

ALTER TABLE account ADD COLUMN count_payments_as_expense INTEGER NOT NULL DEFAULT 0;

UPDATE account SET count_payments_as_expense = 1 WHERE type = 'mortgage';
