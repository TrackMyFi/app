use crate::sync::TokenStore;
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
