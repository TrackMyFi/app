use crate::db::Db;
use crate::models::Transaction;
use libsql::{params, Connection};
use tauri::State;

const COLS: &str = "id, account_id, transfer_account_id, amount, description, date, type, \
    category, is_contribution, is_withdrawal, import_source, generated_balance_id, \
    generated_balance_to_id, paycheck_id, created_at, updated_at";

fn row_to_txn(row: &libsql::Row) -> Result<Transaction, String> {
    Ok(Transaction {
        id: row.get(0).map_err(|e| e.to_string())?,
        account_id: row.get(1).map_err(|e| e.to_string())?,
        transfer_account_id: row.get(2).map_err(|e| e.to_string())?,
        amount: row.get(3).map_err(|e| e.to_string())?,
        description: row.get(4).map_err(|e| e.to_string())?,
        date: row.get(5).map_err(|e| e.to_string())?,
        r#type: row.get(6).map_err(|e| e.to_string())?,
        category: row.get(7).map_err(|e| e.to_string())?,
        is_contribution: row.get::<i64>(8).map_err(|e| e.to_string())? != 0,
        is_withdrawal: row.get::<i64>(9).map_err(|e| e.to_string())? != 0,
        import_source: row.get(10).map_err(|e| e.to_string())?,
        generated_balance_id: row.get(11).map_err(|e| e.to_string())?,
        generated_balance_to_id: row.get(12).map_err(|e| e.to_string())?,
        paycheck_id: row.get(13).map_err(|e| e.to_string())?,
        created_at: row.get(14).map_err(|e| e.to_string())?,
        updated_at: row.get(15).map_err(|e| e.to_string())?,
    })
}

pub async fn list_contribution_txns(conn: &Connection, year: i32) -> Result<Vec<Transaction>, String> {
    let sql = format!(
        "SELECT {COLS} FROM txn WHERE is_contribution = 1 \
         AND strftime('%Y', date) IN (?1, ?2) ORDER BY date DESC, id DESC"
    );
    let mut rows = conn
        .query(&sql, params![year.to_string(), (year - 1).to_string()])
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(row_to_txn(&row)?);
    }
    Ok(out)
}

pub async fn list_contribution_years(conn: &Connection) -> Result<Vec<String>, String> {
    let mut rows = conn
        .query(
            "SELECT DISTINCT strftime('%Y', date) AS year FROM txn \
             WHERE is_contribution = 1 ORDER BY year DESC",
            (),
        )
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(row.get::<String>(0).map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub async fn list_contribution_txns_cmd(db: State<'_, Db>, year: i32) -> Result<Vec<Transaction>, String> {
    let conn = db.conn().await?;
    list_contribution_txns(&conn, year).await
}

#[tauri::command]
pub async fn list_contribution_years_cmd(db: State<'_, Db>) -> Result<Vec<String>, String> {
    let conn = db.conn().await?;
    list_contribution_years(&conn).await
}
