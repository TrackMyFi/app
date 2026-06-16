-- Replace current_age (stale after every birthday) with date_of_birth so age auto-updates
CREATE TABLE fire_profile_new (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  date_of_birth TEXT,
  target_retirement_age INTEGER NOT NULL,
  annual_expenses_target REAL NOT NULL,
  lean_fire_annual_expenses REAL,
  fat_fire_annual_expenses REAL,
  annual_income REAL NOT NULL,
  expected_return_rate REAL NOT NULL,
  inflation_rate REAL NOT NULL,
  hsa_coverage TEXT NOT NULL DEFAULT 'self',
  onboarding_completed INTEGER NOT NULL DEFAULT 0
);

INSERT INTO fire_profile_new
  SELECT id, NULL, target_retirement_age, annual_expenses_target,
         lean_fire_annual_expenses, fat_fire_annual_expenses, annual_income,
         expected_return_rate, inflation_rate, hsa_coverage, onboarding_completed
  FROM fire_profile;

DROP TABLE fire_profile;
ALTER TABLE fire_profile_new RENAME TO fire_profile;
