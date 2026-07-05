use crate::db::Db;
use crate::models::Transaction;
use libsql::{params, Connection};
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetMonth {
    pub year: i32,
    pub month: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetMonthTarget {
    pub savings_target: f64,
    pub source_year: i32,
    pub source_month: i32,
}

const TXN_COLS: &str = "id, account_id, transfer_account_id, amount, description, date, type, \
    category, is_contribution, is_withdrawal, import_source, generated_balance_id, \
    generated_balance_to_id, paycheck_id, vendor_category, simplefin_id, suppressed_as, \
    created_at, updated_at, raw_description";

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
        vendor_category: row.get(14).map_err(|e| e.to_string())?,
        simplefin_id: row.get(15).map_err(|e| e.to_string())?,
        suppressed_as: row.get(16).map_err(|e| e.to_string())?,
        created_at: row.get(17).map_err(|e| e.to_string())?,
        updated_at: row.get(18).map_err(|e| e.to_string())?,
        raw_description: row.get(19).map_err(|e| e.to_string())?,
    })
}

pub async fn list_budget_months(conn: &Connection) -> Result<Vec<BudgetMonth>, String> {
    let mut rows = conn
        .query(
            "SELECT DISTINCT CAST(strftime('%Y', date) AS INTEGER) AS year, \
             CAST(strftime('%m', date) AS INTEGER) AS month \
             FROM txn ORDER BY year DESC, month DESC",
            (),
        )
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(BudgetMonth {
            year: row.get(0).map_err(|e| e.to_string())?,
            month: row.get(1).map_err(|e| e.to_string())?,
        });
    }
    Ok(out)
}

pub async fn list_budget_txns(conn: &Connection, year: i32, month: i32) -> Result<Vec<Transaction>, String> {
    let sql = format!(
        "SELECT {TXN_COLS} FROM txn \
         WHERE strftime('%Y', date) = printf('%04d', ?1) \
           AND strftime('%m', date) = printf('%02d', ?2) \
           AND suppressed_as IS NULL \
         ORDER BY date ASC, id ASC"
    );
    let mut rows = conn
        .query(&sql, params![year, month])
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(row_to_txn(&row)?);
    }
    Ok(out)
}

pub async fn get_budget_month_target(conn: &Connection, year: i32, month: i32) -> Result<Option<BudgetMonthTarget>, String> {
    let mut rows = conn
        .query(
            "SELECT savings_target, year, month FROM budget_month \
             WHERE (year < ?1 OR (year = ?1 AND month <= ?2)) \
             ORDER BY year DESC, month DESC LIMIT 1",
            params![year, month],
        )
        .await
        .map_err(|e| e.to_string())?;
    match rows.next().await.map_err(|e| e.to_string())? {
        Some(row) => Ok(Some(BudgetMonthTarget {
            savings_target: row.get(0).map_err(|e| e.to_string())?,
            source_year: row.get(1).map_err(|e| e.to_string())?,
            source_month: row.get(2).map_err(|e| e.to_string())?,
        })),
        None => Ok(None),
    }
}

pub async fn set_budget_month_target(conn: &Connection, year: i32, month: i32, savings_target: f64) -> Result<(), String> {
    conn.execute(
        "INSERT INTO budget_month (year, month, savings_target) VALUES (?1, ?2, ?3) \
         ON CONFLICT(year, month) DO UPDATE SET savings_target = excluded.savings_target",
        params![year, month, savings_target],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ---- thin command wrappers ----

#[tauri::command]
pub async fn list_budget_months_cmd(db: State<'_, Db>) -> Result<Vec<BudgetMonth>, String> {
    let conn = db.conn().await?;
    list_budget_months(&conn).await
}

#[tauri::command]
pub async fn list_budget_txns_cmd(db: State<'_, Db>, year: i32, month: i32) -> Result<Vec<Transaction>, String> {
    let conn = db.conn().await?;
    list_budget_txns(&conn, year, month).await
}

#[tauri::command]
pub async fn get_budget_month_target_cmd(db: State<'_, Db>, year: i32, month: i32) -> Result<Option<BudgetMonthTarget>, String> {
    let conn = db.conn().await?;
    get_budget_month_target(&conn, year, month).await
}

#[tauri::command]
pub async fn set_budget_month_target_cmd(db: State<'_, Db>, year: i32, month: i32, savings_target: f64) -> Result<(), String> {
    let conn = db.conn().await?;
    set_budget_month_target(&conn, year, month, savings_target).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use libsql::Builder;

    async fn test_conn() -> Connection {
        let db = Builder::new_local(":memory:").build().await.unwrap();
        let conn = db.connect().unwrap();
        crate::migrations::run(&conn).await.unwrap();
        conn
    }

    async fn seed_txn(conn: &Connection, date: &str) {
        conn.execute(
            "INSERT INTO account (name, type, is_active, include_in_fire_calculations, created_at) \
             VALUES ('Checking', 'checking', 1, 0, '2025-01-01')",
            (),
        ).await.unwrap();
        let acct_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO txn (account_id, transfer_account_id, amount, description, date, \
             type, category, is_contribution, import_source, generated_balance_id, \
             generated_balance_to_id, paycheck_id, vendor_category, simplefin_id, created_at, updated_at) \
             VALUES (?1, NULL, 100.0, 'Test', ?2, 'income', 'fixed', 0, 'manual', \
             NULL, NULL, NULL, NULL, NULL, ?2, ?2)",
            params![acct_id, date],
        ).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_budget_month_target_none() {
        let conn = test_conn().await;
        let result = get_budget_month_target(&conn, 2025, 6).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_budget_month_target_exact() {
        let conn = test_conn().await;
        set_budget_month_target(&conn, 2025, 6, 2000.0).await.unwrap();
        let result = get_budget_month_target(&conn, 2025, 6).await.unwrap().unwrap();
        assert_eq!(result.savings_target, 2000.0);
        assert_eq!(result.source_year, 2025);
        assert_eq!(result.source_month, 6);
    }

    #[tokio::test]
    async fn test_get_budget_month_target_fallback() {
        let conn = test_conn().await;
        set_budget_month_target(&conn, 2025, 1, 1500.0).await.unwrap();
        // Request June 2025 — no record for that month, should fall back to Jan 2025
        let result = get_budget_month_target(&conn, 2025, 6).await.unwrap().unwrap();
        assert_eq!(result.savings_target, 1500.0);
        assert_eq!(result.source_year, 2025);
        assert_eq!(result.source_month, 1);
    }

    #[tokio::test]
    async fn test_get_budget_month_target_no_future_fallback() {
        let conn = test_conn().await;
        // A record exists for July — requesting June should NOT use July
        set_budget_month_target(&conn, 2025, 7, 1500.0).await.unwrap();
        let result = get_budget_month_target(&conn, 2025, 6).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_set_budget_month_target_upsert() {
        let conn = test_conn().await;
        set_budget_month_target(&conn, 2025, 6, 2000.0).await.unwrap();
        set_budget_month_target(&conn, 2025, 6, 2500.0).await.unwrap();
        let result = get_budget_month_target(&conn, 2025, 6).await.unwrap().unwrap();
        assert_eq!(result.savings_target, 2500.0);
    }

    #[tokio::test]
    async fn test_list_budget_months() {
        let conn = test_conn().await;
        seed_txn(&conn, "2025-06-15").await;
        seed_txn(&conn, "2025-06-20").await;
        seed_txn(&conn, "2025-05-10").await;
        let months = list_budget_months(&conn).await.unwrap();
        assert_eq!(months.len(), 2);
        assert_eq!(months[0].year, 2025);
        assert_eq!(months[0].month, 6);
        assert_eq!(months[1].year, 2025);
        assert_eq!(months[1].month, 5);
    }

    #[tokio::test]
    async fn test_list_budget_txns() {
        let conn = test_conn().await;
        seed_txn(&conn, "2025-06-15").await;
        seed_txn(&conn, "2025-07-01").await;
        let txns = list_budget_txns(&conn, 2025, 6).await.unwrap();
        assert_eq!(txns.len(), 1);
        assert!(txns[0].date.starts_with("2025-06"));
    }

}
