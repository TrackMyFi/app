use crate::db::Db;
use crate::models::{AssetAttachment, MigrationSummary, StorageInfo};
use crate::storage::{
    build_object_store, build_store_from_spec, delete_credentials, local_attachments_dir,
    read_credentials, read_storage_config, write_credentials, write_storage_config, StorageConfig,
    StorageCredentials, StoreSpec,
};
use bytes::Bytes;
use object_store::ObjectStore;
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveStorageConfigArgs {
    pub provider: String,
    pub bucket_name: Option<String>,
    pub r2_account_id: Option<String>,
    pub s3_region: Option<String>,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub service_account_json: Option<String>,
}

#[tauri::command]
pub async fn get_storage_config_cmd(app: tauri::AppHandle) -> Result<StorageInfo, String> {
    let cfg = read_storage_config(&app);
    let local_path = local_attachments_dir(&app)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unavailable".into());
    let has_credentials = read_credentials()
        .map(|c| c.is_some())
        .unwrap_or(false);
    Ok(StorageInfo {
        provider: if cfg.provider.is_empty() { "local".into() } else { cfg.provider },
        bucket_name: cfg.bucket_name,
        r2_account_id: cfg.r2_account_id,
        s3_region: cfg.s3_region,
        local_path,
        has_credentials,
    })
}

#[tauri::command]
pub async fn save_storage_config_cmd(
    app: tauri::AppHandle,
    args: SaveStorageConfigArgs,
) -> Result<(), String> {
    write_storage_config(
        &app,
        &StorageConfig {
            provider: args.provider.clone(),
            bucket_name: args.bucket_name,
            r2_account_id: args.r2_account_id,
            s3_region: args.s3_region,
        },
    )?;

    // Only write credentials when the provider actually needs them.
    if args.provider != "local" {
        let creds = StorageCredentials {
            access_key_id: args.access_key_id,
            secret_access_key: args.secret_access_key,
            service_account_json: args.service_account_json,
        };
        write_credentials(&creds)?;
    }

    Ok(())
}

#[tauri::command]
pub async fn clear_storage_config_cmd(app: tauri::AppHandle) -> Result<(), String> {
    write_storage_config(&app, &StorageConfig::default())?;
    let _ = delete_credentials(); // best-effort; may not exist
    Ok(())
}

/// Returns the number of attachments not yet stored in `new_provider`.
/// Used by the UI to decide whether to offer a migration prompt.
#[tauri::command]
pub async fn count_migratable_attachments_cmd(
    db: State<'_, Db>,
    new_provider: String,
) -> Result<i64, String> {
    let conn = db.conn().await?;
    crate::commands::asset_events::count_attachments_not_provider(&conn, &new_provider).await
}

/// Migrates all attachments stored under the current configured provider to the new
/// provider specified in `new_args`, then saves the new config. Running migration
/// before saving ensures the old credentials are still available as the source.
#[tauri::command]
pub async fn migrate_and_save_storage_config_cmd(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    new_args: SaveStorageConfigArgs,
) -> Result<MigrationSummary, String> {
    let local_dir = local_attachments_dir(&app)?;

    // Source = currently saved config + credentials (still intact at this point).
    let src_store = build_object_store(&app)?;
    let src_cfg = read_storage_config(&app);
    let src_provider = if src_cfg.provider.is_empty() { "local".to_string() } else { src_cfg.provider };

    // Destination = new args (not yet saved to disk).
    let dst_provider = new_args.provider.clone();
    let dst_store = build_store_from_spec(&StoreSpec {
        provider: dst_provider.clone(),
        bucket_name: new_args.bucket_name.clone(),
        r2_account_id: new_args.r2_account_id.clone(),
        s3_region: new_args.s3_region.clone(),
        access_key_id: new_args.access_key_id.clone(),
        secret_access_key: new_args.secret_access_key.clone(),
        service_account_json: new_args.service_account_json.clone(),
        local_dir: local_dir.clone(),
    })?;

    // Fetch every attachment stored under the source provider.
    let conn = db.conn().await?;
    let attachments = crate::commands::asset_events::list_attachments_by_provider(
        &conn,
        &src_provider,
    )
    .await?;

    let mut migrated = 0i64;
    let mut failed_names: Vec<String> = Vec::new();

    for att in &attachments {
        let obj_path = object_store::path::Path::from(att.object_key.as_str());

        // Download from source.
        let get_result = match src_store.get(&obj_path).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("migration: get {} failed: {e}", att.object_key);
                failed_names.push(att.original_name.clone());
                continue;
            }
        };
        let data: bytes::Bytes = match get_result.bytes().await {
            Ok(b) => b,
            Err(e) => {
                eprintln!("migration: read {} failed: {e}", att.object_key);
                failed_names.push(att.original_name.clone());
                continue;
            }
        };

        // Upload to destination.
        if let Err(e) = dst_store.put(&obj_path, data.into()).await {
            eprintln!("migration: put {} failed: {e}", att.object_key);
            failed_names.push(att.original_name.clone());
            continue;
        }

        // Update the provider in the DB row.
        if let Err(e) = crate::commands::asset_events::update_attachment_provider(
            &conn,
            att.id,
            &dst_provider,
        )
        .await
        {
            eprintln!("migration: db update {} failed: {e}", att.id);
            failed_names.push(att.original_name.clone());
            continue;
        }

        migrated += 1;
    }

    // Now safe to save the new config (migration is done; old creds no longer needed).
    write_storage_config(
        &app,
        &StorageConfig {
            provider: new_args.provider.clone(),
            bucket_name: new_args.bucket_name,
            r2_account_id: new_args.r2_account_id,
            s3_region: new_args.s3_region,
        },
    )?;
    if new_args.provider != "local" {
        write_credentials(&StorageCredentials {
            access_key_id: new_args.access_key_id,
            secret_access_key: new_args.secret_access_key,
            service_account_json: new_args.service_account_json,
        })?;
    } else {
        let _ = delete_credentials();
    }

    let failed = failed_names.len() as i64;
    Ok(MigrationSummary { migrated, failed, failed_names })
}

#[tauri::command]
pub async fn list_attachments_cmd(
    db: State<'_, Db>,
    asset_event_id: i32,
) -> Result<Vec<AssetAttachment>, String> {
    let conn = db.conn().await?;
    crate::commands::asset_events::list_attachments(&conn, asset_event_id).await
}

#[tauri::command]
pub async fn upload_attachment_cmd(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    asset_event_id: i32,
    local_file_path: String,
) -> Result<AssetAttachment, String> {
    let path = std::path::Path::new(&local_file_path);
    let original_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid file path")?
        .to_string();

    let data = tokio::fs::read(path).await.map_err(|e| e.to_string())?;
    let byte_size = data.len() as i64;
    let bytes = Bytes::from(data);

    let cfg = read_storage_config(&app);
    let provider = if cfg.provider.is_empty() { "local".to_string() } else { cfg.provider };

    // Generate a stable, content-addressed-style key independent of event ID.
    let object_key = format!(
        "attachments/{}/{}",
        uuid::Uuid::new_v4(),
        original_name
    );

    let store = build_object_store(&app)?;
    let obj_path = object_store::path::Path::from(object_key.as_str());
    store
        .put(&obj_path, bytes.into())
        .await
        .map_err(|e| e.to_string())?;

    let created_at = chrono_now();
    let conn = db.conn().await?;
    crate::commands::asset_events::insert_attachment(
        &conn,
        asset_event_id,
        &object_key,
        &original_name,
        &provider,
        Some(byte_size),
        &created_at,
    )
    .await
}

#[tauri::command]
pub async fn delete_attachment_cmd(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    attachment_id: i32,
) -> Result<(), String> {
    let conn = db.conn().await?;
    // Fetch the attachment to get its object key before deleting the row.
    let mut rows = conn
        .query(
            "SELECT id, asset_event_id, object_key, original_name, provider, byte_size, created_at \
             FROM asset_attachment WHERE id = ?1",
            libsql::params![attachment_id],
        )
        .await
        .map_err(|e| e.to_string())?;

    let Some(row) = rows.next().await.map_err(|e| e.to_string())? else {
        return Err(format!("attachment {attachment_id} not found"));
    };
    let object_key: String = row.get(2).map_err(|e| e.to_string())?;
    let provider: String = row.get(4).map_err(|e| e.to_string())?;
    drop(rows);

    // Only attempt to delete from storage if the attachment was stored with the current provider.
    let current_provider = {
        let cfg = read_storage_config(&app);
        if cfg.provider.is_empty() { "local".to_string() } else { cfg.provider }
    };

    if provider == current_provider {
        if let Ok(store) = build_object_store(&app) {
            let path = object_store::path::Path::from(object_key.as_str());
            let _ = store.delete(&path).await; // best-effort
        }
    }

    crate::commands::asset_events::delete_attachment_row(&conn, attachment_id).await
}

#[tauri::command]
pub async fn open_attachment_cmd(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    attachment_id: i32,
) -> Result<(), String> {
    let conn = db.conn().await?;
    let mut rows = conn
        .query(
            "SELECT id, asset_event_id, object_key, original_name, provider, byte_size, created_at \
             FROM asset_attachment WHERE id = ?1",
            libsql::params![attachment_id],
        )
        .await
        .map_err(|e| e.to_string())?;

    let Some(row) = rows.next().await.map_err(|e| e.to_string())? else {
        return Err(format!("attachment {attachment_id} not found"));
    };
    let object_key: String = row.get(2).map_err(|e| e.to_string())?;
    let original_name: String = row.get(3).map_err(|e| e.to_string())?;
    let provider: String = row.get(4).map_err(|e| e.to_string())?;
    drop(rows);

    let current_provider = {
        let cfg = read_storage_config(&app);
        if cfg.provider.is_empty() { "local".to_string() } else { cfg.provider }
    };

    if provider != current_provider {
        return Err(format!(
            "This attachment was stored using '{provider}' but this device is configured for '{current_provider}'. \
             Configure the same storage provider on this device to open it."
        ));
    }

    let store = build_object_store(&app)?;
    let path = object_store::path::Path::from(object_key.as_str());
    let result = store.get(&path).await.map_err(|e| e.to_string())?;
    let data = result.bytes().await.map_err(|e| e.to_string())?;

    // Write to a temp file, then open with the OS default app.
    let tmp_dir = std::env::temp_dir().join("trackmyfi-attachments");
    std::fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let tmp_path = tmp_dir.join(&original_name);
    std::fs::write(&tmp_path, &data).map_err(|e| e.to_string())?;

    tauri_plugin_opener::open_path(tmp_path, Option::<&str>::None)
        .map_err(|e| e.to_string())
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    // Format as ISO 8601 without pulling in chrono.
    // We use the same pattern the frontend sends for created_at on other entities.
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Basic ISO string — good enough for storage; precise ms not required.
    let (y, mo, d, h, mi, s) = epoch_to_parts(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}.000Z")
}

fn epoch_to_parts(secs: u64) -> (u64, u64, u64, u64, u64, u64) {
    let s = secs % 60;
    let total_min = secs / 60;
    let mi = total_min % 60;
    let total_h = total_min / 60;
    let h = total_h % 24;
    let total_days = total_h / 24;
    // Gregorian calendar approximation.
    let (y, yd) = days_to_year(total_days);
    let (mo, d) = yd_to_month_day(y, yd);
    (y, mo, d, h, mi, s)
}

fn days_to_year(mut days: u64) -> (u64, u64) {
    let mut y = 1970u64;
    loop {
        let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
        let dy = if leap { 366 } else { 365 };
        if days < dy { break; }
        days -= dy;
        y += 1;
    }
    (y, days)
}

fn yd_to_month_day(year: u64, mut yd: u64) -> (u64, u64) {
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let months = [31u64, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut mo = 1u64;
    for dm in months {
        if yd < dm { return (mo, yd + 1); }
        yd -= dm;
        mo += 1;
    }
    (12, 31)
}
