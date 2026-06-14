#!/usr/bin/env bash
#
# Seed the dev database from a snapshot of real (prod) data.
#
# Debug builds (`npm run tauri dev`) automatically use a `dev/` subdirectory of
# the app data dir — see resolve_app_dir() in src-tauri/src/db.rs. That isolation
# is compile-time, so a release build (the .dmg) can never read the dev data and
# `tauri dev` can never read the real data, no matter which command you run.
#
# This copies a point-in-time snapshot of real data into that dev dir as a plain
# LOCAL-ONLY database (sync disabled), so you develop against real data without
# ever touching Turso. Re-run any time to reset dev back to current real data.
#
# Source is prod's local replica, which libSQL keeps synced with Turso — so it
# already mirrors the cloud. (For a guaranteed-fresh pull straight from Turso,
# see TRUE FRESH PULL at the bottom of this file.)
#
# Usage: npm run seed:dev
set -euo pipefail

APP_DIR="$HOME/Library/Application Support/com.trackmyfi.desktop"
DEV_DIR="$APP_DIR/dev"

# Prefer prod's synced replica; fall back to its local-only db if never synced.
if [[ -f "$APP_DIR/trackmyfi-replica.db" ]]; then
  SRC="$APP_DIR/trackmyfi-replica.db"
elif [[ -f "$APP_DIR/trackmyfi.db" ]]; then
  SRC="$APP_DIR/trackmyfi.db"
else
  echo "error: no prod database found in $APP_DIR" >&2
  echo "       (looked for trackmyfi-replica.db and trackmyfi.db)" >&2
  exit 1
fi

DEST="$DEV_DIR/trackmyfi.db"

echo "Source : $SRC"
echo "Dest   : $DEST"

mkdir -p "$DEV_DIR"

# Clear any prior dev state so init() lands on LocalOriginal (opens trackmyfi.db):
#   - stale replica + bootstrapped=true would divert it to LocalReplicaFile
#   - leftover -wal/-shm sidecars would shadow the fresh snapshot
rm -f \
  "$DEV_DIR/trackmyfi-replica.db" \
  "$DEV_DIR/trackmyfi-replica.db-info" \
  "$DEV_DIR/trackmyfi-replica.db-shm" \
  "$DEV_DIR/trackmyfi-replica.db-wal" \
  "$DEST" "$DEST-wal" "$DEST-shm"

# Dump + reload produces a clean, defragmented copy and reads through any WAL
# pages still pending on the source.
sqlite3 "$SRC" .dump | sqlite3 "$DEST"

# Force dev local-only: sync disabled, never bootstrapped.
cat > "$DEV_DIR/sync.json" <<'JSON'
{
  "enabled": false,
  "url": null,
  "bootstrapped": false
}
JSON

ROWS=$(sqlite3 "$DEST" "SELECT count(*) FROM sqlite_master WHERE type='table';")
echo "Done. Dev db seeded ($ROWS tables). Sync is OFF — writes stay local."

# --- TRUE FRESH PULL (optional) ------------------------------------------------
# The replica is normally up to date, but to pull straight from Turso instead,
# install the Turso CLI (https://docs.turso.tech/cli) and replace the sqlite3
# dump line above with:
#
#   turso db shell trackmyfi-development-tomgobich .dump | sqlite3 "$DEST"
# -------------------------------------------------------------------------------
