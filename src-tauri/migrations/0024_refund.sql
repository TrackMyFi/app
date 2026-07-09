-- An income-typed transaction that reverses an earlier expense (a merchant
-- refund, a reversed ACH pull). Balance math is untouched — money really came
-- in, so the row stays type='income' — but cash-flow classification counts it
-- as a NEGATIVE outflow in its category bucket instead of income, netting the
-- original expense out of spending totals.
ALTER TABLE txn ADD COLUMN is_refund INTEGER NOT NULL DEFAULT 0;
