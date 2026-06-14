use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex as StdMutex;
use tokio::sync::Mutex as AsyncMutex;
use ts_rs::TS;

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

use tauri::{AppHandle, Emitter, Manager};

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

// Relies on `SELECT *` column order matching the destination's positional
// `INSERT ... VALUES`; this holds only because src and dst share an identical
// schema (both run the same migrations).
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
    // insert order can never fail the copy. Must run BEFORE BEGIN — this PRAGMA is a
    // no-op inside a transaction.
    dst.execute("PRAGMA foreign_keys=OFF", ())
        .await
        .map_err(|e| e.to_string())?;

    // Wrap the whole copy in one transaction so a mid-copy failure rolls back
    // rather than leaving the destination half-populated.
    dst.execute("BEGIN", ()).await.map_err(|e| e.to_string())?;
    let mut total = 0usize;
    for table in list_data_tables(src).await? {
        match copy_table(src, dst, &table).await {
            Ok(n) => total += n,
            Err(e) => {
                let _ = dst.execute("ROLLBACK", ()).await;
                return Err(e);
            }
        }
    }
    dst.execute("COMMIT", ()).await.map_err(|e| e.to_string())?;
    Ok(total)
}

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
    #[ts(type = "number | null")]
    pub last_synced_at: Option<i64>,
    pub last_error: Option<String>,
}

impl SyncStatus {
    pub fn local() -> Self {
        Self { mode: "local".into(), status: "idle".into(), last_synced_at: None, last_error: None }
    }
    pub fn synced_just_now() -> Self {
        Self { mode: "synced".into(), status: "idle".into(), last_synced_at: Some(now_ms()), last_error: None }
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
    emit_status(app);

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
    emit_status(app);
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
    let mut cfg = read_app_config(&app);
    cfg.enabled = false;
    write_app_config(&app, &cfg)?;
    KeyringStore.delete()?;
    Ok(())
}

/// Request an app restart to apply a sync mode change.
#[tauri::command]
pub fn restart_app(app: AppHandle) {
    app.restart();
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
}
