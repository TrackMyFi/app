-- Optional current value for free-text assets (real-estate assets derive value
-- from their account's latest balance snapshot instead). The asset's current
-- value is read from its most recent event that has a value set.
ALTER TABLE asset_event ADD COLUMN asset_value REAL;
