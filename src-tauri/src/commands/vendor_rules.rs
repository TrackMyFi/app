use crate::db::Db;
use crate::models::VendorRule;
use libsql::{params, Connection};
use tauri::State;

fn row_to_rule(row: &libsql::Row) -> Result<VendorRule, String> {
    Ok(VendorRule {
        id: row.get(0).map_err(|e| e.to_string())?,
        keyword: row.get(1).map_err(|e| e.to_string())?,
        vendor_name: row.get(2).map_err(|e| e.to_string())?,
        created_at: row.get(3).map_err(|e| e.to_string())?,
    })
}

pub async fn list_vendor_rules(conn: &Connection) -> Result<Vec<VendorRule>, String> {
    let mut rows = conn
        .query(
            "SELECT id, keyword, vendor_name, created_at FROM vendor_rules ORDER BY keyword",
            (),
        )
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(row_to_rule(&row)?);
    }
    Ok(out)
}

pub async fn delete_vendor_rule(conn: &Connection, id: i32) -> Result<(), String> {
    conn.execute("DELETE FROM vendor_rules WHERE id = ?1", params![id])
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn update_vendor_rule(
    conn: &Connection,
    id: i32,
    keyword: String,
    vendor_name: String,
) -> Result<(), String> {
    conn.execute(
        "UPDATE vendor_rules SET keyword = ?1, vendor_name = ?2 WHERE id = ?3",
        params![keyword, vendor_name, id],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn list_vendor_rules_cmd(db: State<'_, Db>) -> Result<Vec<VendorRule>, String> {
    let conn = db.conn().await?;
    list_vendor_rules(&conn).await
}

#[tauri::command]
pub async fn create_vendor_rule_cmd(
    db: State<'_, Db>,
    keyword: String,
    vendor_name: String,
    created_at: String,
) -> Result<VendorRule, String> {
    let conn = db.conn().await?;
    conn.execute(
        "INSERT INTO vendor_rules (keyword, vendor_name, created_at) VALUES (?1, ?2, ?3)",
        params![keyword.clone(), vendor_name.clone(), created_at.clone()],
    )
    .await
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid() as i32;
    Ok(VendorRule { id, keyword, vendor_name, created_at })
}

#[tauri::command]
pub async fn delete_vendor_rule_cmd(db: State<'_, Db>, id: i32) -> Result<(), String> {
    let conn = db.conn().await?;
    delete_vendor_rule(&conn, id).await
}

#[tauri::command]
pub async fn update_vendor_rule_cmd(
    db: State<'_, Db>,
    id: i32,
    keyword: String,
    vendor_name: String,
) -> Result<(), String> {
    let conn = db.conn().await?;
    update_vendor_rule(&conn, id, keyword, vendor_name).await
}
