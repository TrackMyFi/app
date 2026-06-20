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

async fn is_liability(conn: &Connection, account_id: i32) -> Result<bool, String> {
    let mut rows = conn
        .query(
            "SELECT type FROM account WHERE id = ?1",
            params![account_id],
        )
        .await
        .map_err(|e| e.to_string())?;
    match rows.next().await.map_err(|e| e.to_string())? {
        Some(r) => {
            let t = r.get::<String>(0).map_err(|e| e.to_string())?;
            Ok(t == "liability" || t == "mortgage")
        }
        None => Ok(false),
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

/// The signed amount a transaction adds to one account's running balance.
///
/// `to_side` is true when `account_id` is the *destination* of a transfer (the
/// account referenced by `generated_balance_to_id`). A liability stores debt
/// owed, so its balance moves opposite to an asset for the same money flow:
/// money sent out raises an asset's loss but raises a liability's debt, and money
/// received raises an asset but pays down (lowers) a liability's debt. Likewise a
/// refund (income) pays down a liability while a purchase (expense) adds to it.
async fn side_delta(
    conn: &Connection,
    account_id: i32,
    ty: &str,
    amount: f64,
    to_side: bool,
) -> Result<f64, String> {
    let liability = is_liability(conn, account_id).await?;
    let delta = if to_side {
        // transfer destination: money in
        if liability { -amount } else { amount }
    } else if ty == "transfer" {
        // transfer source: money out
        if liability { amount } else { -amount }
    } else {
        let asset_delta = if ty == "income" { amount } else { -amount };
        if liability { -asset_delta } else { asset_delta }
    };
    Ok(delta)
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
        let src_new = src_base + side_delta(conn, account_id, ty, amount, false).await?;
        let dst_new = dst_base + side_delta(conn, to, ty, amount, true).await?;
        let gen = insert_snapshot(conn, account_id, src_new, date).await?;
        let gen_to = insert_snapshot(conn, to, dst_new, date).await?;
        Ok((Some(gen), Some(gen_to)))
    } else {
        let base = base_balance(conn, account_id, date).await?;
        let delta = side_delta(conn, account_id, ty, amount, false).await?;
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

/// Balance of the most recent snapshot strictly before `date` (any kind), or 0.0
/// when none exists. Used as the starting point for a re-projection.
async fn balance_before(conn: &Connection, account_id: i32, date: &str) -> Result<f64, String> {
    let mut rows = conn
        .query(
            "SELECT balance FROM account_balance WHERE account_id = ?1 AND recorded_at < ?2 \
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

/// The signed delta a transaction-generated snapshot contributes, or `None` when
/// the snapshot is not tied to any transaction (a manual snapshot).
async fn snapshot_delta(
    conn: &Connection,
    account_id: i32,
    snapshot_id: i32,
) -> Result<Option<f64>, String> {
    let mut rows = conn
        .query(
            "SELECT type, amount, \
             CASE WHEN generated_balance_to_id = ?1 THEN 1 ELSE 0 END AS to_side \
             FROM txn WHERE generated_balance_id = ?1 OR generated_balance_to_id = ?1 LIMIT 1",
            params![snapshot_id],
        )
        .await
        .map_err(|e| e.to_string())?;
    match rows.next().await.map_err(|e| e.to_string())? {
        Some(r) => {
            let ty: String = r.get(0).map_err(|e| e.to_string())?;
            let amount: f64 = r.get(1).map_err(|e| e.to_string())?;
            let to_side = r.get::<i64>(2).map_err(|e| e.to_string())? != 0;
            Ok(Some(side_delta(conn, account_id, &ty, amount, to_side).await?))
        }
        None => Ok(None),
    }
}

/// Recompute every transaction-generated snapshot for `account_id` dated on or
/// after `from_date`, walking chronologically and rebuilding each as
/// `running_balance + its transaction's delta`. Manual snapshots are left
/// untouched and reset the running balance, so they anchor the chain: a delta
/// introduced by an out-of-order transaction propagates forward through later
/// generated snapshots but stops at the next manual snapshot.
///
/// Call this only from the single-transaction command paths. Bulk import feeds
/// rows in ascending date order, so each new snapshot already lands at the end of
/// the chain and re-projecting per row would be needless O(n^2) work.
async fn reproject_account(
    conn: &Connection,
    account_id: i32,
    from_date: &str,
) -> Result<(), String> {
    let mut running = balance_before(conn, account_id, from_date).await?;

    // Collect the forward chain first; we can't run UPDATEs while a query's rows
    // are still streaming on the same connection.
    let mut rows = conn
        .query(
            "SELECT id, balance FROM account_balance WHERE account_id = ?1 AND recorded_at >= ?2 \
             ORDER BY recorded_at ASC, id ASC",
            params![account_id, from_date],
        )
        .await
        .map_err(|e| e.to_string())?;
    let mut chain: Vec<(i32, f64)> = Vec::new();
    while let Some(r) = rows.next().await.map_err(|e| e.to_string())? {
        chain.push((
            r.get::<i32>(0).map_err(|e| e.to_string())?,
            r.get::<f64>(1).map_err(|e| e.to_string())?,
        ));
    }

    for (id, current) in chain {
        match snapshot_delta(conn, account_id, id).await? {
            Some(delta) => {
                running += delta;
                if (running - current).abs() > 1e-6 {
                    conn.execute(
                        "UPDATE account_balance SET balance = ?1 WHERE id = ?2",
                        params![running, id],
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                }
            }
            // Manual snapshot: an absolute anchor that resets the running balance.
            None => running = current,
        }
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
///
/// Wrapped in a single transaction so all inserts are committed in one round-trip
/// to Turso instead of one HTTP request per row (embedded replica write path).
pub async fn bulk_create_transactions(
    conn: &Connection,
    rows: &[NewTransaction],
) -> Result<i64, String> {
    let tx = conn.transaction().await.map_err(|e| e.to_string())?;
    let mut count = 0i64;
    for t in rows {
        tx.execute(
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
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(count)
}

/// Import many transactions with balance snapshot generation in a single database
/// transaction. Rows must arrive sorted by date (ascending) so each base_balance
/// query sees the snapshots written by prior rows in the same transaction.
///
/// Uses manual BEGIN/COMMIT (not conn.transaction()) so we can reuse the existing
/// create_transaction inner function and all its snapshot helpers unchanged.
pub async fn bulk_create_transactions_with_snapshots(
    conn: &Connection,
    rows: &[NewTransaction],
) -> Result<i64, String> {
    conn.execute("BEGIN", ()).await.map_err(|e| e.to_string())?;
    let mut count = 0i64;
    for t in rows {
        if let Err(e) = create_transaction(conn, t).await {
            conn.execute("ROLLBACK", ()).await.ok();
            return Err(e);
        }
        count += 1;
    }
    conn.execute("COMMIT", ()).await.map_err(|e| e.to_string())?;
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

/// Re-project `from_date` forward for each distinct account in `accounts`, so an
/// out-of-order change ripples through later transaction-tied snapshots.
async fn reproject_accounts(
    conn: &Connection,
    accounts: &[Option<i32>],
    from_date: &str,
) -> Result<(), String> {
    let mut seen: Vec<i32> = Vec::new();
    for acc in accounts.iter().flatten() {
        if !seen.contains(acc) {
            seen.push(*acc);
            reproject_account(conn, *acc, from_date).await?;
        }
    }
    Ok(())
}

/// Create a single transaction, then ripple its snapshot forward through any
/// later transaction-tied snapshots (see [`reproject_account`]). This is the
/// path used by the UI's add-transaction form — distinct from
/// [`bulk_create_transactions_with_snapshots`], whose ascending-order rows never
/// land out of sequence and so need no re-projection.
pub async fn create_transaction_synced(
    conn: &Connection,
    t: &NewTransaction,
) -> Result<i32, String> {
    let id = create_transaction(conn, t).await?;
    // Only a balance-updating transaction writes a snapshot worth rippling.
    if t.update_balance {
        reproject_accounts(conn, &[Some(t.account_id), t.transfer_account_id], &t.date).await?;
    }
    Ok(id)
}

/// Update a single transaction, then re-project both its old and new positions.
/// An edit can move the date or switch accounts, so every account that the
/// transaction touched before or after the change is rippled from whichever date
/// comes first.
pub async fn update_transaction_synced(
    conn: &Connection,
    t: &UpdateTransaction,
) -> Result<(), String> {
    let old = get_transaction(conn, t.id).await?;
    update_transaction(conn, t).await?;
    let from_date = old.date.as_str().min(t.date.as_str());
    reproject_accounts(
        conn,
        &[
            Some(old.account_id),
            old.transfer_account_id,
            Some(t.account_id),
            t.transfer_account_id,
        ],
        from_date,
    )
    .await
}

/// Delete a single transaction, then re-project the accounts it touched so later
/// snapshots drop the deleted transaction's contribution.
pub async fn delete_transaction_synced(conn: &Connection, id: i32) -> Result<(), String> {
    let old = get_transaction(conn, id).await?;
    delete_transaction(conn, id).await?;
    reproject_accounts(conn, &[Some(old.account_id), old.transfer_account_id], &old.date).await
}

#[tauri::command]
pub async fn create_transaction_cmd(
    db: State<'_, Db>,
    transaction: NewTransaction,
) -> Result<i32, String> {
    let conn = db.conn().await?;
    create_transaction_synced(&conn, &transaction).await
}

#[tauri::command]
pub async fn update_transaction_cmd(
    db: State<'_, Db>,
    transaction: UpdateTransaction,
) -> Result<(), String> {
    let conn = db.conn().await?;
    update_transaction_synced(&conn, &transaction).await
}

#[tauri::command]
pub async fn delete_transaction_cmd(db: State<'_, Db>, id: i32) -> Result<(), String> {
    let conn = db.conn().await?;
    delete_transaction_synced(&conn, id).await
}

#[tauri::command]
pub async fn bulk_create_transactions_cmd(
    db: State<'_, Db>,
    transactions: Vec<NewTransaction>,
) -> Result<i64, String> {
    let conn = db.conn().await?;
    bulk_create_transactions(&conn, &transactions).await
}

#[tauri::command]
pub async fn bulk_create_transactions_with_snapshots_cmd(
    db: State<'_, Db>,
    transactions: Vec<NewTransaction>,
) -> Result<i64, String> {
    let conn = db.conn().await?;
    bulk_create_transactions_with_snapshots(&conn, &transactions).await
}
