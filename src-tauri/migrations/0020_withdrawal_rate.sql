-- Configurable safe withdrawal rate. 0.04 (the 4% rule, i.e. expenses × 25)
-- was the hardcoded assumption before this column existed; longer early-retirement
-- horizons often plan on 3.5% or 3.25%.
ALTER TABLE fire_profile ADD COLUMN withdrawal_rate REAL NOT NULL DEFAULT 0.04;
