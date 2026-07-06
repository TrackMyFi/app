-- Traditional (pre-tax) share of a mixed 401k, as a 0..1 fraction.
-- NULL for other account types, or when the user hasn't set a split.
ALTER TABLE account ADD COLUMN traditional_pct REAL;
