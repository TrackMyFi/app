use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

use object_store::ObjectStore;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StorageConfig {
    pub provider: String, // "local" | "r2" | "gcs" | "s3"
    pub bucket_name: Option<String>,
    pub r2_account_id: Option<String>,
    pub s3_region: Option<String>,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            provider: "local".into(),
            bucket_name: None,
            r2_account_id: None,
            s3_region: None,
        }
    }
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

/// Reads non-secret storage config from the DB (synced via Turso).
pub async fn read_storage_config_db(conn: &libsql::Connection) -> StorageConfig {
    let mut rows = match conn
        .query(
            "SELECT provider, bucket_name, r2_account_id, s3_region \
             FROM storage_config WHERE id = 1",
            (),
        )
        .await
    {
        Ok(r) => r,
        Err(_) => return StorageConfig::default(),
    };
    match rows.next().await {
        Ok(Some(row)) => {
            let provider = row
                .get::<String>(0)
                .unwrap_or_else(|_| "local".into());
            StorageConfig {
                provider: if provider.is_empty() { "local".into() } else { provider },
                bucket_name: row.get::<Option<String>>(1).unwrap_or(None),
                r2_account_id: row.get::<Option<String>>(2).unwrap_or(None),
                s3_region: row.get::<Option<String>>(3).unwrap_or(None),
            }
        }
        _ => StorageConfig::default(),
    }
}

/// Writes non-secret storage config to the DB (syncs via Turso automatically).
pub async fn write_storage_config_db(
    conn: &libsql::Connection,
    cfg: &StorageConfig,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO storage_config (id, provider, bucket_name, r2_account_id, s3_region)
         VALUES (1, ?1, ?2, ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET
           provider      = excluded.provider,
           bucket_name   = excluded.bucket_name,
           r2_account_id = excluded.r2_account_id,
           s3_region     = excluded.s3_region",
        libsql::params![
            cfg.provider.clone(),
            cfg.bucket_name.clone(),
            cfg.r2_account_id.clone(),
            cfg.s3_region.clone(),
        ],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
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

pub async fn build_object_store(
    app: &AppHandle,
    conn: &libsql::Connection,
) -> Result<Arc<dyn ObjectStore>, String> {
    let cfg = read_storage_config_db(conn).await;
    // Only touch the keychain when the provider actually needs credentials —
    // a local-storage setup should never trigger keychain access (or prompts).
    let creds = if cfg.provider == "local" {
        StorageCredentials::default()
    } else {
        read_credentials()?.unwrap_or_default()
    };
    let local_dir = local_attachments_dir(app)?;
    build_store_from_spec(&StoreSpec {
        provider: cfg.provider,
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
