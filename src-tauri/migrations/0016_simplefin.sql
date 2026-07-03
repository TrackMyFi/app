-- SimpleFIN Bridge integration.
--
-- account.simplefin_id      links a local account to a SimpleFIN account id.
-- account_balance.source    'manual' (user-entered or txn-generated) | 'simplefin'.
-- txn.simplefin_id          SimpleFIN transaction id; unique so re-syncs and
--                           overlapping fetch windows dedupe via INSERT OR IGNORE.
-- txn.vendor_category       category supplied by the bank/bridge, kept separate
--                           from the app's own category field.
-- simplefin_state           singleton sync bookkeeping + cached remote account
--                           list (so the linking UI never needs an extra API
--                           call — SimpleFIN asks clients to poll ~once a day).

ALTER TABLE account ADD COLUMN simplefin_id TEXT;
ALTER TABLE account_balance ADD COLUMN source TEXT NOT NULL DEFAULT 'manual';
ALTER TABLE txn ADD COLUMN simplefin_id TEXT;
ALTER TABLE txn ADD COLUMN vendor_category TEXT;

CREATE UNIQUE INDEX idx_txn_simplefin_id ON txn(simplefin_id) WHERE simplefin_id IS NOT NULL;
CREATE INDEX idx_account_simplefin ON account(simplefin_id);

CREATE TABLE simplefin_state (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  claimed_at TEXT,
  last_attempt_at TEXT,
  last_success_at TEXT,
  last_error TEXT,
  -- JSON array of messages returned in the account set's "errors" field —
  -- typically "Connection to <bank> may need attention". Present even when the
  -- sync itself succeeded.
  bridge_errors TEXT,
  -- JSON cache of the remote accounts seen on the last fetch.
  accounts_json TEXT
);

INSERT INTO simplefin_state (id) VALUES (1);
