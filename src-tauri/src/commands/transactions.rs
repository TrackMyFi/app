use crate::db::Db;
use crate::models::Transaction;
use libsql::{params, Connection, Value};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use tauri::State;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewTransaction {
    pub account_id: i32,
    pub transfer_account_id: Option<i32>,
    pub amount: f64,
    pub description: String,
    pub date: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub category: String,
    pub is_contribution: bool,
    pub import_source: String,
    pub update_balance: bool,
    pub created_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTransaction {
    pub id: i32,
    pub account_id: i32,
    pub transfer_account_id: Option<i32>,
    pub amount: f64,
    pub description: String,
    pub date: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub category: String,
    pub is_contribution: bool,
    pub update_balance: bool,
    pub updated_at: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TransactionFilter {
    pub account_id: Option<i32>,
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    pub category: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct TransactionPage {
    pub rows: Vec<Transaction>,
    pub total_count: i32,
    pub total_income: f64,
    pub total_expense: f64,
    pub net: f64,
}

const COLS: &str = "id, account_id, transfer_account_id, amount, description, date, type, \
    category, is_contribution, import_source, generated_balance_id, generated_balance_to_id, \
    paycheck_id, created_at, updated_at";

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
        import_source: row.get(9).map_err(|e| e.to_string())?,
        generated_balance_id: row.get(10).map_err(|e| e.to_string())?,
        generated_balance_to_id: row.get(11).map_err(|e| e.to_string())?,
        paycheck_id: row.get(12).map_err(|e| e.to_string())?,
        created_at: row.get(13).map_err(|e| e.to_string())?,
        updated_at: row.get(14).map_err(|e| e.to_string())?,
    })
}

// Build the WHERE clause + positional params from a filter.
fn build_where(f: &TransactionFilter, params: &mut Vec<Value>) -> String {
    let mut clauses: Vec<String> = Vec::new();
    if let Some(a) = f.account_id {
        clauses.push("(account_id = ? OR transfer_account_id = ?)".into());
        params.push(Value::Integer(a as i64));
        params.push(Value::Integer(a as i64));
    }
    if let Some(t) = &f.r#type {
        clauses.push("type = ?".into());
        params.push(Value::Text(t.clone()));
    }
    if let Some(c) = &f.category {
        clauses.push("category = ?".into());
        params.push(Value::Text(c.clone()));
    }
    if let Some(s) = &f.start_date {
        clauses.push("date >= ?".into());
        params.push(Value::Text(s.clone()));
    }
    if let Some(e) = &f.end_date {
        clauses.push("date <= ?".into());
        params.push(Value::Text(e.clone()));
    }
    if let Some(q) = &f.search {
        clauses.push("description LIKE ?".into());
        params.push(Value::Text(format!("%{}%", q)));
    }
    if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    }
}

pub async fn list_transactions(
    conn: &Connection,
    f: &TransactionFilter,
) -> Result<TransactionPage, String> {
    // page rows
    let mut row_params: Vec<Value> = Vec::new();
    let where_sql = build_where(f, &mut row_params);
    let limit = f.limit.unwrap_or(200);
    let offset = f.offset.unwrap_or(0);
    row_params.push(Value::Integer(limit));
    row_params.push(Value::Integer(offset));
    let sql = format!(
        "SELECT {COLS} FROM txn {where_sql} ORDER BY date DESC, id DESC LIMIT ? OFFSET ?"
    );
    let mut rows = conn
        .query(&sql, libsql::params_from_iter(row_params))
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(row_to_txn(&row)?);
    }

    // totals over the full filter (transfers excluded)
    let mut agg_params: Vec<Value> = Vec::new();
    let agg_where = build_where(f, &mut agg_params);
    let agg_sql = format!(
        "SELECT \
           COUNT(*), \
           CAST(COALESCE(SUM(CASE WHEN type='income' THEN amount ELSE 0 END), 0) AS REAL), \
           CAST(COALESCE(SUM(CASE WHEN type='expense' THEN amount ELSE 0 END), 0) AS REAL) \
         FROM txn {agg_where}"
    );
    let mut agg = conn
        .query(&agg_sql, libsql::params_from_iter(agg_params))
        .await
        .map_err(|e| e.to_string())?;
    let arow = agg
        .next()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "aggregate row missing".to_string())?;
    let total_count: i64 = arow.get(0).map_err(|e| e.to_string())?;
    let total_income: f64 = arow.get(1).map_err(|e| e.to_string())?;
    let total_expense: f64 = arow.get(2).map_err(|e| e.to_string())?;

    Ok(TransactionPage {
        rows: out,
        total_count: total_count as i32,
        total_income,
        total_expense,
        net: total_income - total_expense,
    })
}

pub async fn get_transaction(conn: &Connection, id: i32) -> Result<Transaction, String> {
    let sql = format!("SELECT {COLS} FROM txn WHERE id = ?1");
    let mut rows = conn
        .query(&sql, params![id])
        .await
        .map_err(|e| e.to_string())?;
    match rows.next().await.map_err(|e| e.to_string())? {
        Some(row) => row_to_txn(&row),
        None => Err(format!("transaction {id} not found")),
    }
}

async fn base_balance(conn: &Connection, account_id: i32, date: &str) -> Result<f64, String> {
    let mut rows = conn
        .query(
            "SELECT balance FROM account_balance WHERE account_id = ?1 AND recorded_at <= ?2 \
             ORDER BY recorded_at DESC, id DESC LIMIT 1",
            params![account_id, date],
        )
        .await
        .map_err(|e| e.to_string())?;
    match rows.next().await.map_err(|e| e.to_string())? {
        Some(r) => r.get::<f64>(0).map_err(|e| e.to_string()),
        None => Ok(0.0),
    }
}

async fn insert_snapshot(
    conn: &Connection,
    account_id: i32,
    balance: f64,
    recorded_at: &str,
) -> Result<i32, String> {
    conn.execute(
        "INSERT INTO account_balance (account_id, balance, recorded_at) VALUES (?1, ?2, ?3)",
        params![account_id, balance, recorded_at],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid() as i32)
}

async fn delete_snapshot(conn: &Connection, id: i32) -> Result<(), String> {
    conn.execute("DELETE FROM account_balance WHERE id = ?1", params![id])
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// Returns (generated_balance_id, generated_balance_to_id) for a materialized txn.
async fn materialize_snapshots(
    conn: &Connection,
    account_id: i32,
    transfer_account_id: Option<i32>,
    amount: f64,
    ty: &str,
    date: &str,
) -> Result<(Option<i32>, Option<i32>), String> {
    if ty == "transfer" {
        let to = transfer_account_id.ok_or("transfer requires transferAccountId")?;
        let src_base = base_balance(conn, account_id, date).await?;
        let dst_base = base_balance(conn, to, date).await?;
        let gen = insert_snapshot(conn, account_id, src_base - amount, date).await?;
        let gen_to = insert_snapshot(conn, to, dst_base + amount, date).await?;
        Ok((Some(gen), Some(gen_to)))
    } else {
        let delta = if ty == "income" { amount } else { -amount };
        let base = base_balance(conn, account_id, date).await?;
        let gen = insert_snapshot(conn, account_id, base + delta, date).await?;
        Ok((Some(gen), None))
    }
}

// Reads the current generated ids for a txn (used before re-materializing or deleting).
async fn generated_ids(conn: &Connection, txn_id: i32) -> Result<(Option<i32>, Option<i32>), String> {
    let mut rows = conn
        .query(
            "SELECT generated_balance_id, generated_balance_to_id FROM txn WHERE id = ?1",
            params![txn_id],
        )
        .await
        .map_err(|e| e.to_string())?;
    match rows.next().await.map_err(|e| e.to_string())? {
        Some(r) => Ok((
            r.get(0).map_err(|e| e.to_string())?,
            r.get(1).map_err(|e| e.to_string())?,
        )),
        None => Ok((None, None)),
    }
}

async fn clear_generated(conn: &Connection, ids: (Option<i32>, Option<i32>)) -> Result<(), String> {
    if let Some(id) = ids.0 {
        delete_snapshot(conn, id).await?;
    }
    if let Some(id) = ids.1 {
        delete_snapshot(conn, id).await?;
    }
    Ok(())
}

pub async fn create_transaction(conn: &Connection, t: &NewTransaction) -> Result<i32, String> {
    let (gen_id, gen_to_id) = if t.update_balance {
        materialize_snapshots(
            conn,
            t.account_id,
            t.transfer_account_id,
            t.amount,
            &t.r#type,
            &t.date,
        )
        .await?
    } else {
        (None, None)
    };

    conn.execute(
        "INSERT INTO txn (account_id, transfer_account_id, amount, description, date, type, \
         category, is_contribution, import_source, generated_balance_id, \
         generated_balance_to_id, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
        params![
            t.account_id,
            t.transfer_account_id,
            t.amount,
            t.description.clone(),
            t.date.clone(),
            t.r#type.clone(),
            t.category.clone(),
            t.is_contribution,
            t.import_source.clone(),
            gen_id,
            gen_to_id,
            t.created_at.clone()
        ],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid() as i32)
}

pub async fn update_transaction(conn: &Connection, t: &UpdateTransaction) -> Result<(), String> {
    // Remove any previously generated snapshots first, then re-materialize so the
    // result reflects the new amount/date/account.
    clear_generated(conn, generated_ids(conn, t.id).await?).await?;

    let (gen_id, gen_to_id) = if t.update_balance {
        materialize_snapshots(
            conn,
            t.account_id,
            t.transfer_account_id,
            t.amount,
            &t.r#type,
            &t.date,
        )
        .await?
    } else {
        (None, None)
    };

    conn.execute(
        "UPDATE txn SET account_id=?1, transfer_account_id=?2, amount=?3, description=?4, \
         date=?5, type=?6, category=?7, is_contribution=?8, generated_balance_id=?9, \
         generated_balance_to_id=?10, updated_at=?11 WHERE id=?12",
        params![
            t.account_id,
            t.transfer_account_id,
            t.amount,
            t.description.clone(),
            t.date.clone(),
            t.r#type.clone(),
            t.category.clone(),
            t.is_contribution,
            gen_id,
            gen_to_id,
            t.updated_at.clone(),
            t.id
        ],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn delete_transaction(conn: &Connection, id: i32) -> Result<(), String> {
    clear_generated(conn, generated_ids(conn, id).await?).await?;
    conn.execute("DELETE FROM txn WHERE id = ?1", params![id])
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Insert many transactions in one batch. Never materializes balance snapshots,
/// and forces import_source = "csv".
pub async fn bulk_create_transactions(
    conn: &Connection,
    rows: &[NewTransaction],
) -> Result<i64, String> {
    let mut count = 0i64;
    for t in rows {
        conn.execute(
            "INSERT INTO txn (account_id, transfer_account_id, amount, description, date, type, \
             category, is_contribution, import_source, generated_balance_id, \
             generated_balance_to_id, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'csv', NULL, NULL, ?9, ?9)",
            params![
                t.account_id,
                t.transfer_account_id,
                t.amount,
                t.description.clone(),
                t.date.clone(),
                t.r#type.clone(),
                t.category.clone(),
                t.is_contribution,
                t.created_at.clone()
            ],
        )
        .await
        .map_err(|e| e.to_string())?;
        count += 1;
    }
    Ok(count)
}

// ---- thin command wrappers ----

#[tauri::command]
pub async fn list_transactions_cmd(
    db: State<'_, Db>,
    filter: TransactionFilter,
) -> Result<TransactionPage, String> {
    let conn = db.conn().await?;
    list_transactions(&conn, &filter).await
}

#[tauri::command]
pub async fn get_transaction_cmd(db: State<'_, Db>, id: i32) -> Result<Transaction, String> {
    let conn = db.conn().await?;
    get_transaction(&conn, id).await
}

#[tauri::command]
pub async fn create_transaction_cmd(
    db: State<'_, Db>,
    transaction: NewTransaction,
) -> Result<i32, String> {
    let conn = db.conn().await?;
    create_transaction(&conn, &transaction).await
}

#[tauri::command]
pub async fn update_transaction_cmd(
    db: State<'_, Db>,
    transaction: UpdateTransaction,
) -> Result<(), String> {
    let conn = db.conn().await?;
    update_transaction(&conn, &transaction).await
}

#[tauri::command]
pub async fn delete_transaction_cmd(db: State<'_, Db>, id: i32) -> Result<(), String> {
    let conn = db.conn().await?;
    delete_transaction(&conn, id).await
}

#[tauri::command]
pub async fn bulk_create_transactions_cmd(
    db: State<'_, Db>,
    transactions: Vec<NewTransaction>,
) -> Result<i64, String> {
    let conn = db.conn().await?;
    bulk_create_transactions(&conn, &transactions).await
}
