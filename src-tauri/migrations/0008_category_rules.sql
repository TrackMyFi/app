CREATE TABLE category_rules (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  keyword TEXT NOT NULL,
  category TEXT NOT NULL,
  created_at TEXT NOT NULL
);

INSERT INTO category_rules (keyword, category, created_at) VALUES
  ('netflix', 'fixed', datetime('now')),
  ('spotify', 'fixed', datetime('now')),
  ('hulu', 'fixed', datetime('now')),
  ('disney+', 'fixed', datetime('now')),
  ('apple.com/bill', 'fixed', datetime('now')),
  ('amazon prime', 'fixed', datetime('now')),
  ('youtube premium', 'fixed', datetime('now')),
  ('amazon', 'discretionary', datetime('now')),
  ('walmart', 'discretionary', datetime('now')),
  ('target', 'discretionary', datetime('now')),
  ('starbucks', 'discretionary', datetime('now')),
  ('mcdonald', 'discretionary', datetime('now'));
