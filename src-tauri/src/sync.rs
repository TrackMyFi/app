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
