-- SimpleFIN transfer detection.
--
-- SimpleFIN has no transfer concept: a transfer between two linked accounts
-- arrives as two independent rows (an expense on the source account, an income
-- on the destination). A post-import pass collapses such pairs into the app's
-- canonical single-row transfer, keeping the source row and deleting the
-- destination row.
--
-- txn.simplefin_counterpart_id records the deleted destination row's SimpleFIN
-- id on the surviving transfer. Without it, the deleted row would be
-- re-imported on the next sync (the 3-day overlap window re-fetches recent
-- transactions, and the unique simplefin_id index no longer blocks an id that
-- was deleted) — the import skips ids present in this column.

ALTER TABLE txn ADD COLUMN simplefin_counterpart_id TEXT;

CREATE UNIQUE INDEX idx_txn_simplefin_counterpart_id
  ON txn(simplefin_counterpart_id) WHERE simplefin_counterpart_id IS NOT NULL;
