use crate::db::Db;
use crate::models::CategoryRule;
use libsql::{params, Connection};
use tauri::State;

fn row_to_rule(row: &libsql::Row) -> Result<CategoryRule, String> {
    Ok(CategoryRule {
        id: row.get(0).map_err(|e| e.to_string())?,
        keyword: row.get(1).map_err(|e| e.to_string())?,
        category: row.get(2).map_err(|e| e.to_string())?,
        created_at: row.get(3).map_err(|e| e.to_string())?,
    })
}

pub async fn list_category_rules(conn: &Connection) -> Result<Vec<CategoryRule>, String> {
    let mut rows = conn
        .query(
            "SELECT id, keyword, category, created_at FROM category_rules ORDER BY keyword",
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


pub async fn delete_category_rule(conn: &Connection, id: i32) -> Result<(), String> {
    conn.execute("DELETE FROM category_rules WHERE id = ?1", params![id])
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn update_category_rule(
    conn: &Connection,
    id: i32,
    keyword: String,
    category: String,
) -> Result<(), String> {
    conn.execute(
        "UPDATE category_rules SET keyword = ?1, category = ?2 WHERE id = ?3",
        params![keyword, category, id],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn list_category_rules_cmd(db: State<'_, Db>) -> Result<Vec<CategoryRule>, String> {
    let conn = db.conn().await?;
    list_category_rules(&conn).await
}

#[tauri::command]
pub async fn create_category_rule_cmd(
    db: State<'_, Db>,
    keyword: String,
    category: String,
    created_at: String,
) -> Result<CategoryRule, String> {
    let conn = db.conn().await?;
    conn.execute(
        "INSERT INTO category_rules (keyword, category, created_at) VALUES (?1, ?2, ?3)",
        params![keyword.clone(), category.clone(), created_at.clone()],
    )
    .await
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid() as i32;
    Ok(CategoryRule { id, keyword, category, created_at })
}

#[tauri::command]
pub async fn delete_category_rule_cmd(db: State<'_, Db>, id: i32) -> Result<(), String> {
    let conn = db.conn().await?;
    delete_category_rule(&conn, id).await
}

#[tauri::command]
pub async fn update_category_rule_cmd(
    db: State<'_, Db>,
    id: i32,
    keyword: String,
    category: String,
) -> Result<(), String> {
    let conn = db.conn().await?;
    update_category_rule(&conn, id, keyword, category).await
}
