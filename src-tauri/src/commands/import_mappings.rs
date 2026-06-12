use crate::db::Db;
use crate::models::ImportMapping;
use libsql::{params, Connection};
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewImportMapping {
    pub name: String,
    pub config: String,
    pub created_at: String,
}

fn row_to_mapping(row: &libsql::Row) -> Result<ImportMapping, String> {
    Ok(ImportMapping {
        id: row.get(0).map_err(|e| e.to_string())?,
        name: row.get(1).map_err(|e| e.to_string())?,
        config: row.get(2).map_err(|e| e.to_string())?,
        created_at: row.get(3).map_err(|e| e.to_string())?,
    })
}

pub async fn list_import_mappings(conn: &Connection) -> Result<Vec<ImportMapping>, String> {
    let mut rows = conn
        .query(
            "SELECT id, name, config, created_at FROM import_mapping ORDER BY name",
            (),
        )
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(row_to_mapping(&row)?);
    }
    Ok(out)
}

pub async fn create_import_mapping(
    conn: &Connection,
    m: &NewImportMapping,
) -> Result<i32, String> {
    conn.execute(
        "INSERT INTO import_mapping (name, config, created_at) VALUES (?1, ?2, ?3)",
        params![m.name.clone(), m.config.clone(), m.created_at.clone()],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid() as i32)
}

pub async fn delete_import_mapping(conn: &Connection, id: i32) -> Result<(), String> {
    conn.execute("DELETE FROM import_mapping WHERE id = ?1", params![id])
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn list_import_mappings_cmd(db: State<'_, Db>) -> Result<Vec<ImportMapping>, String> {
    let conn = db.conn().await?;
    list_import_mappings(&conn).await
}

#[tauri::command]
pub async fn create_import_mapping_cmd(
    db: State<'_, Db>,
    mapping: NewImportMapping,
) -> Result<i32, String> {
    let conn = db.conn().await?;
    create_import_mapping(&conn, &mapping).await
}

#[tauri::command]
pub async fn delete_import_mapping_cmd(db: State<'_, Db>, id: i32) -> Result<(), String> {
    let conn = db.conn().await?;
    delete_import_mapping(&conn, id).await
}
