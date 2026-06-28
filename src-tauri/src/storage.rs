use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

use object_store::ObjectStore;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct StorageConfig {
    pub provider: String, // "local" | "r2" | "gcs" | "s3"
    pub bucket_name: Option<String>,
    pub r2_account_id: Option<String>,
    pub s3_region: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StorageCredentials {
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub service_account_json: Option<String>,
}

// Mirrors sync.rs: debug builds get a separate keychain entry.
#[cfg(debug_assertions)]
const KEYCHAIN_SERVICE: &str = "com.trackmyfi.desktop.dev";
#[cfg(not(debug_assertions))]
const KEYCHAIN_SERVICE: &str = "com.trackmyfi.desktop";
const KEYCHAIN_STORAGE_USER: &str = "storage-credentials";

pub fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::db::resolve_app_dir(app.path().app_config_dir().map_err(|e| e.to_string())?);
    Ok(dir.join("storage.json"))
}

pub fn read_storage_config(app: &AppHandle) -> StorageConfig {
    let Ok(path) = config_path(app) else {
        return StorageConfig::default();
    };
    match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => StorageConfig::default(),
    }
}

pub fn write_storage_config(app: &AppHandle, cfg: &StorageConfig) -> Result<(), String> {
    let path = config_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

pub fn read_credentials() -> Result<Option<StorageCredentials>, String> {
    match read_creds_str()? {
        Some(s) => serde_json::from_str(&s).map(Some).map_err(|e| e.to_string()),
        None => Ok(None),
    }
}

pub fn write_credentials(creds: &StorageCredentials) -> Result<(), String> {
    let json = serde_json::to_string(creds).map_err(|e| e.to_string())?;
    write_creds_str(&json)
}

pub fn delete_credentials() -> Result<(), String> {
    delete_creds_str()
}

pub fn local_attachments_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::db::resolve_app_dir(
        app.path().app_data_dir().map_err(|e| e.to_string())?,
    )
    .join("attachments");
    Ok(dir)
}

/// Inline spec for building a store without reading from disk. Used during migration
/// so we can construct the destination store from form input before saving new config.
pub struct StoreSpec {
    pub provider: String,
    pub bucket_name: Option<String>,
    pub r2_account_id: Option<String>,
    pub s3_region: Option<String>,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub service_account_json: Option<String>,
    /// Directory used when provider is "local".
    pub local_dir: PathBuf,
}

pub fn build_store_from_spec(spec: &StoreSpec) -> Result<Arc<dyn ObjectStore>, String> {
    match spec.provider.as_str() {
        "r2" => {
            let bucket = spec.bucket_name.as_deref().ok_or("R2 bucket name is required")?;
            let account_id = spec.r2_account_id.as_deref().ok_or("R2 account ID is required")?;
            let access_key = spec.access_key_id.as_deref().ok_or("R2 access key ID is required")?;
            let secret_key = spec.secret_access_key.as_deref().ok_or("R2 secret access key is required")?;
            let endpoint = format!("https://{account_id}.r2.cloudflarestorage.com");
            let store = object_store::aws::AmazonS3Builder::new()
                .with_bucket_name(bucket)
                .with_region("auto")
                .with_access_key_id(access_key)
                .with_secret_access_key(secret_key)
                .with_endpoint(endpoint)
                .build()
                .map_err(|e| e.to_string())?;
            Ok(Arc::new(store))
        }
        "gcs" => {
            let bucket = spec.bucket_name.as_deref().ok_or("GCS bucket name is required")?;
            let sa_json = spec.service_account_json.as_deref().ok_or("GCS service account JSON is required")?;
            let store = object_store::gcp::GoogleCloudStorageBuilder::new()
                .with_bucket_name(bucket)
                .with_service_account_key(sa_json)
                .build()
                .map_err(|e| e.to_string())?;
            Ok(Arc::new(store))
        }
        "s3" => {
            let bucket = spec.bucket_name.as_deref().ok_or("S3 bucket name is required")?;
            let region = spec.s3_region.as_deref().ok_or("S3 region is required")?;
            let access_key = spec.access_key_id.as_deref().ok_or("S3 access key ID is required")?;
            let secret_key = spec.secret_access_key.as_deref().ok_or("S3 secret access key is required")?;
            let store = object_store::aws::AmazonS3Builder::new()
                .with_bucket_name(bucket)
                .with_region(region)
                .with_access_key_id(access_key)
                .with_secret_access_key(secret_key)
                .build()
                .map_err(|e| e.to_string())?;
            Ok(Arc::new(store))
        }
        _ => {
            std::fs::create_dir_all(&spec.local_dir).map_err(|e| e.to_string())?;
            let store = object_store::local::LocalFileSystem::new_with_prefix(&spec.local_dir)
                .map_err(|e| e.to_string())?;
            Ok(Arc::new(store))
        }
    }
}

pub fn build_object_store(app: &AppHandle) -> Result<Arc<dyn ObjectStore>, String> {
    let cfg = read_storage_config(app);
    let creds = read_credentials()?.unwrap_or_default();
    let local_dir = local_attachments_dir(app)?;
    build_store_from_spec(&StoreSpec {
        provider: if cfg.provider.is_empty() { "local".into() } else { cfg.provider },
        bucket_name: cfg.bucket_name,
        r2_account_id: cfg.r2_account_id,
        s3_region: cfg.s3_region,
        access_key_id: creds.access_key_id,
        secret_access_key: creds.secret_access_key,
        service_account_json: creds.service_account_json,
        local_dir,
    })
}

// ---- platform keychain helpers ----

#[cfg(target_os = "macos")]
fn read_creds_str() -> Result<Option<String>, String> {
    crate::sync::macos_keychain::get(KEYCHAIN_SERVICE, KEYCHAIN_STORAGE_USER)
}

#[cfg(target_os = "macos")]
fn write_creds_str(json: &str) -> Result<(), String> {
    crate::sync::macos_keychain::set(KEYCHAIN_SERVICE, KEYCHAIN_STORAGE_USER, json)
}

#[cfg(target_os = "macos")]
fn delete_creds_str() -> Result<(), String> {
    crate::sync::macos_keychain::delete(KEYCHAIN_SERVICE, KEYCHAIN_STORAGE_USER)
}

#[cfg(not(target_os = "macos"))]
fn read_creds_str() -> Result<Option<String>, String> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_STORAGE_USER)
        .map_err(|e| e.to_string())?;
    match entry.get_password() {
        Ok(p) => Ok(Some(p)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(not(target_os = "macos"))]
fn write_creds_str(json: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_STORAGE_USER)
        .map_err(|e| e.to_string())?;
    entry.set_password(json).map_err(|e| e.to_string())
}

#[cfg(not(target_os = "macos"))]
fn delete_creds_str() -> Result<(), String> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_STORAGE_USER)
        .map_err(|e| e.to_string())?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
