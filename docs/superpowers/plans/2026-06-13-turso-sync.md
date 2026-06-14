# Turso Sync Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add opt-in Turso cloud sync to TrackMyFI via a libSQL embedded replica, keeping the app fully local-first by default and losing no data across enable/disable/bootstrap transitions.

**Architecture:** A single branch in `db::init` chooses `Builder::new_local` (today's behavior) or `Builder::new_remote_replica` based on config stored *outside* the synced DB (token in OS keychain, URL + flags in `sync.json`). A new `sync.rs` module owns config, keychain access, the sync funnel, the background timer, and the first-enable bootstrap that copies existing local rows up to an empty cloud DB. A small Settings panel drives it with setup instructions for both dashboard and CLI users.

**Tech Stack:** Rust (Tauri 2, libsql 0.9 with default `sync`/`remote`/`replication` features, `keyring` 3), Vue 3 + NuxtUI 4 + Pinia, ts-rs for the status type.

---

## Verified API facts (from installed crate source — do not re-derive)

- `libsql::Builder::new_local(path).build().await -> Result<Database>`.
- `libsql::Builder::new_remote_replica(path, url: String, token: String).build().await -> Result<Database>`.
- `Database::sync().await -> Result<Replicated>`; `Replicated::frames_synced() -> usize`.
- `Database::connect() -> Result<Connection>`.
- `Rows::column_count() -> i32`, `Rows::column_name(i32) -> Option<&str>`, `Row::get_value(i32) -> Result<libsql::Value>`, `Row::get::<T>(i32)`.
- `libsql::params_from_iter(iter)` binds an iterator of values positionally; `libsql::Value` is accepted.
- libsql 0.9 default features already include `sync`, `remote`, `replication`, `tls` — no Cargo feature changes needed.
- `keyring` 3 API: `Entry::new(service, user) -> Result<Entry>`, `Entry::set_password(&str)`, `Entry::get_password() -> Result<String>`, `Entry::delete_credential()`, error variant `keyring::Error::NoEntry` when the entry is missing.
- Tauri v2: emit with `use tauri::Emitter; app.emit("event", payload)`. `AppHandle::restart()` relaunches and never returns. Run-loop events via `.build(ctx)?` then `.run(|handle, event| …)` matching `tauri::RunEvent::ExitRequested`.

## File structure

- `src-tauri/Cargo.toml` — add `keyring = "3"`.
- `src-tauri/src/sync.rs` — **new.** Config (`SyncConfig` + read/write), `TokenStore` trait + `KeyringStore`, `SyncStatus` (ts-rs) + `SyncShared` managed state, `do_sync`, bootstrap helpers (`list_data_tables`, `copy_all_data`), and all sync commands.
- `src-tauri/src/db.rs` — **modify.** Path helpers, `DbMode`, `decide_db_source`, branching `init`.
- `src-tauri/src/lib.rs` — **modify.** Declare module, manage `SyncShared`, register commands, spawn the interval task, run-loop exit sync.
- `src/lib/types/SyncStatus.ts` — **generated** by ts-rs.
- `src/lib/api/sync.ts` — **new.** invoke wrappers + event listener.
- `src/stores/sync.ts` — **new.** Pinia store holding live status.
- `src/pages/Settings.vue` — **modify.** Sync panel + setup-instructions tabs + restart dialog.
- `src/App.vue` — **modify.** Initialize the sync store (event subscription) once on mount.
- `README.md` — **modify.** Condensed setup steps.

## Conventions to follow (from the existing codebase)

- Rust commands: testable inner `async fn(conn/…)` + thin `#[tauri::command]` wrapper with `_cmd` suffix where it queries; existing non-suffixed names exist too — match the file you edit. Map rows by index. `map_err(|e| e.to_string())`.
- Types: `#[derive(Serialize, Deserialize, TS, Clone)]`, `#[serde(rename_all = "camelCase")]`, `#[ts(export, export_to = "../../src/lib/types/")]`. ts-rs writes the `.ts` file during `cargo test`.
- Frontend: API wrappers in `src/lib/api/*.ts` using `invoke<T>('cmd', { camelCaseArgs })`; Pinia stores in `src/stores/*.ts`.
- Tauri dialogs: use `@tauri-apps/plugin-dialog`'s async `confirm()` — `window.confirm` is a no-op in the webview.

---

### Task 1: Add `keyring` dep and the sync-config module

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/sync.rs`
- Modify: `src-tauri/src/lib.rs` (add `pub mod sync;`)

- [ ] **Step 1: Add the dependency**

In `src-tauri/Cargo.toml`, under `[dependencies]`, add after the `libsql = "0.9"` line:

```toml
keyring = "3"
```

- [ ] **Step 2: Create `sync.rs` with the config type + a failing round-trip test**

Create `src-tauri/src/sync.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Device-local sync configuration. Stored as `sync.json` in the app config dir,
/// deliberately OUTSIDE the libSQL database (which is the thing being synced).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct SyncConfig {
    pub enabled: bool,
    pub url: Option<String>,
    pub bootstrapped: bool,
}

pub fn read_config(path: &Path) -> SyncConfig {
    match std::fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => SyncConfig::default(),
    }
}

pub fn write_config(path: &Path, cfg: &SyncConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("trackmyfi_test_{name}.json"))
    }

    #[test]
    fn missing_file_yields_default() {
        let p = tmp("missing_cfg");
        let _ = std::fs::remove_file(&p);
        assert_eq!(read_config(&p), SyncConfig::default());
    }

    #[test]
    fn round_trips_config() {
        let p = tmp("round_trip_cfg");
        let cfg = SyncConfig { enabled: true, url: Some("libsql://x".into()), bootstrapped: true };
        write_config(&p, &cfg).unwrap();
        assert_eq!(read_config(&p), cfg);
        let _ = std::fs::remove_file(&p);
    }
}
```

Add `pub mod sync;` to `src-tauri/src/lib.rs` (alongside the other `pub mod` lines).

- [ ] **Step 3: Run the tests to verify they pass (compile + behavior)**

Run: `cd src-tauri && cargo test sync::tests -- --nocapture`
Expected: both `missing_file_yields_default` and `round_trips_config` PASS. (Confirms `keyring` resolves and the module compiles.)

- [ ] **Step 4: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/sync.rs src-tauri/src/lib.rs
git commit -m "feat(sync): add keyring dep + sync-config (sync.json) module"
```

---

### Task 2: DB source decision + branching `db::init`

**Files:**
- Modify: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/sync.rs` (add path helpers used by db.rs)

- [ ] **Step 1: Add path helpers to `sync.rs`**

Append to `src-tauri/src/sync.rs` (above the `#[cfg(test)]` block):

```rust
use tauri::{AppHandle, Manager};

pub fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("sync.json"))
}

pub fn read_app_config(app: &AppHandle) -> SyncConfig {
    match config_path(app) {
        Ok(p) => read_config(&p),
        Err(_) => SyncConfig::default(),
    }
}

pub fn write_app_config(app: &AppHandle, cfg: &SyncConfig) -> Result<(), String> {
    let p = config_path(app)?;
    write_config(&p, cfg)
}
```

- [ ] **Step 2: Write the failing `decide_db_source` test in `db.rs`**

Replace the contents of `src-tauri/src/db.rs` with:

```rust
use libsql::{Builder, Connection, Database};
use tauri::{AppHandle, Manager};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DbMode {
    Local,
    Synced,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DbSource {
    /// Remote embedded replica at the replica file.
    RemoteReplica,
    /// Previously-synced replica file opened locally (sync disabled).
    LocalReplicaFile,
    /// Original local-only file (never synced).
    LocalOriginal,
}

/// Pure decision: which file/builder to use given config + whether the replica file exists.
pub fn decide_db_source(sync_enabled: bool, has_creds: bool, replica_exists: bool) -> DbSource {
    if sync_enabled && has_creds {
        DbSource::RemoteReplica
    } else if replica_exists {
        DbSource::LocalReplicaFile
    } else {
        DbSource::LocalOriginal
    }
}

pub struct Db {
    pub db: Database,
    pub mode: DbMode,
}

impl Db {
    pub async fn conn(&self) -> Result<Connection, String> {
        self.db.connect().map_err(|e| e.to_string())
    }
    pub fn is_synced(&self) -> bool {
        self.mode == DbMode::Synced
    }
}

pub const LOCAL_DB: &str = "trackmyfi.db";
pub const REPLICA_DB: &str = "trackmyfi-replica.db";
pub const BACKUP_DB: &str = "trackmyfi.db.pre-sync-backup";

pub async fn init(app: &AppHandle) -> Result<Db, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let cfg = crate::sync::read_app_config(app);
    let token = crate::sync::KeyringStore.get().unwrap_or(None);
    let has_creds = cfg.url.is_some() && token.is_some();
    let replica_path = dir.join(REPLICA_DB);
    let replica_exists = replica_path.exists();

    let source = decide_db_source(cfg.enabled, has_creds, replica_exists);

    let (db, mode) = match source {
        DbSource::RemoteReplica => {
            let url = cfg.url.clone().unwrap();
            let token = token.unwrap();
            let db = Builder::new_remote_replica(replica_path, url, token)
                .build()
                .await
                .map_err(|e| e.to_string())?;
            // Pull on startup so this device sees other devices' edits.
            let _ = db.sync().await.map_err(|e| e.to_string())?;
            (db, DbMode::Synced)
        }
        DbSource::LocalReplicaFile => {
            let db = Builder::new_local(replica_path)
                .build()
                .await
                .map_err(|e| e.to_string())?;
            (db, DbMode::Local)
        }
        DbSource::LocalOriginal => {
            let db = Builder::new_local(dir.join(LOCAL_DB))
                .build()
                .await
                .map_err(|e| e.to_string())?;
            (db, DbMode::Local)
        }
    };

    let conn = db.connect().map_err(|e| e.to_string())?;
    crate::migrations::run(&conn).await?;
    Ok(Db { db, mode })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synced_when_enabled_with_creds() {
        assert_eq!(decide_db_source(true, true, false), DbSource::RemoteReplica);
        assert_eq!(decide_db_source(true, true, true), DbSource::RemoteReplica);
    }

    #[test]
    fn falls_back_to_replica_file_when_disabled_but_synced_before() {
        assert_eq!(decide_db_source(false, true, true), DbSource::LocalReplicaFile);
        assert_eq!(decide_db_source(false, false, true), DbSource::LocalReplicaFile);
    }

    #[test]
    fn uses_original_when_never_synced() {
        assert_eq!(decide_db_source(false, false, false), DbSource::LocalOriginal);
        // Enabled flag set but creds missing => not synced yet.
        assert_eq!(decide_db_source(true, false, false), DbSource::LocalOriginal);
    }
}
```

> Note: this references `crate::sync::KeyringStore` and its `.get()` — created in Task 4. The crate will not fully compile until Task 4 lands; that is expected in subagent-driven order. To compile/test Task 2 in isolation, run Task 4 first or temporarily stub `KeyringStore`. The recommended execution order is Task 1 → 4 → 2 → 3 → 5 → 6 → 7 → 8 → 9 → 10.

- [ ] **Step 3: Run the pure-decision tests**

Run: `cd src-tauri && cargo test db::tests`
Expected: three tests PASS (after Task 4 provides `KeyringStore`).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/db.rs src-tauri/src/sync.rs
git commit -m "feat(sync): branch db::init on config + decide_db_source"
```

---

### Task 3: Bootstrap table-copy helpers

**Files:**
- Modify: `src-tauri/src/sync.rs`

- [ ] **Step 1: Write the failing copy test**

Append to `sync.rs` above the existing `#[cfg(test)]` block:

```rust
use libsql::Connection;

/// App data tables to copy during bootstrap — everything except sqlite internals
/// and the migration bookkeeping table (the replica runs migrations itself).
pub async fn list_data_tables(conn: &Connection) -> Result<Vec<String>, String> {
    let mut rows = conn
        .query(
            "SELECT name FROM sqlite_master WHERE type='table' \
             AND name NOT LIKE 'sqlite_%' AND name != 'schema_migrations' ORDER BY name",
            (),
        )
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(r.get::<String>(0).map_err(|e| e.to_string())?);
    }
    Ok(out)
}

async fn copy_table(src: &Connection, dst: &Connection, table: &str) -> Result<usize, String> {
    let mut rows = src
        .query(&format!("SELECT * FROM \"{table}\""), ())
        .await
        .map_err(|e| e.to_string())?;
    let ncols = rows.column_count();
    let placeholders = std::iter::repeat("?")
        .take(ncols as usize)
        .collect::<Vec<_>>()
        .join(",");
    let insert = format!("INSERT INTO \"{table}\" VALUES ({placeholders})");
    let mut count = 0usize;
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        let mut vals: Vec<libsql::Value> = Vec::with_capacity(ncols as usize);
        for i in 0..ncols {
            vals.push(row.get_value(i).map_err(|e| e.to_string())?);
        }
        dst.execute(&insert, libsql::params_from_iter(vals))
            .await
            .map_err(|e| e.to_string())?;
        count += 1;
    }
    Ok(count)
}

/// Copy every app data table from `src` into `dst`. Assumes both have identical schema.
pub async fn copy_all_data(src: &Connection, dst: &Connection) -> Result<usize, String> {
    // FK enforcement is off by default in libSQL; be explicit so child-before-parent
    // insert order can never fail the copy.
    let _ = dst.execute("PRAGMA foreign_keys=OFF", ()).await;
    let mut total = 0usize;
    for table in list_data_tables(src).await? {
        total += copy_table(src, dst, &table).await?;
    }
    Ok(total)
}
```

Add this test inside the existing `#[cfg(test)] mod tests` block in `sync.rs`:

```rust
    use libsql::Builder;

    async fn open_local(name: &str) -> libsql::Database {
        let p = std::env::temp_dir().join(format!("trackmyfi_copytest_{name}.db"));
        let _ = std::fs::remove_file(&p);
        Builder::new_local(p).build().await.unwrap()
    }

    #[tokio::test]
    async fn copies_rows_between_dbs() {
        let src_db = open_local("src").await;
        let dst_db = open_local("dst").await;
        let src = src_db.connect().unwrap();
        let dst = dst_db.connect().unwrap();

        for c in [&src, &dst] {
            c.execute("CREATE TABLE account (id INTEGER PRIMARY KEY, name TEXT)", ())
                .await
                .unwrap();
            c.execute("CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, name TEXT)", ())
                .await
                .unwrap();
        }
        src.execute("INSERT INTO account VALUES (1, 'Checking')", ()).await.unwrap();
        src.execute("INSERT INTO account VALUES (2, 'Brokerage')", ()).await.unwrap();
        // schema_migrations must NOT be copied (excluded from data tables).
        src.execute("INSERT INTO schema_migrations VALUES (1, 'init')", ()).await.unwrap();

        let copied = copy_all_data(&src, &dst).await.unwrap();
        assert_eq!(copied, 2);

        let mut rows = dst.query("SELECT count(*) FROM account", ()).await.unwrap();
        let n: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
        assert_eq!(n, 2);

        let mut mig = dst.query("SELECT count(*) FROM schema_migrations", ()).await.unwrap();
        let m: i64 = mig.next().await.unwrap().unwrap().get(0).unwrap();
        assert_eq!(m, 0, "schema_migrations must not be copied");
    }
```

- [ ] **Step 2: Run the test to verify it passes**

Run: `cd src-tauri && cargo test sync::tests::copies_rows_between_dbs`
Expected: PASS (2 rows copied, `schema_migrations` excluded).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/sync.rs
git commit -m "feat(sync): bootstrap table-copy helpers (copy_all_data)"
```

---

### Task 4: `TokenStore` trait + `KeyringStore`

**Files:**
- Modify: `src-tauri/src/sync.rs`

- [ ] **Step 1: Add the trait + keyring implementation**

Append to `sync.rs` (above the test module):

```rust
const KEYCHAIN_SERVICE: &str = "com.trackmyfi.app";
const KEYCHAIN_USER: &str = "turso-sync-token";

/// Abstraction over secret storage so tests never touch the real OS keychain.
pub trait TokenStore: Send + Sync {
    fn get(&self) -> Result<Option<String>, String>;
    fn set(&self, token: &str) -> Result<(), String>;
    fn delete(&self) -> Result<(), String>;
}

pub struct KeyringStore;

impl TokenStore for KeyringStore {
    fn get(&self) -> Result<Option<String>, String> {
        let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_USER).map_err(|e| e.to_string())?;
        match entry.get_password() {
            Ok(p) => Ok(Some(p)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
    fn set(&self, token: &str) -> Result<(), String> {
        let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_USER).map_err(|e| e.to_string())?;
        entry.set_password(token).map_err(|e| e.to_string())
    }
    fn delete(&self) -> Result<(), String> {
        let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_USER).map_err(|e| e.to_string())?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}
```

- [ ] **Step 2: Verify the crate compiles**

Run: `cd src-tauri && cargo build`
Expected: builds clean (now that `KeyringStore` exists, Task 2's `db.rs` reference resolves).

- [ ] **Step 3: Run the full Rust test suite so far**

Run: `cd src-tauri && cargo test`
Expected: existing 47 tests + new sync/db tests all PASS.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/sync.rs
git commit -m "feat(sync): TokenStore trait + KeyringStore (OS keychain)"
```

---

### Task 5: `SyncStatus` type, managed state, `do_sync`, status/sync-now commands

**Files:**
- Modify: `src-tauri/src/sync.rs`

- [ ] **Step 1: Add the status type, shared state, timestamp helper, and `do_sync`**

Append to `sync.rs` (above the test module). Add `use tauri::Emitter;` to the existing imports at the top of the file.

```rust
use std::sync::Mutex as StdMutex;
use tokio::sync::Mutex as AsyncMutex;
use ts_rs::TS;

/// Background sync interval. Lifecycle (startup pull + close push) does the real
/// work; this is a backstop for long-open sessions. One-line tunable.
pub const SYNC_INTERVAL_SECS: u64 = 900; // 15 minutes

#[derive(Serialize, Clone, Debug, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct SyncStatus {
    /// "local" | "synced"
    pub mode: String,
    /// "idle" | "syncing" | "error"
    pub status: String,
    /// epoch milliseconds of last successful sync, or null
    pub last_synced_at: Option<i64>,
    pub last_error: Option<String>,
}

impl SyncStatus {
    pub fn local() -> Self {
        Self { mode: "local".into(), status: "idle".into(), last_synced_at: None, last_error: None }
    }
    pub fn synced_idle() -> Self {
        Self { mode: "synced".into(), status: "idle".into(), last_synced_at: None, last_error: None }
    }
}

/// Managed state: current status snapshot + a lock serializing concurrent syncs.
pub struct SyncShared {
    pub status: StdMutex<SyncStatus>,
    pub lock: AsyncMutex<()>,
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn snapshot(app: &AppHandle) -> SyncStatus {
    app.state::<SyncShared>().status.lock().unwrap().clone()
}

fn emit_status(app: &AppHandle) {
    let _ = app.emit("sync-status", snapshot(app));
}

/// The single funnel all sync triggers call. No-op (returns Ok) when not in synced mode.
pub async fn do_sync(app: &AppHandle) -> Result<(), String> {
    let db = app.state::<crate::db::Db>();
    if !db.is_synced() {
        return Ok(());
    }
    let shared = app.state::<SyncShared>();
    let _guard = shared.lock.lock().await; // serialize timer vs. manual click
    {
        let mut s = shared.status.lock().unwrap();
        s.status = "syncing".into();
    }
    emit_status(&app);

    let result = db.db.sync().await;

    {
        let mut s = shared.status.lock().unwrap();
        match &result {
            Ok(_) => {
                s.status = "idle".into();
                s.last_synced_at = Some(now_ms());
                s.last_error = None;
            }
            Err(e) => {
                s.status = "error".into();
                s.last_error = Some(e.to_string());
            }
        }
    }
    emit_status(&app);
    result.map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_sync_status(app: AppHandle) -> Result<SyncStatus, String> {
    Ok(snapshot(&app))
}

#[tauri::command]
pub async fn sync_now(app: AppHandle) -> Result<(), String> {
    do_sync(&app).await
}
```

- [ ] **Step 2: Generate the TypeScript type via ts-rs**

Run: `cd src-tauri && cargo test`
Expected: PASS, and `src/lib/types/SyncStatus.ts` is created (ts-rs export test runs during `cargo test`).

- [ ] **Step 3: Confirm the generated type**

Run: `cat src/lib/types/SyncStatus.ts`
Expected: an exported `SyncStatus` interface with `mode: string`, `status: string`, `lastSyncedAt: number | null`, `lastError: string | null`.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/sync.rs src/lib/types/SyncStatus.ts
git commit -m "feat(sync): SyncStatus type, shared state, do_sync, status/sync_now commands"
```

---

### Task 6: Enable / disable commands (bootstrap wiring)

**Files:**
- Modify: `src-tauri/src/sync.rs`

- [ ] **Step 1: Add the enable/disable commands**

Append to `sync.rs` (above the test module):

```rust
use crate::db::{BACKUP_DB, LOCAL_DB, REPLICA_DB};

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// Enable sync: validate creds, bootstrap (seed empty cloud from local OR adopt
/// populated cloud), back up the old local file, persist config + token.
/// Returns a human-readable outcome string for the UI.
#[tauri::command]
pub async fn save_sync_config(app: AppHandle, url: String, token: String) -> Result<String, String> {
    let dir = data_dir(&app)?;
    let replica_path = dir.join(REPLICA_DB);
    let local_path = dir.join(LOCAL_DB);

    // Fresh bootstrap only. If a replica file already exists, do not re-seed.
    if replica_path.exists() {
        return Err("Sync is already set up on this device. Disable it first to reconfigure.".into());
    }

    // Build replica + validate credentials via an initial sync (pull).
    let db = libsql::Builder::new_remote_replica(replica_path.clone(), url.clone(), token.clone())
        .build()
        .await
        .map_err(|e| format!("Could not connect to Turso: {e}"))?;
    if let Err(e) = db.sync().await {
        // Clean up the partial replica so a retry starts clean.
        drop(db);
        let _ = std::fs::remove_file(&replica_path);
        return Err(format!("Could not connect to Turso (check the URL and token): {e}"));
    }

    let conn = db.connect().map_err(|e| e.to_string())?;
    let cloud_tables = list_data_tables(&conn).await?;

    let outcome = if cloud_tables.is_empty() {
        // Empty cloud: create schema locally, copy existing local data up.
        crate::migrations::run(&conn).await?;
        if local_path.exists() {
            let old = libsql::Builder::new_local(local_path.clone())
                .build()
                .await
                .map_err(|e| e.to_string())?;
            let old_conn = old.connect().map_err(|e| e.to_string())?;
            let copied = copy_all_data(&old_conn, &conn).await?;
            db.sync().await.map_err(|e| e.to_string())?; // push everything up
            format!("Sync enabled. Uploaded your existing data ({copied} records) to Turso.")
        } else {
            db.sync().await.map_err(|e| e.to_string())?;
            "Sync enabled. Started a fresh cloud database.".to_string()
        }
    } else {
        // Populated cloud (another device seeded it). Adopt it; do not merge.
        "Sync enabled. This device now mirrors your existing cloud data. \
         Your previous local data on this device was kept as a backup but not merged."
            .to_string()
    };

    // Release the replica handle before the app re-opens it on restart.
    drop(conn);
    drop(db);

    // Back up the old local file (never auto-deleted).
    if local_path.exists() {
        let _ = std::fs::rename(&local_path, dir.join(BACKUP_DB));
    }

    // Persist token (keychain) + config (sync.json).
    KeyringStore.set(&token)?;
    write_app_config(&app, &SyncConfig { enabled: true, url: Some(url), bootstrapped: true })?;

    Ok(outcome)
}

/// Disable sync: stop syncing, delete the token. The replica file is retained and
/// opened locally on next launch, so the latest data is kept.
#[tauri::command]
pub async fn clear_sync_config(app: AppHandle) -> Result<(), String> {
    KeyringStore.delete()?;
    let mut cfg = read_app_config(&app);
    cfg.enabled = false;
    write_app_config(&app, &cfg)?;
    Ok(())
}

/// Restart the app to apply a sync mode change. Diverges (never returns).
#[tauri::command]
pub fn restart_app(app: AppHandle) {
    app.restart();
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd src-tauri && cargo build`
Expected: builds clean.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/sync.rs
git commit -m "feat(sync): enable/disable commands with first-run bootstrap"
```

---

### Task 7: Wire state, commands, timer, and exit-sync in `lib.rs`

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Manage `SyncShared`, set initial status, register commands**

In `src-tauri/src/lib.rs`:

1. Add imports near the top: `use tauri::{Manager, RunEvent};` (replace the existing `use tauri::Manager;`).
2. In the `.setup(|app| { … })` closure, after `app.manage(db);`, insert:

```rust
            // Seed sync status from the DB mode and manage shared sync state.
            let initial = if app.state::<db::Db>().is_synced() {
                sync::SyncStatus::synced_idle()
            } else {
                sync::SyncStatus::local()
            };
            app.manage(sync::SyncShared {
                status: std::sync::Mutex::new(initial),
                lock: tokio::sync::Mutex::new(()),
            });

            // Background backstop sync (only meaningful in synced mode; do_sync no-ops otherwise).
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut tick =
                    tokio::time::interval(std::time::Duration::from_secs(sync::SYNC_INTERVAL_SECS));
                tick.tick().await; // consume the immediate first tick
                loop {
                    tick.tick().await;
                    let _ = sync::do_sync(&handle).await;
                }
            });
```

3. In `invoke_handler(tauri::generate_handler![ … ])`, append these entries (after the budget commands):

```rust
            sync::get_sync_status,
            sync::sync_now,
            sync::save_sync_config,
            sync::clear_sync_config,
            sync::restart_app,
```

- [ ] **Step 2: Add the exit-sync run loop**

Replace the final:

```rust
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
```

with:

```rust
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|handle, event| {
            if let RunEvent::ExitRequested { .. } = event {
                // Best-effort final push so another device sees this session's edits.
                let _ = tauri::async_runtime::block_on(sync::do_sync(handle));
            }
        });
```

- [ ] **Step 3: Add `tokio` as a runtime dependency**

`do_sync`'s timer uses `tokio::time` / `tokio::sync` at runtime (tokio is currently only a dev-dependency). In `src-tauri/Cargo.toml` under `[dependencies]`, add:

```toml
tokio = { version = "1", features = ["sync", "time"] }
```

- [ ] **Step 4: Verify the whole backend builds and tests pass**

Run: `cd src-tauri && cargo build && cargo test`
Expected: clean build; all tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "feat(sync): manage state, register commands, timer + exit-sync"
```

---

### Task 8: Frontend API wrapper + Pinia store

**Files:**
- Create: `src/lib/api/sync.ts`
- Create: `src/stores/sync.ts`
- Modify: `src/App.vue`

- [ ] **Step 1: Create the API wrapper**

Create `src/lib/api/sync.ts`:

```ts
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { SyncStatus } from '../types/SyncStatus'

export const getSyncStatus = () => invoke<SyncStatus>('get_sync_status')

export const saveSyncConfig = (url: string, token: string) =>
  invoke<string>('save_sync_config', { url, token })

export const clearSyncConfig = () => invoke<void>('clear_sync_config')

export const syncNow = () => invoke<void>('sync_now')

export const restartApp = () => invoke<void>('restart_app')

export const onSyncStatus = (cb: (s: SyncStatus) => void): Promise<UnlistenFn> =>
  listen<SyncStatus>('sync-status', (e) => cb(e.payload))
```

- [ ] **Step 2: Create the store**

Create `src/stores/sync.ts`:

```ts
import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { SyncStatus } from '../lib/types/SyncStatus'
import { getSyncStatus, onSyncStatus } from '../lib/api/sync'

export const useSyncStore = defineStore('sync', () => {
  const status = ref<SyncStatus | null>(null)
  let subscribed = false

  async function init() {
    status.value = await getSyncStatus()
    if (!subscribed) {
      subscribed = true
      await onSyncStatus((s) => {
        status.value = s
      })
    }
  }

  function setStatus(s: SyncStatus) {
    status.value = s
  }

  return { status, init, setStatus }
})
```

- [ ] **Step 3: Initialize the store on app mount**

In `src/App.vue`, inside `<script setup>`, add (matching the file's existing import style):

```ts
import { onMounted } from 'vue'
import { useSyncStore } from './stores/sync'

const syncStore = useSyncStore()
onMounted(() => {
  syncStore.init()
})
```

> If `App.vue` already has an `onMounted`, merge the `syncStore.init()` call into it rather than adding a second one.

- [ ] **Step 4: Verify the frontend typechecks**

Run: `npm run build`
Expected: `vue-tsc` + `vite build` complete with no errors.

- [ ] **Step 5: Commit**

```bash
git add src/lib/api/sync.ts src/stores/sync.ts src/App.vue
git commit -m "feat(sync): frontend api wrapper + sync store with live status"
```

---

### Task 9: Settings → Sync panel + setup instructions

**Files:**
- Modify: `src/pages/Settings.vue`

- [ ] **Step 1: Add sync state + actions to the script**

In `src/pages/Settings.vue` `<script setup>`, add after the existing fireProfile setup:

```ts
import { ref, computed } from 'vue'
import { confirm } from '@tauri-apps/plugin-dialog'
import { useSyncStore } from '../stores/sync'
import {
  saveSyncConfig,
  clearSyncConfig,
  syncNow,
  restartApp,
} from '../lib/api/sync'

const syncStore = useSyncStore()
const syncUrl = ref('')
const syncToken = ref('')
const syncBusy = ref(false)
const syncMessage = ref('')

const isSynced = computed(() => syncStore.status?.mode === 'synced')
const lastSynced = computed(() => {
  const ms = syncStore.status?.lastSyncedAt
  return ms ? new Date(ms).toLocaleString() : 'never'
})

async function enableSync() {
  if (!syncUrl.value || !syncToken.value) {
    syncMessage.value = 'Enter both the database URL and the auth token.'
    return
  }
  syncBusy.value = true
  syncMessage.value = ''
  try {
    const outcome = await saveSyncConfig(syncUrl.value.trim(), syncToken.value.trim())
    syncToken.value = ''
    const restart = await confirm(`${outcome}\n\nRestart now to start syncing?`, {
      title: 'Sync enabled',
      kind: 'info',
    })
    if (restart) await restartApp()
  } catch (e) {
    syncMessage.value = String(e)
  } finally {
    syncBusy.value = false
  }
}

async function disableSync() {
  const ok = await confirm(
    'Stop syncing on this device? Your data stays on this machine; the cloud copy is left untouched.',
    { title: 'Disable sync', kind: 'warning' },
  )
  if (!ok) return
  syncBusy.value = true
  try {
    await clearSyncConfig()
    const restart = await confirm('Sync disabled. Restart now to apply?', {
      title: 'Disable sync',
      kind: 'info',
    })
    if (restart) await restartApp()
  } catch (e) {
    syncMessage.value = String(e)
  } finally {
    syncBusy.value = false
  }
}

async function runSyncNow() {
  syncBusy.value = true
  try {
    await syncNow()
  } catch (e) {
    syncMessage.value = String(e)
  } finally {
    syncBusy.value = false
  }
}
```

- [ ] **Step 2: Add the panel + instructions to the template**

In `Settings.vue`, after the closing `</UForm>` but before the final `</div>`, add:

```vue
    <hr class="my-8 border-default" />

    <section class="space-y-3">
      <h2 class="text-xl font-bold">Cloud Sync (Turso)</h2>
      <p class="text-sm text-muted">
        Optional. Keeps an encrypted-at-rest backup in your own free Turso database and
        reconciles your data across machines. The app works fully offline without this.
      </p>

      <div class="text-sm">
        Status:
        <span class="font-medium">{{ isSynced ? 'Syncing' : 'Local only' }}</span>
        <span v-if="isSynced"> · last synced {{ lastSynced }}</span>
        <span v-if="syncStore.status?.status === 'syncing'"> · syncing…</span>
        <span v-if="syncStore.status?.lastError" class="text-error">
          · {{ syncStore.status.lastError }}
        </span>
      </div>

      <template v-if="!isSynced">
        <UFormField label="Database URL">
          <UInput v-model="syncUrl" placeholder="libsql://your-db-name-you.turso.io" class="w-full" />
        </UFormField>
        <UFormField label="Auth token (treated like a password)">
          <UInput v-model="syncToken" type="password" class="w-full" />
        </UFormField>
        <UButton :loading="syncBusy" @click="enableSync">Enable sync</UButton>
      </template>

      <template v-else>
        <div class="flex gap-2">
          <UButton :loading="syncBusy" @click="runSyncNow">Sync now</UButton>
          <UButton color="error" variant="soft" :loading="syncBusy" @click="disableSync">
            Disable sync
          </UButton>
        </div>
      </template>

      <p v-if="syncMessage" class="text-sm text-error">{{ syncMessage }}</p>

      <UAccordion
        :items="[{ label: 'How to set this up', slot: 'help', icon: 'i-lucide-circle-help' }]"
      >
        <template #help>
          <div class="text-sm space-y-4 p-2">
            <div>
              <h3 class="font-semibold mb-1">Option A — Turso website (no terminal)</h3>
              <ol class="list-decimal ml-5 space-y-1">
                <li>Go to <span class="font-mono">turso.tech</span> and create a free account.</li>
                <li>Click <strong>Create Database</strong> and give it a name (e.g. <em>trackmyfi</em>).</li>
                <li>Open the database, find <strong>Database URL</strong>, and copy it (starts with <span class="font-mono">libsql://</span>).</li>
                <li>Create a database token (look for <strong>Tokens</strong> / <strong>Create Token</strong>) and copy it.</li>
                <li>Paste both above and click <strong>Enable sync</strong>.</li>
              </ol>
            </div>
            <div>
              <h3 class="font-semibold mb-1">Option B — Turso CLI (technical)</h3>
              <pre class="bg-elevated rounded p-2 overflow-x-auto"><code>turso auth signup
turso db create trackmyfi
turso db show trackmyfi --url        # the Database URL
turso db tokens create trackmyfi     # the auth token</code></pre>
              <p class="mt-1">Paste the URL and token above and click <strong>Enable sync</strong>.</p>
            </div>
            <p class="text-muted">
              It's free-tier. The URL isn't secret, but the token is — treat it like a password.
              You own this cloud database.
            </p>
          </div>
        </template>
      </UAccordion>
    </section>
```

> Verify `UAccordion`'s slot API against the installed NuxtUI 4 (`components.d.ts` / NuxtUI docs). If the `slot`-name pattern differs in this version, use the documented per-item content API instead — the content (the two options) stays identical.

- [ ] **Step 3: Typecheck**

Run: `npm run build`
Expected: clean.

- [ ] **Step 4: Browser smoke (render only — backend invoke won't work outside Tauri)**

Verify the Settings page renders the new section with the form, status line, and the "How to set this up" accordion showing both options. (Use the preview workflow.)

- [ ] **Step 5: Commit**

```bash
git add src/pages/Settings.vue
git commit -m "feat(sync): Settings panel with enable/disable + setup instructions"
```

---

### Task 10: Docs, memory, and full verification

**Files:**
- Modify: `README.md`
- Update: project memory

- [ ] **Step 1: Add a condensed sync section to `README.md`**

Add a "Cloud Sync (optional)" section: the app is local-first; to enable sync, create a free Turso DB (website or `turso db create`), copy the Database URL + a token, paste them in Settings → Cloud Sync, and restart. Note the token lives in the OS keychain and the data file is covered by OS full-disk encryption.

- [ ] **Step 2: Run the full automated gate**

Run:
```bash
cd src-tauri && cargo test && cargo build && cd .. && npm test && npm run build
```
Expected: all Rust tests pass, cargo builds, Vitest passes (95+), `vue-tsc` + `vite build` clean.

- [ ] **Step 3: Manual smoke test (documented — requires a real Turso DB)**

Walk this checklist with `npm run tauri dev`:
1. With existing local data, open Settings → Cloud Sync → paste a fresh empty Turso DB's URL + token → Enable. Expect the "Uploaded your existing data (N records)" message → restart.
2. After restart, status shows "Syncing", last-synced is recent. Verify the cloud has data via `turso db shell <db> "SELECT count(*) FROM account;"`.
3. Make an edit (add a balance), click "Sync now", confirm the change reaches the cloud.
4. Confirm `trackmyfi.db.pre-sync-backup` exists in the app data dir.
5. Disable sync → restart → confirm the app still shows the latest data (now reading `trackmyfi-replica.db` locally).
6. Bad-credential path: enter a wrong token → expect a clear error and no mode change (no replica file left behind).

- [ ] **Step 4: Update project memory**

Update `project_trackmyfi_design.md`: mark "background Turso sync" as BUILT (date), record the two-file model, `sync.json` + keychain split, `SYNC_INTERVAL_SECS = 900`, and the bootstrap-copy approach. Add a one-line pointer if needed.

- [ ] **Step 5: Commit**

```bash
git add README.md
git commit -m "docs(sync): document optional Turso cloud sync setup"
```

---

## Self-Review

**Spec coverage:**
- Opt-in / local-first default → Task 2 (`decide_db_source`, branching `init`). ✓
- Config outside DB (keychain + sync.json) → Tasks 1, 4. ✓
- Triggers (startup pull / timer / close push / manual) → Task 2 (startup), Task 7 (timer + exit), Task 5 (`sync_now`). ✓
- 15-min constant, not in UI → Task 5 (`SYNC_INTERVAL_SECS`). ✓
- Status state + `sync-status` event → Tasks 5, 8. ✓
- Offline = quiet error, retried → `do_sync` sets `error`, timer keeps ticking. ✓
- Two-file model + file selection table → Task 2. ✓
- First-enable bootstrap (validate → empty? seed : adopt → backup → persist) → Task 6. ✓
- Disable keeps replica file as local → Task 6 + Task 2 fallback. ✓
- Restart prompt → Task 9 (`confirm` + `restart_app`). ✓
- Setup instructions (dashboard + CLI) → Task 9. ✓
- Testing (config round-trip, file-selection, table-copy, keychain behind trait) → Tasks 1, 2, 3, 4. ✓
- Out of scope (merge two populated DBs) → Task 6 adopts cloud, warns, keeps backup. ✓

**Placeholder scan:** Two explicit "verify against installed version" notes remain (NuxtUI `UAccordion` slot API in Task 9; execution-order note in Task 2). These are genuine version-surface checks, not missing content — the full code is present either way.

**Type consistency:** `Db { db, mode }` / `is_synced()` used consistently across db.rs, sync.rs, lib.rs. `SyncStatus` fields (`mode`, `status`, `lastSyncedAt`, `lastError`) consistent Rust↔TS↔Vue. Commands `get_sync_status`/`sync_now`/`save_sync_config`/`clear_sync_config`/`restart_app` match between Rust definitions, lib.rs registration, and `src/lib/api/sync.ts`.
