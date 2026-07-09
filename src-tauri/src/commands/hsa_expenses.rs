use crate::db::Db;
use crate::models::{HsaAttachment, HsaExpense};
use libsql::{params, Connection};
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewHsaExpense {
    pub account_id: Option<i32>,
    pub date: String,
    pub description: String,
    pub category: String,
    pub amount: f64,
    pub person: Option<String>,
    pub provider: Option<String>,
    pub notes: Option<String>,
    pub reimbursed: bool,
    pub reimbursed_date: Option<String>,
    pub created_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateHsaExpense {
    pub id: i32,
    pub account_id: Option<i32>,
    pub date: String,
    pub description: String,
    pub category: String,
    pub amount: f64,
    pub person: Option<String>,
    pub provider: Option<String>,
    pub notes: Option<String>,
    pub reimbursed: bool,
    pub reimbursed_date: Option<String>,
    pub updated_at: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HsaExpenseFilter {
    pub account_id: Option<i32>,
    pub category: Option<String>,
    pub reimbursed: Option<bool>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub search: Option<String>,
}

// col 13 is a boolean from a subquery — see the SELECT wrapper in list/get below.
const COLS: &str = "id, account_id, date, description, category, amount, person, \
    provider, notes, reimbursed, reimbursed_date, created_at, updated_at";

fn row_to_hsa_expense(row: &libsql::Row) -> Result<HsaExpense, String> {
    Ok(HsaExpense {
        id: row.get(0).map_err(|e| e.to_string())?,
        account_id: row.get(1).map_err(|e| e.to_string())?,
        date: row.get(2).map_err(|e| e.to_string())?,
        description: row.get(3).map_err(|e| e.to_string())?,
        category: row.get(4).map_err(|e| e.to_string())?,
        amount: row.get(5).map_err(|e| e.to_string())?,
        person: row.get(6).map_err(|e| e.to_string())?,
        provider: row.get(7).map_err(|e| e.to_string())?,
        notes: row.get(8).map_err(|e| e.to_string())?,
        reimbursed: row.get::<i64>(9).map_err(|e| e.to_string())? != 0,
        reimbursed_date: row.get(10).map_err(|e| e.to_string())?,
        created_at: row.get(11).map_err(|e| e.to_string())?,
        updated_at: row.get(12).map_err(|e| e.to_string())?,
        // col 13: EXISTS subquery returns 0/1 as integer; map to bool
        has_attachment: row.get::<i64>(13).unwrap_or(0) != 0,
    })
}

fn row_to_attachment(row: &libsql::Row) -> Result<HsaAttachment, String> {
    Ok(HsaAttachment {
        id: row.get(0).map_err(|e| e.to_string())?,
        hsa_expense_id: row.get(1).map_err(|e| e.to_string())?,
        object_key: row.get(2).map_err(|e| e.to_string())?,
        original_name: row.get(3).map_err(|e| e.to_string())?,
        provider: row.get(4).map_err(|e| e.to_string())?,
        byte_size: row.get(5).map_err(|e| e.to_string())?,
        created_at: row.get(6).map_err(|e| e.to_string())?,
    })
}

const ATTACHMENT_COLS: &str =
    "id, hsa_expense_id, object_key, original_name, provider, byte_size, created_at";

pub async fn list_attachments(conn: &Connection, hsa_expense_id: i32) -> Result<Vec<HsaAttachment>, String> {
    let sql = format!("SELECT {ATTACHMENT_COLS} FROM hsa_attachment WHERE hsa_expense_id = ?1 ORDER BY id");
    let mut rows = conn
        .query(&sql, params![hsa_expense_id])
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(row_to_attachment(&row)?);
    }
    Ok(out)
}

pub async fn get_attachment(conn: &Connection, id: i32) -> Result<HsaAttachment, String> {
    let sql = format!("SELECT {ATTACHMENT_COLS} FROM hsa_attachment WHERE id = ?1");
    let mut rows = conn.query(&sql, params![id]).await.map_err(|e| e.to_string())?;
    match rows.next().await.map_err(|e| e.to_string())? {
        Some(row) => row_to_attachment(&row),
        None => Err(format!("HSA attachment {id} not found")),
    }
}

pub async fn insert_attachment(
    conn: &Connection,
    hsa_expense_id: i32,
    object_key: &str,
    original_name: &str,
    provider: &str,
    byte_size: Option<i64>,
    created_at: &str,
) -> Result<HsaAttachment, String> {
    conn.execute(
        "INSERT INTO hsa_attachment (hsa_expense_id, object_key, original_name, provider, byte_size, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![hsa_expense_id, object_key, original_name, provider, byte_size, created_at],
    )
    .await
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid() as i32;
    get_attachment(conn, id).await
}

pub async fn delete_attachment_row(conn: &Connection, id: i32) -> Result<(), String> {
    conn.execute("DELETE FROM hsa_attachment WHERE id = ?1", params![id])
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn update_attachment_provider(
    conn: &Connection,
    id: i32,
    provider: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE hsa_attachment SET provider = ?1 WHERE id = ?2",
        params![provider, id],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn list_attachments_by_provider(
    conn: &Connection,
    provider: &str,
) -> Result<Vec<HsaAttachment>, String> {
    let sql = format!("SELECT {ATTACHMENT_COLS} FROM hsa_attachment WHERE provider = ?1 ORDER BY id");
    let mut rows = conn
        .query(&sql, params![provider])
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(row_to_attachment(&row)?);
    }
    Ok(out)
}

pub async fn count_attachments_not_provider(
    conn: &Connection,
    provider: &str,
) -> Result<i64, String> {
    let mut rows = conn
        .query(
            "SELECT COUNT(*) FROM hsa_attachment WHERE provider != ?1",
            params![provider],
        )
        .await
        .map_err(|e| e.to_string())?;
    match rows.next().await.map_err(|e| e.to_string())? {
        Some(row) => row.get::<i64>(0).map_err(|e| e.to_string()),
        None => Ok(0),
    }
}

pub async fn get_hsa_expense(conn: &Connection, id: i32) -> Result<HsaExpense, String> {
    let sql = format!(
        "SELECT {COLS}, EXISTS(SELECT 1 FROM hsa_attachment WHERE hsa_expense_id = hsa_expense.id) \
         FROM hsa_expense WHERE id = ?1"
    );
    let mut rows = conn.query(&sql, params![id]).await.map_err(|e| e.to_string())?;
    match rows.next().await.map_err(|e| e.to_string())? {
        Some(row) => row_to_hsa_expense(&row),
        None => Err(format!("HSA expense {id} not found")),
    }
}

pub async fn list_hsa_expenses(
    conn: &Connection,
    f: &HsaExpenseFilter,
) -> Result<Vec<HsaExpense>, String> {
    let mut where_clauses: Vec<String> = Vec::new();
    let mut bind_params: Vec<libsql::Value> = Vec::new();

    if let Some(a) = f.account_id {
        where_clauses.push("account_id = ?".into());
        bind_params.push(libsql::Value::Integer(a as i64));
    }
    if let Some(c) = &f.category {
        where_clauses.push("category = ?".into());
        bind_params.push(libsql::Value::Text(c.clone()));
    }
    if let Some(r) = f.reimbursed {
        where_clauses.push("reimbursed = ?".into());
        bind_params.push(libsql::Value::Integer(if r { 1 } else { 0 }));
    }
    if let Some(s) = &f.start_date {
        where_clauses.push("date >= ?".into());
        bind_params.push(libsql::Value::Text(s.clone()));
    }
    if let Some(e) = &f.end_date {
        where_clauses.push("date <= ?".into());
        bind_params.push(libsql::Value::Text(e.clone()));
    }
    if let Some(q) = &f.search {
        where_clauses.push("(description LIKE ? OR person LIKE ? OR provider LIKE ? OR notes LIKE ?)".into());
        let like = format!("%{}%", q);
        for _ in 0..4 {
            bind_params.push(libsql::Value::Text(like.clone()));
        }
    }

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    let sql = format!(
        "SELECT {COLS}, EXISTS(SELECT 1 FROM hsa_attachment WHERE hsa_expense_id = hsa_expense.id) \
         FROM hsa_expense {where_sql} ORDER BY date DESC, id DESC"
    );
    let mut rows = conn
        .query(&sql, libsql::params_from_iter(bind_params))
        .await
        .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(row_to_hsa_expense(&row)?);
    }
    Ok(out)
}

pub async fn create_hsa_expense(conn: &Connection, e: &NewHsaExpense) -> Result<HsaExpense, String> {
    conn.execute(
        "INSERT INTO hsa_expense (account_id, date, description, category, amount, person, \
         provider, notes, reimbursed, reimbursed_date, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
        params![
            e.account_id, e.date.clone(), e.description.clone(), e.category.clone(),
            e.amount, e.person.clone(), e.provider.clone(), e.notes.clone(),
            e.reimbursed as i64, e.reimbursed_date.clone(), e.created_at.clone()
        ],
    )
    .await
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid() as i32;
    get_hsa_expense(conn, id).await
}

pub async fn update_hsa_expense(
    conn: &Connection,
    e: &UpdateHsaExpense,
) -> Result<HsaExpense, String> {
    // Verify the expense exists before modifying anything
    get_hsa_expense(conn, e.id).await?;

    conn.execute(
        "UPDATE hsa_expense SET account_id=?1, date=?2, description=?3, category=?4, \
         amount=?5, person=?6, provider=?7, notes=?8, reimbursed=?9, reimbursed_date=?10, \
         updated_at=?11 WHERE id=?12",
        params![
            e.account_id, e.date.clone(), e.description.clone(), e.category.clone(),
            e.amount, e.person.clone(), e.provider.clone(), e.notes.clone(),
            e.reimbursed as i64, e.reimbursed_date.clone(), e.updated_at.clone(), e.id
        ],
    )
    .await
    .map_err(|e| e.to_string())?;

    get_hsa_expense(conn, e.id).await
}

pub async fn delete_hsa_expense(conn: &Connection, id: i32) -> Result<(), String> {
    conn.execute("DELETE FROM hsa_expense WHERE id = ?1", params![id])
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ---- thin command wrappers ----

#[tauri::command]
pub async fn list_hsa_expenses_cmd(
    db: State<'_, Db>,
    filter: HsaExpenseFilter,
) -> Result<Vec<HsaExpense>, String> {
    let conn = db.conn().await?;
    list_hsa_expenses(&conn, &filter).await
}

#[tauri::command]
pub async fn get_hsa_expense_cmd(db: State<'_, Db>, id: i32) -> Result<HsaExpense, String> {
    let conn = db.conn().await?;
    get_hsa_expense(&conn, id).await
}

#[tauri::command]
pub async fn create_hsa_expense_cmd(
    db: State<'_, Db>,
    expense: NewHsaExpense,
) -> Result<HsaExpense, String> {
    let conn = db.conn().await?;
    create_hsa_expense(&conn, &expense).await
}

#[tauri::command]
pub async fn update_hsa_expense_cmd(
    db: State<'_, Db>,
    expense: UpdateHsaExpense,
) -> Result<HsaExpense, String> {
    let conn = db.conn().await?;
    update_hsa_expense(&conn, &expense).await
}

#[tauri::command]
pub async fn delete_hsa_expense_cmd(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    id: i32,
) -> Result<(), String> {
    let conn = db.conn().await?;
    // Fetch attachments before deletion so we can remove objects from storage.
    let attachments = list_attachments(&conn, id).await.unwrap_or_default();
    if !attachments.is_empty() {
        if let Ok(store) = crate::storage::build_object_store(&app, &conn).await {
            for att in &attachments {
                let path = object_store::path::Path::from(att.object_key.as_str());
                let _ = store.delete(&path).await; // best-effort; DB cascade still cleans the row
            }
        }
    }
    delete_hsa_expense(&conn, id).await
}
