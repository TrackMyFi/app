# Turso Sync — Manual Smoke-Test Checklist

This checklist cannot be run by the automated verification gate. It requires a **real,
running app** (`npm run tauri dev`), a **real free Turso database** (URL + auth token), and
app **restarts** to exercise the local ↔ synced transitions. Run every step below and confirm
each expectation before considering the Turso sync feature **production-verified**.

Setup: launch the app with `npm run tauri dev`.

## Checklist

1. With existing local data, open Settings → Cloud Sync → paste a fresh empty Turso DB's URL + token → Enable. Expect the "Uploaded your existing data (N records)" message → restart.
2. After restart, status shows "Syncing", last-synced is recent. Verify the cloud has data via `turso db shell <db> "SELECT count(*) FROM account;"`.
3. Make an edit (add a balance), click "Sync now", confirm the change reaches the cloud.
4. Confirm `trackmyfi.db.pre-sync-backup` exists in the app data dir.
5. Disable sync → restart → confirm the app still shows the latest data (now reading `trackmyfi-replica.db` locally).
6. Bad-credential path: enter a wrong token → expect a clear error and no mode change (no replica file left behind).
