# TrackMyFI — Turso Sync Design

**Date:** 2026-06-13
**Status:** Approved (design); implementation plan to follow
**Supersedes/implements:** the long-deferred "background Turso sync" item from the
original design (`docs/superpowers/specs/2026-06-09-trackmyfi-design.md`).

## Goal

Wire the embedded-replica cloud sync that the app was architected for but never
built. Today `db::init` uses `Builder::new_local`; this feature lets a user
optionally point the app at a free Turso cloud database so their financial data
is durable and reconciled across machines — while keeping the app fully
local-first and offline by default.

## Principles

- **Local-first, sync opt-in.** With no credentials configured the app behaves
  exactly as it does today: a local libSQL file, no account, no network.
- **The synced DB stays clean.** Sync configuration lives *outside* the libSQL
  database, because that database is the thing being replicated. Storing sync
  config in it would be circular and would leak device-specific settings to
  every other device.
- **No data loss across any transition** — enabling, disabling, or first-run
  bootstrap always preserves the user's data and keeps a backup.
- **Honest, bounded scope.** v1 handles the realistic single-user, multi-device
  case. Merging two already-populated databases is explicitly out of scope.

## Architecture

### The single branch point: `db::init`

Builder selection depends on configuration and which files exist:

| Condition | Opens | Builder |
|---|---|---|
| sync enabled + credentials present | `trackmyfi-replica.db` | `new_remote_replica(path, url, token)` |
| previously synced, now disabled | `trackmyfi-replica.db` | `new_local` (keeps latest synced data) |
| never synced | `trackmyfi.db` | `new_local` (today's path) |

Two files are used deliberately:

- `trackmyfi.db` — the original local-only file (what exists today).
- `trackmyfi-replica.db` — created the first time sync is enabled; managed by
  libSQL's embedded-replica machinery (it carries sync metadata). Once it
  exists it is the canonical store, even if sync is later disabled (it opens
  cleanly as a plain local DB).

After remote-replica build + migrations on startup, kick one `sync()`.

### Configuration storage (outside the DB)

| Item | Storage | Rationale |
|---|---|---|
| Auth token (secret) | OS keychain via the `keyring` crate | Never in plaintext on disk, never in the synced DB |
| Sync URL + `enabled` + `bootstrapped` flags | `sync.json` in Tauri's `app_config_dir` | Not secret; device-local; keeps the synced DB clean |

No new migration is required — sync metadata never touches the schema.

### New module: `src-tauri/src/sync.rs`

Owns:

- `sync.json` read/write/round-trip.
- Keychain get/set/delete, wrapped behind a thin trait so tests never touch the
  real OS keychain.
- `do_sync()` — the single funnel all triggers call, guarded by a
  `tokio::Mutex` (or an in-progress `AtomicBool`) so the timer and a manual
  click can't overlap. Reads always hit the local replica and never block.
- The background interval task.
- The first-enable bootstrap routine (below).

New `keyring` dependency in `Cargo.toml`. This is the only new dependency.

### New Tauri commands

- `get_sync_status` → returns `SyncState`.
- `save_sync_config(url, token)` → validates, bootstraps if needed, persists,
  signals the UI to prompt a restart.
- `clear_sync_config` → disables sync, deletes the keychain token.
- `sync_now` → one manual `do_sync()`.

### Shared status state

`SyncState` lives in Tauri-managed state, is returned by `get_sync_status`, and
is updated by the loop:

```
{ mode: "local" | "synced",
  status: "idle" | "syncing" | "error",
  lastSyncedAt: string | null,
  lastError: string | null }
```

After each sync attempt the loop emits a `sync-status` Tauri event so Settings
shows live state without polling.

## Sync triggers

All funnel through `do_sync()`:

- **Startup** — `new_remote_replica` pulls on build; we also kick one `sync()`
  after migrations.
- **Periodic** — a `tokio` interval task spawned in `lib.rs` setup, only when
  sync is enabled. Interval is a single named constant `SYNC_INTERVAL`,
  defaulting to **15 minutes**, not exposed in the UI (YAGNI). The lifecycle
  triggers do the real work; the timer is a backstop for long-open sessions.
- **On close** — a final `sync()` on the window close/exit event.
- **Manual** — `sync_now` behind a "Sync now" button.

Rationale for 15 min: this is low-write-frequency personal-finance data, and
Turso's free tier meters syncs/rows — minute-level polling would burn quota to
copy nothing. Startup-pull + close-push cover the "edited on laptop, opened on
desktop" case; "Sync now" covers backfills.

**Offline is not an error we nag about.** A failed `sync()` is logged, surfaced
quietly as `lastError`, and retried on the next tick; the app keeps working
locally.

## First-enable bootstrap

A libSQL embedded replica manages its own local file, so we cannot hand it the
existing plain `trackmyfi.db` and expect it to push that up. Bootstrap is an
explicit step inside `save_sync_config`, run *before* the restart prompt so
errors are interactive:

1. **Validate** the URL + token by opening a remote replica and doing one
   `sync()`. Bad credentials → abort; nothing has changed.
2. **Inspect the cloud DB:**
   - **Empty (common case)** → run migrations on the replica, then copy every
     app table from `trackmyfi.db` into the replica row-by-row in Rust (not
     `ATTACH` — embedded-replica connections don't reliably support it), then
     `sync()` to push it all up.
   - **Already populated** (another device seeded it first) → adopt the cloud
     copy; do **not** merge. Keep this machine's old data as a backup file and
     warn the user.
3. **Back up** the old file as `trackmyfi.db.pre-sync-backup` (never deleted
   automatically).
4. Write `sync.json` (`enabled: true`, url, `bootstrapped: true`) + token to
   keychain → prompt restart.

Bootstrap is guarded to run once (keyed on `bootstrapped` / replica-file
existence) so a restart does not re-copy.

## Disable flow

`clear_sync_config` writes `sync.json` `enabled: false` and deletes the keychain
token. On restart, `db::init` opens `trackmyfi-replica.db` as a plain local DB,
so the latest synced data is retained (the stale `pre-sync-backup` is not used).

## UI — Settings → Sync panel

Added to the existing `src/pages/Settings.vue`:

- URL field + token field (token rendered as a password input).
- Enable / Disable controls; "Sync now" button.
- Live status: mode, syncing/idle/error, last-synced time, last error.
- A restart prompt after enabling/disabling (uses the async dialog plugin —
  `window.confirm` is a no-op in the Tauri webview).
- Collapsible **"How to set this up"** with two tabs:
  - **Dashboard (non-technical):** numbered, screenshot-friendly steps — sign up
    at turso.tech → create a database → open it → copy the **Database URL** →
    create a token → copy it → paste both fields → Enable. Plain language, no
    terminal.
  - **CLI (technical):** `turso auth signup`, `turso db create trackmyfi`,
    `turso db show trackmyfi --url`, `turso db tokens create trackmyfi`, paste,
    Enable.
  - Both note: free-tier; the URL isn't secret but the token is (treat like a
    password); the user owns their cloud DB.

A condensed version of the setup steps also goes into `README.md` and the
project memory.

## Testing

- **Rust unit tests** (existing `tokio` dev-dep pattern):
  - `sync.json` read/write/round-trip.
  - `db::init` file-selection logic across all three states (temp dirs).
  - Table-copy bootstrap against two temp/in-memory libSQL DBs: seed source,
    copy, assert row counts and a sample row per table.
  - Keychain access behind a trait so tests don't touch the real OS keychain.
- **Manual smoke test** (documented checklist; needs a real Turso DB + a
  restart): enable on a populated DB → verify cloud has data → restart → edit →
  "Sync now" → confirm on a second machine / `turso db shell`.
- Existing 95 Vitest + 47 cargo stay green. `keyring` is the only new dep.

## Verification spike (before coding)

Confirm the exact libsql 0.9 API against the real docs/source — `new_remote_replica`
signature, `sync()`, and that a replica file opens cleanly via `new_local`. The
crate's surface has shifted across versions; pin this in the plan rather than
trust memory.

## Out of scope (v1)

- Merging two already-populated databases; conflict-resolution UI.
- Encryption beyond OS full-disk encryption (FileVault/BitLocker) — see
  `docs/superpowers/specs/2026-06-13-encryption-at-rest-design.md`.
- App-managed accounts / any backend of our own.
- User-configurable sync interval.
