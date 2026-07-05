-- The unedited description material a bank/bridge sent for a transaction.
--
-- SimpleFIN imports pick ONE display description (payee → description → memo,
-- with payment-processor wrappers like "LINK.COM*" stripped because they hide
-- the real merchant). That cleanup is lossy, so the full distinct set of raw
-- fields is kept alongside — the user can always see exactly what the bank
-- said and correct any future obfuscation. NULL on manual/CSV rows.

ALTER TABLE txn ADD COLUMN raw_description TEXT;
ALTER TABLE simplefin_pending_txn ADD COLUMN raw_description TEXT;
