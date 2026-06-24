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

pub struct Db {
    pub db: Database,
    pub mode: DbMode,
}

impl Db {
    pub async fn conn(&self) -> Result<Connection, String> {
        let conn = self.db.connect().map_err(|e| e.to_string())?;
        // Wait (up to 5s) for a contended write lock instead of immediately
        // failing with "database is locked". At startup the background sync
        // catch-up pulls the cloud and runs migrations — writes to the same
        // replica file the UI is concurrently reading. Without a busy timeout
        // those reads intermittently error and leave a page blank; with it they
        // simply wait out the brief writer and succeed.
        conn.busy_timeout(std::time::Duration::from_millis(5000))
            .map_err(|e| e.to_string())?;
        Ok(conn)
    }
    pub fn is_synced(&self) -> bool {
        self.mode == DbMode::Synced
    }
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

    let (db, mode) = match source {
        DbSource::RemoteReplica => {
            let url = cfg.url.clone().unwrap();
            let token = token.unwrap();
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

    // Local modes have no network pull, so migrate inline now — it's fast and
    // purely local. Synced mode defers migrations to the background catch-up
    // (after the first pull) so startup never waits on the network, and so a
    // migration already applied on another device is pulled before this device
    // would try to re-apply it. See `sync::initial_catch_up`.
    if mode == DbMode::Local {
        let conn = db.connect().map_err(|e| e.to_string())?;
        crate::migrations::run(&conn).await?;
    }
    Ok(Db { db, mode })
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
