use crate::sync::TokenStore;
use libsql::{Builder, Connection, Database};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Isolates debug builds (`tauri dev`) into a `dev/` subdirectory so they can
/// never share a database — or sync config — with a release build (the `.dmg`).
///
/// This is decided by the binary at compile time, so it holds no matter which
/// command launched it: you cannot accidentally point `tauri dev` at real data.
pub fn resolve_app_dir(base: PathBuf) -> PathBuf {
    if cfg!(debug_assertions) {
        base.join("dev")
    } else {
        base
    }
}

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

/// Pure decision: which file/builder to use.
///
/// The replica file is preferred over the original ONLY when sync was actually
/// completed (`bootstrapped`) — either still enabled, or deliberately disabled
/// after a real sync. A replica file left behind by a failed/aborted setup,
/// with no bootstrapped config, is ignored so it can never hide the real local
/// DB.
pub fn decide_db_source(
    sync_enabled: bool,
    has_creds: bool,
    bootstrapped: bool,
    replica_exists: bool,
) -> DbSource {
    if sync_enabled && has_creds {
        DbSource::RemoteReplica
    } else if bootstrapped && replica_exists {
        DbSource::LocalReplicaFile
    } else {
        DbSource::LocalOriginal
    }
}

/// Everything needed to rebuild the embedded replica `Database` from scratch —
/// the in-process equivalent of an app restart. Only present in synced mode.
struct ReplicaSpec {
    path: PathBuf,
    url: String,
    token: String,
}

pub struct Db {
    /// Behind an async RwLock so the whole `Database` can be swapped out after
    /// replica corruption (see `ensure_healthy`). Normal use is read-locked;
    /// only a rebuild takes the write lock.
    db: tokio::sync::RwLock<Database>,
    pub mode: DbMode,
    /// Rebuild parameters (synced mode only; local modes never corrupt this way).
    replica: Option<ReplicaSpec>,
    /// One shared connection handed out (as cheap `Arc` clones) to every command.
    ///
    /// Opening a fresh connection per invoke meant a page firing several reads via
    /// `Promise.all` opened a swarm of simultaneous connections to the WAL file.
    /// Overlapping the background `sync()` writer, that swarm blew past the
    /// `busy_timeout` and surfaced as "database is locked". libSQL serialises
    /// operations on a single connection internally, so funnelling all queries
    /// through one connection makes a page's concurrent reads queue onto one
    /// reader instead — leaving the clean WAL model of one reader + the sync
    /// writer, which the busy timeout absorbs.
    ///
    /// Behind a mutex because a Turso `sync()` can REPLACE the replica file
    /// (snapshot re-bootstrap on a generation change); the connection opened at
    /// launch would keep pointing at the dead file and every query app-wide
    /// would fail with SQLITE_NOTADB until restart. `refresh_conn` swaps in a
    /// freshly-opened connection after each sync.
    conn: std::sync::Mutex<Connection>,
}

impl Db {
    /// Build the shared connection and apply the busy timeout once.
    ///
    /// Wait (up to 5s) for a contended write lock instead of immediately failing
    /// with "database is locked". At startup the background sync catch-up pulls
    /// the cloud and runs migrations — writes to the same replica file the UI is
    /// concurrently reading. Without a busy timeout those reads intermittently
    /// error and leave a page blank; with it they simply wait out the brief
    /// writer and succeed.
    fn open_conn(db: &Database) -> Result<Connection, String> {
        let conn = db.connect().map_err(|e| e.to_string())?;
        conn.busy_timeout(std::time::Duration::from_millis(5000))
            .map_err(|e| e.to_string())?;
        Ok(conn)
    }

    pub async fn conn(&self) -> Result<Connection, String> {
        Ok(self.conn.lock().unwrap().clone())
    }

    /// Reopen the shared connection against the current `Database`. Called
    /// after every Turso sync — see the `conn` field docs. In-flight commands
    /// holding the old clone finish (or fail) on it; every later `conn()` call
    /// gets the fresh one.
    pub async fn refresh_conn(&self) -> Result<(), String> {
        let fresh = Self::open_conn(&*self.db.read().await)?;
        *self.conn.lock().unwrap() = fresh;
        Ok(())
    }

    /// A dedicated connection, isolated from the shared one. Use only for a
    /// command that runs a multi-statement `conn.transaction()` (BEGIN…COMMIT
    /// across awaits) — on the shared connection a concurrent reader could
    /// interleave a statement into the open transaction. Reads and single-call
    /// writes should use `conn()`.
    pub async fn fresh_conn(&self) -> Result<Connection, String> {
        Self::open_conn(&*self.db.read().await)
    }
    pub fn is_synced(&self) -> bool {
        self.mode == DbMode::Synced
    }

    /// Pull from / push to the remote (embedded replica). No-op result mapping
    /// only; callers decide what a failure means.
    pub async fn sync(&self) -> Result<(), String> {
        self.db
            .read()
            .await
            .sync()
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// Cheap smoke test on a freshly-opened connection: does the database file
    /// still look like a database to a new reader?
    async fn verify(&self) -> Result<(), String> {
        let conn = self.fresh_conn().await?;
        conn.query("SELECT count(*) FROM sqlite_master", ())
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// Whether an error message is the replica-corruption signature.
    pub fn is_corruption_error(e: &str) -> bool {
        let e = e.to_lowercase();
        e.contains("not a database") || e.contains("disk image is malformed")
    }

    /// Verify the database is readable; if it shows the corruption signature,
    /// rebuild the whole `Database` from scratch (close + reopen the replica —
    /// the in-process equivalent of the app restart that recovers it) and
    /// re-verify. Returns a human-readable outcome for the sync log.
    pub async fn ensure_healthy(&self) -> Result<HealthOutcome, String> {
        let Err(e) = self.verify().await else {
            return Ok(HealthOutcome::Healthy);
        };
        if !Self::is_corruption_error(&e) {
            // Unhealthy in some other way (e.g. busy) — not ours to fix here.
            return Err(format!("verify failed (not corruption): {e}"));
        }
        let Some(spec) = &self.replica else {
            return Err(format!("corruption detected but no replica spec to rebuild from: {e}"));
        };
        let fresh_db =
            Builder::new_remote_replica(spec.path.clone(), spec.url.clone(), spec.token.clone())
                .build()
                .await
                .map_err(|e| format!("rebuild failed: {e}"))?;
        {
            // Swap. The old Database drops here (in-flight connection clones
            // keep it alive until they finish; they fail and complete).
            let mut guard = self.db.write().await;
            *guard = fresh_db;
        }
        self.refresh_conn().await?;
        self.verify()
            .await
            .map(|_| HealthOutcome::Recovered { detected: e })
            .map_err(|e2| format!("still corrupt after rebuild: {e2}"))
    }
}

/// Outcome of `Db::ensure_healthy`, for the sync log.
pub enum HealthOutcome {
    Healthy,
    /// Corruption was detected (with the original error) and a full database
    /// reopen fixed it.
    Recovered { detected: String },
}

pub const LOCAL_DB: &str = "trackmyfi.db";
pub const REPLICA_DB: &str = "trackmyfi-replica.db";
pub const BACKUP_DB: &str = "trackmyfi.db.pre-sync-backup";

pub async fn init(app: &AppHandle) -> Result<Db, String> {
    let dir = resolve_app_dir(app.path().app_data_dir().map_err(|e| e.to_string())?);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let cfg = crate::sync::read_app_config(app);
    let token = match crate::sync::KeyringStore.get() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("warning: could not read sync token from keychain: {e}");
            None
        }
    };
    let has_creds = cfg.url.is_some() && token.is_some();
    let replica_path = dir.join(REPLICA_DB);
    let replica_exists = replica_path.exists();

    let source = decide_db_source(cfg.enabled, has_creds, cfg.bootstrapped, replica_exists);

    let mut replica_spec: Option<ReplicaSpec> = None;
    let (db, mode) = match source {
        DbSource::RemoteReplica => {
            let url = cfg.url.clone().unwrap();
            let token = token.unwrap();
            replica_spec = Some(ReplicaSpec {
                path: replica_path.clone(),
                url: url.clone(),
                token: token.clone(),
            });
            let db = Builder::new_remote_replica(replica_path, url, token)
                .build()
                .await
                .map_err(|e| e.to_string())?;
            // Deliberately NO startup `db.sync()` here. The local replica file
            // already holds the last-synced data, so reads are served instantly
            // and the window renders immediately. The initial cloud pull (so this
            // device sees other devices' edits) plus migrations run in the
            // background once the app is interactive — see `sync::initial_catch_up`,
            // kicked off from `lib.rs`'s setup hook.
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

    // The single connection every command shares (see `Db::conn`).
    let conn = Db::open_conn(&db)?;

    // Local modes have no network pull, so migrate inline now — it's fast and
    // purely local. Synced mode defers migrations to the background catch-up
    // (after the first pull) so startup never waits on the network, and so a
    // migration already applied on another device is pulled before this device
    // would try to re-apply it. See `sync::initial_catch_up`.
    if mode == DbMode::Local {
        crate::migrations::run(&conn).await?;
    }
    Ok(Db {
        db: tokio::sync::RwLock::new(db),
        mode,
        replica: replica_spec,
        conn: std::sync::Mutex::new(conn),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // decide_db_source(sync_enabled, has_creds, bootstrapped, replica_exists)

    #[test]
    fn synced_when_enabled_with_creds() {
        assert_eq!(decide_db_source(true, true, true, false), DbSource::RemoteReplica);
        assert_eq!(decide_db_source(true, true, true, true), DbSource::RemoteReplica);
    }

    #[test]
    fn falls_back_to_replica_file_when_disabled_but_synced_before() {
        // Deliberately disabled after a real sync (bootstrapped) => keep using the replica.
        assert_eq!(decide_db_source(false, true, true, true), DbSource::LocalReplicaFile);
        assert_eq!(decide_db_source(false, false, true, true), DbSource::LocalReplicaFile);
    }

    #[test]
    fn uses_original_when_never_synced() {
        assert_eq!(decide_db_source(false, false, false, false), DbSource::LocalOriginal);
        // Enabled flag set but creds missing => not synced yet.
        assert_eq!(decide_db_source(true, false, false, false), DbSource::LocalOriginal);
    }

    #[test]
    fn ignores_stale_replica_without_bootstrap() {
        // A replica file left by a failed/aborted setup (no bootstrapped config)
        // must NOT be opened — it would hide the real local DB.
        assert_eq!(decide_db_source(false, false, false, true), DbSource::LocalOriginal);
        assert_eq!(decide_db_source(false, true, false, true), DbSource::LocalOriginal);
    }

    #[test]
    fn debug_builds_isolate_into_dev_subdir() {
        let base = PathBuf::from("/tmp/app");
        let resolved = resolve_app_dir(base.clone());
        // Debug builds (tauri dev) must never share a dir with a release build.
        if cfg!(debug_assertions) {
            assert_eq!(resolved, base.join("dev"));
        } else {
            assert_eq!(resolved, base);
        }
    }
}
