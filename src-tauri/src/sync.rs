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
    let dir = crate::db::resolve_app_dir(app.path().app_config_dir().map_err(|e| e.to_string())?);
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

/// Keychain service name. Debug builds (`tauri dev`) use a separate entry so a
/// dev session can never read or overwrite the real app's sync token — mirroring
/// the data-dir isolation in `crate::db::resolve_app_dir`.
#[cfg(debug_assertions)]
const KEYCHAIN_SERVICE: &str = "com.trackmyfi.desktop.dev";
#[cfg(not(debug_assertions))]
const KEYCHAIN_SERVICE: &str = "com.trackmyfi.desktop";
const KEYCHAIN_USER: &str = "turso-sync-token";

/// Abstraction over secret storage so tests never touch the real OS keychain.
pub trait TokenStore: Send + Sync {
    fn get(&self) -> Result<Option<String>, String>;
    fn set(&self, token: &str) -> Result<(), String>;
    fn delete(&self) -> Result<(), String>;
}

pub struct KeyringStore;

// On macOS we read and write keychain items by spawning Apple's `/usr/bin/security`
// tool instead of calling Security.framework from our own process.
//
// Why: since macOS Sierra every login-keychain item carries a PARTITION ID list —
// a second access gate, separate from the classic ACL application list. It records
// which signing identities may use the item silently: "teamid:XXX" for Developer ID
// apps, "apple-tool:" for Apple's `security` CLI, or a bare "cdhash:..." for apps
// signed without a team ID (our self-signed cert has none). A cdhash changes on
// EVERY build, so items created by the app in-process are pinned to one release:
// after each auto-update macOS shows the "enter the login keychain password" prompt
// again, and "Always Allow" only whitelists the current build's hash. Empirically
// (tested on macOS 15/26): the partition list cannot be stripped, pre-seeded, or
// edited without the keychain password, and the modern data-protection keychain
// kills self-signed binaries that claim its entitlements. The only stable identity
// available to a self-signed app is Apple's own `security` tool: items it creates
// get partition "apple-tool:", which every future invocation matches — so reads
// and writes stay prompt-free across app updates forever.
//
// Access model: `-A` marks the item readable by any application without a prompt —
// the same posture the previous in-process implementation deliberately configured.
//
// Migration: items written by the old in-process code are pinned to an old cdhash,
// so the FIRST read after shipping this change prompts once more. "Always Allow"
// then whitelists `apple-tool:` permanently, and get() rewrites the item so it is
// CLI-owned from then on.
#[cfg(target_os = "macos")]
pub mod macos_keychain {
    use std::io::Write;
    use std::process::{Command, Stdio};

    const SECURITY: &str = "/usr/bin/security";
    // `security` exits 44 (errSecItemNotFound) when no item matches.
    const NOT_FOUND_EXIT: i32 = 44;

    /// Stored values are ASCII-armored as "hex:" + lowercase hex of the UTF-8 bytes.
    /// `find-generic-password -w` prints the raw value only when it is printable
    /// ASCII and silently switches to hex output otherwise; the prefix makes the
    /// stored form unambiguous and lets arbitrary secrets (JSON, unicode) round-trip.
    const HEX_PREFIX: &str = "hex:";

    fn hex_encode(bytes: &[u8]) -> String {
        use std::fmt::Write as _;
        bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
    }

    fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
        if s.len() % 2 != 0 {
            return Err("invalid hex value in keychain item".into());
        }
        (0..s.len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&s[i..i + 2], 16)
                    .map_err(|_| "invalid hex value in keychain item".to_string())
            })
            .collect()
    }

    pub fn get(service: &str, account: &str) -> Result<Option<String>, String> {
        let out = Command::new(SECURITY)
            .args(["find-generic-password", "-s", service, "-a", account, "-w"])
            .output()
            .map_err(|e| format!("could not run security tool: {e}"))?;
        if !out.status.success() {
            if out.status.code() == Some(NOT_FOUND_EXIT) {
                return Ok(None);
            }
            return Err(format!(
                "keychain read failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        let mut raw = String::from_utf8(out.stdout).map_err(|e| e.to_string())?;
        if raw.ends_with('\n') {
            raw.pop();
        }
        match raw.strip_prefix(HEX_PREFIX) {
            Some(hex) => {
                let value = String::from_utf8(hex_decode(hex)?).map_err(|e| e.to_string())?;
                Ok(Some(value))
            }
            None => {
                // Legacy item written by the old in-process code (raw, cdhash-pinned).
                // Rewrite it once so it becomes CLI-owned and prefix-encoded; every
                // read after this — including after future app updates — is silent.
                let _ = set(service, account, &raw);
                Ok(Some(raw))
            }
        }
    }

    /// `service` and `account` must be plain identifiers (no whitespace/quotes) —
    /// they are interpolated into a `security -i` command line. All call sites use
    /// the fixed constants defined in this crate.
    pub fn set(service: &str, account: &str, password: &str) -> Result<(), String> {
        // Recreate rather than update in place: a fresh item is born with the
        // apple-tool: partition and the -A any-app ACL regardless of what owned
        // the old one. (Deletion is not gated by the partition list.)
        let _ = delete(service, account);

        let payload = format!("{HEX_PREFIX}{}", hex_encode(password.as_bytes()));
        // -i reads subcommands from stdin, keeping the secret out of argv (visible
        // via ps); -X passes it as hex, so no CLI quoting rules apply to the value.
        let line = format!(
            "add-generic-password -A -s {service} -a {account} -l {service} -X {}\n",
            hex_encode(payload.as_bytes())
        );
        let mut child = Command::new(SECURITY)
            .arg("-i")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("could not run security tool: {e}"))?;
        child
            .stdin
            .take()
            .expect("stdin was piped")
            .write_all(line.as_bytes())
            .map_err(|e| e.to_string())?;
        let out = child.wait_with_output().map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(())
        } else {
            Err(format!(
                "keychain write failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ))
        }
    }

    pub fn delete(service: &str, account: &str) -> Result<(), String> {
        let out = Command::new(SECURITY)
            .args(["delete-generic-password", "-s", service, "-a", account])
            .stdout(Stdio::null())
            .output()
            .map_err(|e| format!("could not run security tool: {e}"))?;
        if out.status.success() || out.status.code() == Some(NOT_FOUND_EXIT) {
            Ok(())
        } else {
            Err(format!(
                "keychain delete failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ))
        }
    }
}

#[cfg(target_os = "macos")]
impl TokenStore for KeyringStore {
    fn get(&self) -> Result<Option<String>, String> {
        macos_keychain::get(KEYCHAIN_SERVICE, KEYCHAIN_USER)
    }
    fn set(&self, token: &str) -> Result<(), String> {
        macos_keychain::set(KEYCHAIN_SERVICE, KEYCHAIN_USER, token)
    }
    fn delete(&self) -> Result<(), String> {
        macos_keychain::delete(KEYCHAIN_SERVICE, KEYCHAIN_USER)
    }
}

#[cfg(not(target_os = "macos"))]
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

// Makes `dst.table` mirror `src.table`. Clears the destination first so any rows
// the migrations seeded into the freshly-created replica (e.g. the default
// `fire_profile` id=1) are replaced rather than colliding on primary key.
// Relies on `SELECT *` column order matching the destination's positional
// `INSERT ... VALUES`; this holds only because src and dst share an identical
// schema (both run the same migrations).
async fn copy_table(src: &Connection, dst: &Connection, table: &str) -> Result<usize, String> {
    dst.execute(&format!("DELETE FROM \"{table}\""), ())
        .await
        .map_err(|e| e.to_string())?;
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

/// Copy every app data table from `src` into `dst`. Assumes both have identical
/// schema (both run the same migrations).
///
/// Writes are auto-committed, with NO explicit `BEGIN`/`COMMIT` and no
/// `conn.transaction()`. This is deliberate: `dst` is a direct remote Turso
/// connection, where `transaction()` is unimplemented (panics) and a client
/// transaction's writes are unreliable. FK enforcement is off by default on a
/// libSQL/Turso connection, so per-table clear+insert order is safe. Atomicity
/// on failure is provided one level up — a failed bootstrap discards the whole
/// replica file and the caller treats the cloud DB as disposable on retry.
pub async fn copy_all_data(src: &Connection, dst: &Connection) -> Result<usize, String> {
    let mut total = 0usize;
    for table in list_data_tables(src).await? {
        total += copy_table(src, dst, &table).await?;
    }
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

/// Rendezvous that guarantees the post-catch-up `data-refreshed` event is emitted
/// only AFTER both (a) the background catch-up has finished pulling + migrating and
/// (b) the frontend has attached its `data-refreshed` listener.
///
/// Tauri events are not buffered: one emitted before `listen()` has registered is
/// lost forever. The catch-up is spawned the instant the DB opens, so in a release
/// build it can easily finish before the webview's listener exists — which silently
/// dropped the refresh and left the UI showing stale/empty last-synced data until
/// the user navigated away and back. This gate closes that race: whichever of the
/// two conditions completes second triggers the (single) emit.
#[derive(Default)]
struct RefreshGateState {
    frontend_ready: bool,
    catch_up_done: bool,
    emitted: bool,
}

pub struct RefreshGate {
    inner: StdMutex<RefreshGateState>,
}

impl RefreshGate {
    pub fn new() -> Self {
        Self { inner: StdMutex::new(RefreshGateState::default()) }
    }
}

/// Update one side of the gate, then emit `data-refreshed` exactly once if both
/// sides are now satisfied.
fn update_refresh_gate(app: &AppHandle, set: impl FnOnce(&mut RefreshGateState)) {
    let gate = app.state::<RefreshGate>();
    let should_emit = {
        let mut s = gate.inner.lock().unwrap();
        set(&mut s);
        if s.frontend_ready && s.catch_up_done && !s.emitted {
            s.emitted = true;
            true
        } else {
            false
        }
    };
    if should_emit {
        let _ = app.emit("data-refreshed", ());
    }
}

/// Frontend handshake: called once the webview has registered its `data-refreshed`
/// listener, so the backend knows the refresh emit can no longer be missed.
#[tauri::command]
pub fn frontend_ready(app: AppHandle) {
    update_refresh_gate(&app, |s| s.frontend_ready = true);
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

/// Startup catch-up for synced mode, run in the background so the app is
/// interactive immediately (serving last-synced data from the local replica file
/// — see `crate::db::init`, which no longer blocks startup on a network pull).
///
/// Order matters: pull first, THEN migrate. A pull brings down any migration
/// another device already applied to the primary, so this device sees it as
/// applied and skips it rather than re-running it through to the primary. If the
/// pull fails (e.g. offline) we still migrate, so a freshly-updated app has its
/// schema; the rare cross-device double-apply only matters when offline on a new
/// release.
///
/// Emits `data-refreshed` when done so the frontend re-reads — picking up either
/// freshly-pulled remote edits or a just-applied migration.
pub async fn initial_catch_up(app: &AppHandle) -> Result<(), String> {
    let db = app.state::<crate::db::Db>();
    if !db.is_synced() {
        return Ok(());
    }
    let pull = do_sync(app).await;
    let conn = db.conn().await?;
    crate::migrations::run(&conn).await?;
    // Don't emit directly — the frontend listener may not be attached yet. Mark
    // this side of the gate; the emit fires once the frontend is also ready.
    update_refresh_gate(app, |s| s.catch_up_done = true);
    pull
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
    let dir = crate::db::resolve_app_dir(app.path().app_data_dir().map_err(|e| e.to_string())?);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// Best-effort removal of the embedded-replica file and its libSQL sidecars.
/// Used to clean up after a failed bootstrap so a retry starts from scratch.
fn remove_replica_files(dir: &Path) {
    let _ = std::fs::remove_file(dir.join(REPLICA_DB));
    for suffix in ["-info", "-wal", "-shm"] {
        let _ = std::fs::remove_file(dir.join(format!("{REPLICA_DB}{suffix}")));
    }
}

/// Seed the cloud database over a DIRECT remote connection (not the embedded
/// replica). Returns the user-facing outcome string.
///
/// Why direct: writing the local data into an embedded replica and relying on
/// `sync()` to push it does NOT work on Turso's sync protocol — the copied
/// writes are silently dropped (libSQL's own sync code warns about exactly this
/// during bootstrap). Direct Hrana writes are reliable and enforce constraints.
async fn seed_cloud(url: &str, token: &str, local_path: &Path) -> Result<String, String> {
    let remote = libsql::Builder::new_remote(url.to_string(), token.to_string())
        .build()
        .await
        .map_err(|e| format!("Could not connect to Turso: {e}"))?;
    let rconn = remote.connect().map_err(|e| e.to_string())?;

    // The first query authenticates and validates the URL/token.
    let cloud_tables = list_data_tables(&rconn)
        .await
        .map_err(|e| format!("Could not connect to Turso (check the URL and token): {e}"))?;

    if !cloud_tables.is_empty() {
        // Populated cloud (another device seeded it). Adopt it; do not merge.
        return Ok("Sync enabled. This device now mirrors your existing cloud data. \
                   Your previous local data on this device was kept as a backup but not merged."
            .to_string());
    }

    // Empty cloud: create schema + default seeds directly on the cloud, then copy
    // the local data up. copy_all_data clears each table first so the seeded
    // fire_profile (id=1) is replaced rather than colliding.
    crate::migrations::run(&rconn).await?;
    if local_path.exists() {
        let old = libsql::Builder::new_local(local_path.to_path_buf())
            .build()
            .await
            .map_err(|e| e.to_string())?;
        let old_conn = old.connect().map_err(|e| e.to_string())?;
        let copied = copy_all_data(&old_conn, &rconn).await?;
        Ok(format!("Sync enabled. Uploaded your existing data ({copied} records) to Turso."))
    } else {
        Ok("Sync enabled. Started a fresh cloud database.".to_string())
    }
}

/// Enable sync: seed the cloud directly, build the local embedded replica from
/// it, back up the old local file, then persist config + token. Returns a
/// human-readable outcome string for the UI.
#[tauri::command]
pub async fn save_sync_config(app: AppHandle, url: String, token: String) -> Result<String, String> {
    let dir = data_dir(&app)?;
    let replica_path = dir.join(REPLICA_DB);
    let local_path = dir.join(LOCAL_DB);

    // Fresh bootstrap only. If a replica file already exists, do not re-seed.
    if replica_path.exists() {
        return Err("Sync is already set up on this device. Disable it first to reconfigure.".into());
    }

    // Phase 1: seed the cloud over a direct connection (validates creds too).
    let outcome = seed_cloud(&url, &token, &local_path).await?;

    // Phase 2: build the embedded replica and pull the now-populated cloud into
    // it. Any failure here removes the partial replica so a retry starts clean
    // and a restart won't open a half-built replica instead of the real local DB.
    let db = libsql::Builder::new_remote_replica(replica_path.clone(), url.clone(), token.clone())
        .build()
        .await
        .map_err(|e| format!("Could not connect to Turso: {e}"))?;
    if let Err(e) = db.sync().await {
        drop(db);
        remove_replica_files(&dir);
        return Err(format!("Sync setup failed while pulling cloud data: {e}"));
    }
    // Release the replica handle before the app re-opens it on restart.
    drop(db);

    // Phase 3: back up the old local file (never auto-deleted) + persist config.
    if local_path.exists() {
        let _ = std::fs::rename(&local_path, dir.join(BACKUP_DB));
    }
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

    /// Touches the REAL login keychain (a throwaway service name), so it is
    /// ignored by default. Run explicitly with:
    /// `cargo test macos_keychain_round_trip -- --ignored`
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore]
    fn macos_keychain_round_trip() {
        const SVC: &str = "com.trackmyfi.desktop.keychain-test";
        const ACCT: &str = "round-trip";

        // Missing item reads as None; deleting a missing item is Ok.
        macos_keychain::delete(SVC, ACCT).unwrap();
        assert_eq!(macos_keychain::get(SVC, ACCT).unwrap(), None);

        // Values with JSON, unicode, and hex-lookalike content round-trip exactly.
        for value in [
            r#"{"accessKeyId": "AK IA+/=", "secret": "s3\\cr\"et"}"#,
            "café ünïcode ✓",
            "deadbeef00",
            "eyJhbGciOiJFZERTQSJ9.fake.jwt-token",
        ] {
            macos_keychain::set(SVC, ACCT, value).unwrap();
            assert_eq!(macos_keychain::get(SVC, ACCT).unwrap().as_deref(), Some(value));
        }

        // Legacy raw items (written without the hex: prefix) still read back.
        std::process::Command::new("/usr/bin/security")
            .args(["add-generic-password", "-U", "-A", "-s", SVC, "-a", ACCT, "-w", "legacy-raw-token"])
            .output()
            .unwrap();
        assert_eq!(
            macos_keychain::get(SVC, ACCT).unwrap().as_deref(),
            Some("legacy-raw-token")
        );
        // ...and that read migrated the item to the prefixed form on disk.
        let out = std::process::Command::new("/usr/bin/security")
            .args(["find-generic-password", "-s", SVC, "-a", ACCT, "-w"])
            .output()
            .unwrap();
        assert!(String::from_utf8_lossy(&out.stdout).starts_with("hex:"));

        macos_keychain::delete(SVC, ACCT).unwrap();
        assert_eq!(macos_keychain::get(SVC, ACCT).unwrap(), None);
    }

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

    #[tokio::test]
    async fn copy_overwrites_migration_seeded_rows() {
        // Reproduces the bootstrap bug: migrations::run seeds a default
        // fire_profile (id=1) into the destination before copy_all_data runs,
        // so copying the local fire_profile (also id=1) must not collide.
        let src_db = open_local("seed_src").await;
        let dst_db = open_local("seed_dst").await;
        let src = src_db.connect().unwrap();
        let dst = dst_db.connect().unwrap();

        for c in [&src, &dst] {
            c.execute(
                "CREATE TABLE fire_profile (id INTEGER PRIMARY KEY CHECK (id = 1), age INTEGER)",
                (),
            )
            .await
            .unwrap();
            c.execute("CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, name TEXT)", ())
                .await
                .unwrap();
        }
        // dst already holds the migration-seeded default row (what migrations::run does).
        dst.execute("INSERT INTO fire_profile VALUES (1, 30)", ()).await.unwrap();
        // src (the user's local DB) holds the real, edited row at the same id.
        src.execute("INSERT INTO fire_profile VALUES (1, 42)", ()).await.unwrap();

        let copied = copy_all_data(&src, &dst).await.unwrap();
        assert_eq!(copied, 1);

        // The local row must replace the seeded default — exactly one row, with src's value.
        let mut rows = dst.query("SELECT age FROM fire_profile WHERE id = 1", ()).await.unwrap();
        let age: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
        assert_eq!(age, 42, "local row must overwrite the migration-seeded default");
        let mut cnt = dst.query("SELECT count(*) FROM fire_profile", ()).await.unwrap();
        let n: i64 = cnt.next().await.unwrap().unwrap().get(0).unwrap();
        assert_eq!(n, 1);
    }
}
