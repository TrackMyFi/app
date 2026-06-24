use crate::db::Db;
use crate::models::AssetEvent;
use libsql::{params, Connection};
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewAssetEvent {
    pub account_id: Option<i32>,
    pub asset_label: Option<String>,
    pub date: String,
    pub description: String,
    pub kind: String,
    pub cost: f64,
    pub asset_value: Option<f64>,
    pub vendor: Option<String>,
    pub notes: Option<String>,
    pub linked_transaction_id: Option<i32>,
    pub created_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAssetEvent {
    pub id: i32,
    pub account_id: Option<i32>,
    pub asset_label: Option<String>,
    pub date: String,
    pub description: String,
    pub kind: String,
    pub cost: f64,
    pub asset_value: Option<f64>,
    pub vendor: Option<String>,
    pub notes: Option<String>,
    pub linked_transaction_id: Option<i32>,
    pub updated_at: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AssetEventFilter {
    pub account_id: Option<i32>,
    pub kind: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub search: Option<String>,
}

const COLS: &str = "id, account_id, asset_label, date, description, kind, cost, asset_value, \
    vendor, notes, linked_transaction_id, created_at, updated_at";

fn row_to_asset_event(row: &libsql::Row) -> Result<AssetEvent, String> {
    Ok(AssetEvent {
        id: row.get(0).map_err(|e| e.to_string())?,
        account_id: row.get(1).map_err(|e| e.to_string())?,
        asset_label: row.get(2).map_err(|e| e.to_string())?,
        date: row.get(3).map_err(|e| e.to_string())?,
        description: row.get(4).map_err(|e| e.to_string())?,
        kind: row.get(5).map_err(|e| e.to_string())?,
        cost: row.get(6).map_err(|e| e.to_string())?,
        asset_value: row.get(7).map_err(|e| e.to_string())?,
        vendor: row.get(8).map_err(|e| e.to_string())?,
        notes: row.get(9).map_err(|e| e.to_string())?,
        linked_transaction_id: row.get(10).map_err(|e| e.to_string())?,
        created_at: row.get(11).map_err(|e| e.to_string())?,
        updated_at: row.get(12).map_err(|e| e.to_string())?,
    })
}

// At least one of account_id / non-empty asset_label must be present.
fn validate_asset_ref(account_id: Option<i32>, asset_label: &Option<String>) -> Result<(), String> {
    let has_label = asset_label.as_deref().is_some_and(|s| !s.trim().is_empty());
    if account_id.is_none() && !has_label {
        return Err("An asset event must be linked to an account or have an asset label".into());
    }
    Ok(())
}

pub async fn get_asset_event(conn: &Connection, id: i32) -> Result<AssetEvent, String> {
    let sql = format!("SELECT {COLS} FROM asset_event WHERE id = ?1");
    let mut rows = conn.query(&sql, params![id]).await.map_err(|e| e.to_string())?;
    match rows.next().await.map_err(|e| e.to_string())? {
        Some(row) => row_to_asset_event(&row),
        None => Err(format!("asset event {id} not found")),
    }
}

pub async fn list_asset_events(
    conn: &Connection,
    f: &AssetEventFilter,
) -> Result<Vec<AssetEvent>, String> {
    let mut where_clauses: Vec<String> = Vec::new();
    let mut bind_params: Vec<libsql::Value> = Vec::new();

    if let Some(a) = f.account_id {
        where_clauses.push("account_id = ?".into());
        bind_params.push(libsql::Value::Integer(a as i64));
    }
    if let Some(k) = &f.kind {
        where_clauses.push("kind = ?".into());
        bind_params.push(libsql::Value::Text(k.clone()));
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
        where_clauses.push("(description LIKE ? OR asset_label LIKE ? OR vendor LIKE ?)".into());
        let like = format!("%{}%", q);
        bind_params.push(libsql::Value::Text(like.clone()));
        bind_params.push(libsql::Value::Text(like.clone()));
        bind_params.push(libsql::Value::Text(like));
    }

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    let sql = format!("SELECT {COLS} FROM asset_event {where_sql} ORDER BY date DESC, id DESC");
    let mut rows = conn
        .query(&sql, libsql::params_from_iter(bind_params))
        .await
        .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(row_to_asset_event(&row)?);
    }
    Ok(out)
}

pub async fn create_asset_event(conn: &Connection, e: &NewAssetEvent) -> Result<AssetEvent, String> {
    validate_asset_ref(e.account_id, &e.asset_label)?;
    conn.execute(
        "INSERT INTO asset_event (account_id, asset_label, date, description, kind, cost, \
         asset_value, vendor, notes, linked_transaction_id, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
        params![
            e.account_id, e.asset_label.clone(), e.date.clone(), e.description.clone(),
            e.kind.clone(), e.cost, e.asset_value, e.vendor.clone(), e.notes.clone(),
            e.linked_transaction_id, e.created_at.clone()
        ],
    )
    .await
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid() as i32;
    get_asset_event(conn, id).await
}

pub async fn update_asset_event(
    conn: &Connection,
    e: &UpdateAssetEvent,
) -> Result<AssetEvent, String> {
    validate_asset_ref(e.account_id, &e.asset_label)?;
    // Verify the event exists before modifying anything
    get_asset_event(conn, e.id).await?;

    conn.execute(
        "UPDATE asset_event SET account_id=?1, asset_label=?2, date=?3, description=?4, \
         kind=?5, cost=?6, asset_value=?7, vendor=?8, notes=?9, linked_transaction_id=?10, \
         updated_at=?11 WHERE id=?12",
        params![
            e.account_id, e.asset_label.clone(), e.date.clone(), e.description.clone(),
            e.kind.clone(), e.cost, e.asset_value, e.vendor.clone(), e.notes.clone(),
            e.linked_transaction_id, e.updated_at.clone(), e.id
        ],
    )
    .await
    .map_err(|e| e.to_string())?;

    get_asset_event(conn, e.id).await
}

pub async fn delete_asset_event(conn: &Connection, id: i32) -> Result<(), String> {
    conn.execute("DELETE FROM asset_event WHERE id = ?1", params![id])
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ---- thin command wrappers ----

#[tauri::command]
pub async fn list_asset_events_cmd(
    db: State<'_, Db>,
    filter: AssetEventFilter,
) -> Result<Vec<AssetEvent>, String> {
    let conn = db.conn().await?;
    list_asset_events(&conn, &filter).await
}

#[tauri::command]
pub async fn get_asset_event_cmd(db: State<'_, Db>, id: i32) -> Result<AssetEvent, String> {
    let conn = db.conn().await?;
    get_asset_event(&conn, id).await
}

#[tauri::command]
pub async fn create_asset_event_cmd(
    db: State<'_, Db>,
    event: NewAssetEvent,
) -> Result<AssetEvent, String> {
    let conn = db.conn().await?;
    create_asset_event(&conn, &event).await
}

#[tauri::command]
pub async fn update_asset_event_cmd(
    db: State<'_, Db>,
    event: UpdateAssetEvent,
) -> Result<AssetEvent, String> {
    let conn = db.conn().await?;
    update_asset_event(&conn, &event).await
}

#[tauri::command]
pub async fn delete_asset_event_cmd(db: State<'_, Db>, id: i32) -> Result<(), String> {
    let conn = db.conn().await?;
    delete_asset_event(&conn, id).await
}
