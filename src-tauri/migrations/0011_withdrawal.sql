-- A contribution-tracked transfer that moves money OUT of an investment account
-- (e.g. M1 Taxable → PNC Checking). `is_contribution` stays the gate flag — a
-- withdrawal still participates in contribution tracking — and `is_withdrawal` is
-- its sign: withdrawals net down unlimited-account contribution totals, while
-- IRS-limited totals continue to count inflows only.
ALTER TABLE txn ADD COLUMN is_withdrawal INTEGER NOT NULL DEFAULT 0;
