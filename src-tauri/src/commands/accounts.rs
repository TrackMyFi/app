use crate::db::Db;
use crate::models::{Account, AccountBalance};
use libsql::Connection;
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewAccount {
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub institution: Option<String>,
    pub include_in_fire_calculations: bool,
    pub created_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewBalance {
    pub account_id: i32,
    pub balance: f64,
    pub recorded_at: String,
}

fn row_to_account(row: &libsql::Row) -> Result<Account, String> {
    Ok(Account {
        id: row.get(0).map_err(|e| e.to_string())?,
        name: row.get(1).map_err(|e| e.to_string())?,
        r#type: row.get(2).map_err(|e| e.to_string())?,
        institution: row.get(3).map_err(|e| e.to_string())?,
        is_active: row.get::<i64>(4).map_err(|e| e.to_string())? != 0,
        include_in_fire_calculations: row.get::<i64>(5).map_err(|e| e.to_string())? != 0,
        created_at: row.get(6).map_err(|e| e.to_string())?,
    })
}

fn row_to_balance(row: &libsql::Row) -> Result<AccountBalance, String> {
    Ok(AccountBalance {
        id: row.get(0).map_err(|e| e.to_string())?,
        account_id: row.get(1).map_err(|e| e.to_string())?,
        balance: row.get(2).map_err(|e| e.to_string())?,
        recorded_at: row.get(3).map_err(|e| e.to_string())?,
    })
}

// ---- testable inner fns ----

pub async fn list_accounts(conn: &Connection) -> Result<Vec<Account>, String> {
    let mut rows = conn
        .query(
            "SELECT id, name, type, institution, is_active, include_in_fire_calculations, created_at \
             FROM account ORDER BY created_at",
            (),
        )
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(row_to_account(&row)?);
    }
    Ok(out)
}

pub async fn create_account(conn: &Connection, a: &NewAccount) -> Result<i32, String> {
    conn.execute(
        "INSERT INTO account (name, type, institution, is_active, include_in_fire_calculations, created_at) \
         VALUES (?1, ?2, ?3, 1, ?4, ?5)",
        libsql::params![
            a.name.clone(),
            a.r#type.clone(),
            a.institution.clone(),
            a.include_in_fire_calculations,
            a.created_at.clone()
        ],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid() as i32)
}

pub async fn archive_account(conn: &Connection, id: i32) -> Result<(), String> {
    conn.execute(
        "UPDATE account SET is_active = 0 WHERE id = ?1",
        libsql::params![id],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn unarchive_account(conn: &Connection, id: i32) -> Result<(), String> {
    conn.execute(
        "UPDATE account SET is_active = 1 WHERE id = ?1",
        libsql::params![id],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Permanently delete an account and all of its balance snapshots.
/// Balances are removed explicitly rather than relying on FK cascade,
/// since SQLite foreign-key enforcement is off by default per connection.
pub async fn delete_account(conn: &Connection, id: i32) -> Result<(), String> {
    conn.execute(
        "DELETE FROM account_balance WHERE account_id = ?1",
        libsql::params![id],
    )
    .await
    .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM account WHERE id = ?1", libsql::params![id])
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn add_balance(conn: &Connection, b: &NewBalance) -> Result<(), String> {
    conn.execute(
        "INSERT INTO account_balance (account_id, balance, recorded_at) VALUES (?1, ?2, ?3)",
        libsql::params![b.account_id, b.balance, b.recorded_at.clone()],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn list_account_balances(
    conn: &Connection,
    account_id: i32,
) -> Result<Vec<AccountBalance>, String> {
    let mut rows = conn
        .query(
            "SELECT id, account_id, balance, recorded_at FROM account_balance \
             WHERE account_id = ?1 ORDER BY recorded_at",
            libsql::params![account_id],
        )
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(row_to_balance(&row)?);
    }
    Ok(out)
}

pub async fn list_all_balances(conn: &Connection) -> Result<Vec<AccountBalance>, String> {
    let mut rows = conn
        .query(
            "SELECT id, account_id, balance, recorded_at FROM account_balance ORDER BY recorded_at",
            (),
        )
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(row_to_balance(&row)?);
    }
    Ok(out)
}

// ---- thin command wrappers ----

#[tauri::command]
pub async fn list_accounts_cmd(db: State<'_, Db>) -> Result<Vec<Account>, String> {
    let conn = db.conn().await?;
    list_accounts(&conn).await
}

#[tauri::command]
pub async fn create_account_cmd(db: State<'_, Db>, account: NewAccount) -> Result<i32, String> {
    let conn = db.conn().await?;
    create_account(&conn, &account).await
}

#[tauri::command]
pub async fn archive_account_cmd(db: State<'_, Db>, id: i32) -> Result<(), String> {
    let conn = db.conn().await?;
    archive_account(&conn, id).await
}

#[tauri::command]
pub async fn unarchive_account_cmd(db: State<'_, Db>, id: i32) -> Result<(), String> {
    let conn = db.conn().await?;
    unarchive_account(&conn, id).await
}

#[tauri::command]
pub async fn delete_account_cmd(db: State<'_, Db>, id: i32) -> Result<(), String> {
    let conn = db.conn().await?;
    delete_account(&conn, id).await
}

#[tauri::command]
pub async fn add_balance_cmd(db: State<'_, Db>, balance: NewBalance) -> Result<(), String> {
    let conn = db.conn().await?;
    add_balance(&conn, &balance).await
}

#[tauri::command]
pub async fn list_all_balances_cmd(db: State<'_, Db>) -> Result<Vec<AccountBalance>, String> {
    let conn = db.conn().await?;
    list_all_balances(&conn).await
}
