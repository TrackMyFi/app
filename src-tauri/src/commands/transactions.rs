use crate::db::Db;
use crate::models::Transaction;
use libsql::{params, Connection, Value};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use ts_rs::TS;
use tauri::State;

#[derive(Deserialize, Clone)]
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
    pub is_withdrawal: bool,
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
    pub is_withdrawal: bool,
    pub update_balance: bool,
    pub updated_at: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TransactionFilter {
    #[serde(default)]
    pub account_ids: Vec<i32>,
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    #[serde(default)]
    pub search_terms: Vec<String>,
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

/// Per-period (month or year) aggregated cash-flow stats, used for median
/// comparison on the Transactions page. Each row is one calendar period.
#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct PeriodStats {
    /// Calendar period key — "YYYY-MM" for months, "YYYY" for years.
    pub period: String,
    pub income: f64,
    /// Total spending, excluding savings/contributions.
    pub expense: f64,
    /// Total savings / investment contributions (withdrawals subtract).
    pub savings: f64,
    /// Income minus spending (savings is not subtracted — the money is still yours).
    pub net: f64,
    pub cat_fixed: f64,
    pub cat_discretionary: f64,
    pub cat_uncategorized: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeriodStatsFilter {
    #[serde(default)]
    pub account_ids: Vec<i32>,
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub search_terms: Vec<String>,
    /// "month" → group by YYYY-MM;  "year" → group by YYYY.
    pub group_by: String,
    /// The current period key to exclude so a period isn't compared against itself.
    pub exclude_period: String,
}

fn account_type_is_liability(ty: &str) -> bool {
    ty == "liability" || ty == "mortgage"
}

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

// Build the WHERE clause + positional params from a filter.
fn build_where(f: &TransactionFilter, params: &mut Vec<Value>) -> String {
    let mut clauses: Vec<String> = Vec::new();
    if !f.account_ids.is_empty() {
        let ph = (0..f.account_ids.len()).map(|_| "?").collect::<Vec<_>>().join(", ");
        clauses.push(format!("(account_id IN ({ph}) OR transfer_account_id IN ({ph}))"));
        for &id in &f.account_ids { params.push(Value::Integer(id as i64)); }
        for &id in &f.account_ids { params.push(Value::Integer(id as i64)); }
    }
    if !f.types.is_empty() {
        let ph = (0..f.types.len()).map(|_| "?").collect::<Vec<_>>().join(", ");
        clauses.push(format!("type IN ({ph})"));
        for t in &f.types { params.push(Value::Text(t.clone())); }
    }
    if !f.categories.is_empty() {
        let ph = (0..f.categories.len()).map(|_| "?").collect::<Vec<_>>().join(", ");
        clauses.push(format!("category IN ({ph})"));
        for c in &f.categories { params.push(Value::Text(c.clone())); }
    }
    if let Some(s) = &f.start_date {
        clauses.push("date >= ?".into());
        params.push(Value::Text(s.clone()));
    }
    if let Some(e) = &f.end_date {
        clauses.push("date <= ?".into());
        params.push(Value::Text(e.clone()));
    }
    if !f.search_terms.is_empty() {
        let sub = f.search_terms.iter().map(|_| "description LIKE ?").collect::<Vec<_>>().join(" OR ");
        clauses.push(format!("({sub})"));
        for term in &f.search_terms { params.push(Value::Text(format!("%{term}%"))); }
    }
    if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    }
}

/// Pure balance-delta computation with a pre-loaded `is_liability` bool.
/// Mirrors `side_delta` but avoids a DB query per row during bulk import.
fn side_delta_pure(is_liability: bool, ty: &str, amount: f64, to_side: bool) -> f64 {
    if to_side {
        if is_liability { -amount } else { amount }
    } else if ty == "transfer" {
        if is_liability { amount } else { -amount }
    } else {
        let asset_delta = if ty == "income" { amount } else { -amount };
        if is_liability { -asset_delta } else { asset_delta }
    }
}

/// Escape single quotes in a string for use as a SQL string literal.
fn sql_escape(s: &str) -> String {
    s.replace('\'', "''")
}

/// Newest balance in a date-sorted `(date, balance)` slice where `date <= query_date`.
fn mem_base_balance(snaps: &[(String, f64)], date: &str) -> f64 {
    snaps
        .iter()
        .rev()
        .find(|(d, _)| d.as_str() <= date)
        .map(|(_, b)| *b)
        .unwrap_or(0.0)
}

/// Insert `(date, balance)` into a sorted Vec, placing it after all existing entries
/// at the same date so backward iteration finds the most recently inserted entry first.
fn mem_snaps_insert(snaps: &mut Vec<(String, f64)>, date: String, balance: f64) {
    // partition_point returns the first index where d > date; inserting there
    // places the new entry after all same-or-earlier entries.
    let pos = snaps.partition_point(|(d, _)| d.as_str() <= date.as_str());
    snaps.insert(pos, (date, balance));
}

pub async fn list_transactions(
    conn: &Connection,
    f: &TransactionFilter,
) -> Result<TransactionPage, String> {
    // page rows
    let mut row_params: Vec<Value> = Vec::new();
    let where_sql = build_where(f, &mut row_params);
    // SQLite treats LIMIT -1 as "no limit"; use that when the caller passes null
    // so annual queries (which set limit: null) aren't silently capped at 200 rows.
    let limit = f.limit.unwrap_or(-1);
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
///
/// When Turso sync is enabled, writes are forwarded over HTTP to the primary.
/// This implementation collapses the previous ~3N round-trips (one SELECT + one
/// UPDATE per snapshot) into 4 total: balance_before, chain load, is_liability,
/// bulk txn fetch, and a single execute_batch for all UPDATEs.
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

    if chain.is_empty() {
        return Ok(());
    }

    // Preload is_liability once; replaces the per-snapshot call that previously
    // fired inside snapshot_delta() → side_delta() → is_liability().
    let liability = is_liability(conn, account_id).await?;

    // Load all txn delta data for this chain in one query — replaces the per-snapshot
    // snapshot_delta() SELECT. The IN clause is safe: these are integer primary keys
    // produced by the database itself.
    let id_list: String = chain
        .iter()
        .map(|(id, _)| id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let txn_sql = format!(
        "SELECT generated_balance_id, generated_balance_to_id, type, amount \
         FROM txn \
         WHERE generated_balance_id IN ({id_list}) OR generated_balance_to_id IN ({id_list})"
    );
    let mut txn_rows = conn
        .query(&txn_sql, ())
        .await
        .map_err(|e| e.to_string())?;

    // Map: snapshot_id → signed delta for this account.
    // A snapshot absent from this map is a manual anchor (resets running balance).
    let mut delta_map: HashMap<i32, f64> = HashMap::new();
    while let Some(r) = txn_rows.next().await.map_err(|e| e.to_string())? {
        let gen_id: Option<i32> = r.get(0).map_err(|e| e.to_string())?;
        let gen_to_id: Option<i32> = r.get(1).map_err(|e| e.to_string())?;
        let ty: String = r.get(2).map_err(|e| e.to_string())?;
        let amount: f64 = r.get(3).map_err(|e| e.to_string())?;

        // to_side = false: this account is the source (or sole owner for non-transfers).
        if let Some(id) = gen_id {
            delta_map.insert(id, side_delta_pure(liability, &ty, amount, false));
        }
        // to_side = true: this account is the transfer destination.
        if let Some(id) = gen_to_id {
            delta_map.insert(id, side_delta_pure(liability, &ty, amount, true));
        }
    }

    let mut updates: Vec<(f64, i32)> = Vec::new();
    for (id, current) in chain {
        match delta_map.get(&id) {
            Some(&delta) => {
                running += delta;
                if (running - current).abs() > 1e-6 {
                    updates.push((running, id));
                }
            }
            // Manual snapshot: absolute anchor that resets the running balance.
            None => running = current,
        }
    }

    if updates.is_empty() {
        return Ok(());
    }

    // Single execute_batch for all UPDATEs — mirrors bulk_create_transactions_with_snapshots:
    // one BEGIN/COMMIT = one HTTP request to the Turso primary instead of N round-trips.
    let mut sql = String::with_capacity(updates.len() * 80 + 16);
    sql.push_str("BEGIN;\n");
    for (new_balance, snap_id) in &updates {
        sql.push_str(&format!(
            "UPDATE account_balance SET balance = {} WHERE id = {};\n",
            new_balance, snap_id
        ));
    }
    sql.push_str("COMMIT;\n");

    conn.execute_batch(&sql).await.map(|_| ()).map_err(|e| e.to_string())
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
         category, is_contribution, is_withdrawal, import_source, generated_balance_id, \
         generated_balance_to_id, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)",
        params![
            t.account_id,
            t.transfer_account_id,
            t.amount,
            t.description.clone(),
            t.date.clone(),
            t.r#type.clone(),
            t.category.clone(),
            t.is_contribution,
            t.is_withdrawal,
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
         date=?5, type=?6, category=?7, is_contribution=?8, is_withdrawal=?9, \
         generated_balance_id=?10, generated_balance_to_id=?11, updated_at=?12 WHERE id=?13",
        params![
            t.account_id,
            t.transfer_account_id,
            t.amount,
            t.description.clone(),
            t.date.clone(),
            t.r#type.clone(),
            t.category.clone(),
            t.is_contribution,
            t.is_withdrawal,
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
             category, is_contribution, is_withdrawal, import_source, generated_balance_id, \
             generated_balance_to_id, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'csv', NULL, NULL, ?10, ?10)",
            params![
                t.account_id,
                t.transfer_account_id,
                t.amount,
                t.description.clone(),
                t.date.clone(),
                t.r#type.clone(),
                t.category.clone(),
                t.is_contribution,
                t.is_withdrawal,
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

/// Import many transactions with balance snapshot generation.
///
/// Rows must arrive sorted by date (ascending). The prior implementation called
/// `create_transaction` per row, making ~3 remote round-trips per row (base_balance
/// read + snapshot INSERT + txn INSERT). This rewrite instead:
///
/// 1. Preloads account types and all existing balance snapshots for each account
///    involved (local reads, no network hops on an embedded replica).
/// 2. Walks rows in order, computing snapshot values in memory (no per-row reads).
/// 3. Pre-reads `sqlite_sequence` to determine the IDs the next INSERTs will receive.
/// 4. Emits a single `execute_batch` call containing all snapshot + txn INSERTs
///    inside one BEGIN/COMMIT — exactly ONE remote round-trip for all writes.
pub async fn bulk_create_transactions_with_snapshots(
    conn: &Connection,
    rows: &[NewTransaction],
) -> Result<i64, String> {
    if rows.is_empty() {
        return Ok(0);
    }

    // Collect the unique account IDs referenced in this batch.
    let mut account_ids: Vec<i32> = Vec::new();
    for t in rows {
        if !account_ids.contains(&t.account_id) {
            account_ids.push(t.account_id);
        }
        if let Some(id) = t.transfer_account_id {
            if !account_ids.contains(&id) {
                account_ids.push(id);
            }
        }
    }

    // Preload account types (local reads — served from replica file, no network hop).
    let mut account_is_liability: HashMap<i32, bool> = HashMap::new();
    for &id in &account_ids {
        account_is_liability.insert(id, is_liability(conn, id).await?);
    }

    // Preload all existing balance snapshots per account, sorted date ASC.
    // Keeps our in-memory `base_balance` simulation accurate: manual balance
    // entries between import rows act as anchors, matching what the per-row path
    // would see via its `base_balance` DB queries.
    let mut mem_snaps: HashMap<i32, Vec<(String, f64)>> = HashMap::new();
    for &id in &account_ids {
        let mut snaps: Vec<(String, f64)> = Vec::new();
        let mut rows_q = conn
            .query(
                "SELECT recorded_at, balance FROM account_balance \
                 WHERE account_id = ?1 ORDER BY recorded_at ASC, id ASC",
                params![id],
            )
            .await
            .map_err(|e| e.to_string())?;
        while let Some(r) = rows_q.next().await.map_err(|e| e.to_string())? {
            snaps.push((
                r.get::<String>(0).map_err(|e| e.to_string())?,
                r.get::<f64>(1).map_err(|e| e.to_string())?,
            ));
        }
        mem_snaps.insert(id, snaps);
    }

    // Walk rows in date order, computing snapshot values in memory.
    struct PendingSnap {
        account_id: i32,
        balance: f64,
        recorded_at: String,
    }
    let mut pending_snaps: Vec<PendingSnap> = Vec::new();
    // Per input row: indices into pending_snaps for generated_balance_id / _to_id.
    let mut txn_snap_idxs: Vec<(Option<usize>, Option<usize>)> = Vec::new();

    for t in rows {
        if !t.update_balance {
            txn_snap_idxs.push((None, None));
            continue;
        }

        let is_liab_src = *account_is_liability.get(&t.account_id).unwrap_or(&false);

        if t.r#type == "transfer" {
            let to = t.transfer_account_id.ok_or("transfer requires transferAccountId")?;
            let is_liab_dst = *account_is_liability.get(&to).unwrap_or(&false);

            let src_base = mem_base_balance(mem_snaps.get(&t.account_id).unwrap(), &t.date);
            let src_new = src_base + side_delta_pure(is_liab_src, "transfer", t.amount, false);

            let dst_base = mem_base_balance(mem_snaps.get(&to).unwrap(), &t.date);
            let dst_new = dst_base + side_delta_pure(is_liab_dst, "transfer", t.amount, true);

            let snap_idx = pending_snaps.len();
            pending_snaps.push(PendingSnap {
                account_id: t.account_id,
                balance: src_new,
                recorded_at: t.date.clone(),
            });
            let snap_to_idx = pending_snaps.len();
            pending_snaps.push(PendingSnap {
                account_id: to,
                balance: dst_new,
                recorded_at: t.date.clone(),
            });

            // Simulate the inserts so subsequent rows see these balances.
            mem_snaps_insert(mem_snaps.get_mut(&t.account_id).unwrap(), t.date.clone(), src_new);
            mem_snaps_insert(mem_snaps.get_mut(&to).unwrap(), t.date.clone(), dst_new);

            txn_snap_idxs.push((Some(snap_idx), Some(snap_to_idx)));
        } else {
            let src_base = mem_base_balance(mem_snaps.get(&t.account_id).unwrap(), &t.date);
            let new_bal = src_base + side_delta_pure(is_liab_src, &t.r#type, t.amount, false);

            let snap_idx = pending_snaps.len();
            pending_snaps.push(PendingSnap {
                account_id: t.account_id,
                balance: new_bal,
                recorded_at: t.date.clone(),
            });
            mem_snaps_insert(mem_snaps.get_mut(&t.account_id).unwrap(), t.date.clone(), new_bal);

            txn_snap_idxs.push((Some(snap_idx), None));
        }
    }

    // Determine the first snapshot ID the batch will receive. SQLite AUTOINCREMENT
    // assigns IDs in order starting at seq+1 (seq = last-ever-assigned ID).
    let next_snap_id: i64 = if pending_snaps.is_empty() {
        0 // unused
    } else {
        let mut r = conn
            .query(
                "SELECT COALESCE(\
                    (SELECT seq FROM sqlite_sequence WHERE name='account_balance'), 0)",
                (),
            )
            .await
            .map_err(|e| e.to_string())?;
        let row = r
            .next()
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "missing sqlite_sequence row".to_string())?;
        row.get::<i64>(0).map_err(|e| e.to_string())? + 1
    };

    // Build one SQL batch — all snapshot + txn INSERTs inside a single BEGIN/COMMIT.
    // execute_batch sends this as one HTTP request to the Turso primary, collapsing
    // the previous N*3 round-trips into exactly 1.
    let mut sql = String::with_capacity(pending_snaps.len() * 120 + rows.len() * 300 + 32);
    sql.push_str("BEGIN;\n");

    for snap in &pending_snaps {
        sql.push_str(&format!(
            "INSERT INTO account_balance (account_id, balance, recorded_at) \
             VALUES ({}, {}, '{}');\n",
            snap.account_id,
            snap.balance,
            sql_escape(&snap.recorded_at),
        ));
    }

    for (i, t) in rows.iter().enumerate() {
        let (snap_idx_opt, snap_to_idx_opt) = txn_snap_idxs[i];
        let gen_id_sql = match snap_idx_opt {
            Some(idx) => (next_snap_id + idx as i64).to_string(),
            None => "NULL".to_string(),
        };
        let gen_to_id_sql = match snap_to_idx_opt {
            Some(idx) => (next_snap_id + idx as i64).to_string(),
            None => "NULL".to_string(),
        };
        let transfer_id_sql = match t.transfer_account_id {
            Some(id) => id.to_string(),
            None => "NULL".to_string(),
        };
        sql.push_str(&format!(
            "INSERT INTO txn (account_id, transfer_account_id, amount, description, \
             date, type, category, is_contribution, is_withdrawal, import_source, \
             generated_balance_id, generated_balance_to_id, created_at, updated_at) \
             VALUES ({}, {}, {}, '{}', '{}', '{}', '{}', {}, {}, '{}', {}, {}, '{}', '{}');\n",
            t.account_id,
            transfer_id_sql,
            t.amount,
            sql_escape(&t.description),
            sql_escape(&t.date),
            sql_escape(&t.r#type),
            sql_escape(&t.category),
            i32::from(t.is_contribution),
            i32::from(t.is_withdrawal),
            sql_escape(&t.import_source),
            gen_id_sql,
            gen_to_id_sql,
            sql_escape(&t.created_at),
            sql_escape(&t.created_at),
        ));
    }

    sql.push_str("COMMIT;\n");

    conn.execute_batch(&sql).await.map_err(|e| e.to_string())?;

    Ok(rows.len() as i64)
}

/// Return per-period (month or year) aggregated cash-flow stats across all time,
/// applying the same secondary filters used by the main transactions view.
/// Grouping, classification, and the per-period aggregation all happen in Rust so
/// only a small `Vec<PeriodStats>` is transferred to the frontend — not all rows.
pub async fn period_stats(
    conn: &Connection,
    f: &PeriodStatsFilter,
) -> Result<Vec<PeriodStats>, String> {
    let period_fmt = match f.group_by.as_str() {
        "year" => "%Y",
        _ => "%Y-%m",
    };

    // Reuse build_where for the secondary filters; all-time so no date range.
    let base = TransactionFilter {
        account_ids: f.account_ids.clone(),
        types: f.types.clone(),
        categories: f.categories.clone(),
        search_terms: f.search_terms.clone(),
        start_date: None,
        end_date: None,
        limit: None,
        offset: None,
    };
    let mut params: Vec<Value> = Vec::new();
    let where_sql = build_where(&base, &mut params);

    let sql = format!(
        "SELECT t.type, t.amount, t.category, t.is_contribution, t.is_withdrawal, \
         a1.type, a2.type, strftime('{period_fmt}', t.date) \
         FROM txn t \
         LEFT JOIN account a1 ON a1.id = t.account_id \
         LEFT JOIN account a2 ON a2.id = t.transfer_account_id \
         {where_sql} \
         ORDER BY 8"
    );

    let mut rows = conn
        .query(&sql, libsql::params_from_iter(params))
        .await
        .map_err(|e| e.to_string())?;

    // Aggregate into a BTreeMap keyed by period string (BTree keeps periods sorted).
    let mut by_period: BTreeMap<String, PeriodStats> = BTreeMap::new();

    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        let tx_type: String = row.get(0).map_err(|e| e.to_string())?;
        let amount: f64 = row.get(1).map_err(|e| e.to_string())?;
        let category: Option<String> = row.get(2).map_err(|e| e.to_string())?;
        let is_contribution: bool = row.get::<i64>(3).map_err(|e| e.to_string())? != 0;
        let is_withdrawal: bool = row.get::<i64>(4).map_err(|e| e.to_string())? != 0;
        let account_type: Option<String> = row.get(5).map_err(|e| e.to_string())?;
        let transfer_type: Option<String> = row.get(6).map_err(|e| e.to_string())?;
        let period: String = row.get(7).map_err(|e| e.to_string())?;

        // Skip the current period so it's never compared against itself.
        if period == f.exclude_period {
            continue;
        }

        let s = by_period.entry(period.clone()).or_insert(PeriodStats {
            period,
            income: 0.0,
            expense: 0.0,
            savings: 0.0,
            net: 0.0,
            cat_fixed: 0.0,
            cat_discretionary: 0.0,
            cat_uncategorized: 0.0,
        });

        // Classification mirrors classifyFlow() in src/lib/transactions/flow.ts.
        if is_contribution {
            s.savings += if is_withdrawal { -amount } else { amount };
        } else if tx_type == "income" {
            s.income += amount;
        } else if tx_type == "expense" {
            s.expense += amount;
            match category.as_deref().unwrap_or("uncategorized") {
                "fixed" => s.cat_fixed += amount,
                "discretionary" => s.cat_discretionary += amount,
                _ => s.cat_uncategorized += amount,
            }
        } else if tx_type == "transfer" {
            let src_liab = account_type_is_liability(account_type.as_deref().unwrap_or(""));
            let dst_liab = account_type_is_liability(transfer_type.as_deref().unwrap_or(""));
            if src_liab != dst_liab {
                if dst_liab {
                    // asset → liability: debt payment (spending outflow)
                    s.expense += amount;
                    s.cat_uncategorized += amount;
                } else {
                    // liability → asset: credit/refund (income inflow)
                    s.income += amount;
                }
            }
            // same-kind transfers (asset↔asset, liability↔liability) are neutral — skip
        }
    }

    // Compute net after all rows are aggregated.
    for s in by_period.values_mut() {
        s.net = s.income - s.expense;
    }

    Ok(by_period.into_values().collect())
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
pub async fn period_stats_cmd(
    db: State<'_, Db>,
    filter: PeriodStatsFilter,
) -> Result<Vec<PeriodStats>, String> {
    let conn = db.conn().await?;
    period_stats(&conn, &filter).await
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
    // Dedicated connection: this path opens a multi-statement transaction, which
    // must stay isolated from concurrent readers on the shared connection.
    let conn = db.fresh_conn().await?;
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

/// Walk every snapshot for `account_id` from the beginning of time, treating
/// manual snapshots as absolute anchors and recomputing every transaction-bound
/// snapshot as `running_balance + its_transaction_delta`.
#[tauri::command]
pub async fn rebuild_account_balances_cmd(
    db: State<'_, Db>,
    account_id: i32,
) -> Result<(), String> {
    let conn = db.conn().await?;
    reproject_account(&conn, account_id, "0001-01-01").await
}

#[cfg(test)]
mod tests {
    use super::*;
    use libsql::Builder;

    async fn setup_db() -> Connection {
        let db = Builder::new_local(":memory:").build().await.unwrap();
        let conn = db.connect().unwrap();
        crate::migrations::run(&conn).await.unwrap();
        conn
    }

    async fn insert_account(conn: &Connection, name: &str, ty: &str) -> i32 {
        conn.execute(
            "INSERT INTO account (name, type, is_active, include_in_fire_calculations, created_at) \
             VALUES (?1, ?2, 1, 0, '2024-01-01')",
            params![name, ty],
        )
        .await
        .unwrap();
        conn.last_insert_rowid() as i32
    }

    async fn insert_balance(conn: &Connection, account_id: i32, balance: f64, date: &str) {
        conn.execute(
            "INSERT INTO account_balance (account_id, balance, recorded_at) VALUES (?1, ?2, ?3)",
            params![account_id, balance, date],
        )
        .await
        .unwrap();
    }

    fn make_txn(account_id: i32, ty: &str, amount: f64, date: &str) -> NewTransaction {
        NewTransaction {
            account_id,
            transfer_account_id: None,
            amount,
            description: format!("{ty} {amount}"),
            date: date.to_string(),
            r#type: ty.to_string(),
            category: "test".to_string(),
            is_contribution: false,
            is_withdrawal: false,
            import_source: "csv".to_string(),
            update_balance: true,
            created_at: "2024-01-01T00:00:00".to_string(),
        }
    }

    fn make_transfer(from: i32, to: i32, amount: f64, date: &str) -> NewTransaction {
        NewTransaction {
            account_id: from,
            transfer_account_id: Some(to),
            amount,
            description: format!("transfer {amount}"),
            date: date.to_string(),
            r#type: "transfer".to_string(),
            category: "transfer".to_string(),
            is_contribution: false,
            is_withdrawal: false,
            import_source: "csv".to_string(),
            update_balance: true,
            created_at: "2024-01-01T00:00:00".to_string(),
        }
    }

    // Reads (account_id, amount, date, type, generated_balance, generated_balance_to) per txn,
    // ordered by date then id. The balance values verify that snapshot linking is correct.
    async fn read_txn_summaries(
        conn: &Connection,
    ) -> Vec<(i32, f64, String, String, Option<f64>, Option<f64>)> {
        let mut rows = conn
            .query(
                "SELECT t.account_id, t.amount, t.date, t.type, b1.balance, b2.balance \
                 FROM txn t \
                 LEFT JOIN account_balance b1 ON b1.id = t.generated_balance_id \
                 LEFT JOIN account_balance b2 ON b2.id = t.generated_balance_to_id \
                 ORDER BY t.date, t.id",
                (),
            )
            .await
            .unwrap();
        let mut out = Vec::new();
        while let Some(r) = rows.next().await.unwrap() {
            out.push((
                r.get::<i32>(0).unwrap(),
                r.get::<f64>(1).unwrap(),
                r.get::<String>(2).unwrap(),
                r.get::<String>(3).unwrap(),
                r.get::<Option<f64>>(4).unwrap(),
                r.get::<Option<f64>>(5).unwrap(),
            ));
        }
        out
    }

    // Reference implementation: the original row-by-row path using create_transaction.
    // Used only in tests to verify the new batched path produces identical output.
    async fn row_by_row_with_snapshots(
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

    #[tokio::test]
    async fn income_and_expense_accumulate_running_balance() {
        let conn = setup_db().await;
        let acc = insert_account(&conn, "Checking", "checking").await;

        let rows = vec![
            make_txn(acc, "income", 1000.0, "2024-01-15"),
            make_txn(acc, "expense", 500.0, "2024-01-20"),
            make_txn(acc, "income", 200.0, "2024-01-20"),
        ];

        let count = bulk_create_transactions_with_snapshots(&conn, &rows).await.unwrap();
        assert_eq!(count, 3);

        let sums = read_txn_summaries(&conn).await;
        assert_eq!(sums.len(), 3);
        // 0 + 1000 = 1000
        assert_eq!(sums[0].4, Some(1000.0));
        // 1000 - 500 = 500
        assert_eq!(sums[1].4, Some(500.0));
        // 500 + 200 = 700 (two rows share the same date; second sees first's snapshot)
        assert_eq!(sums[2].4, Some(700.0));
    }

    #[tokio::test]
    async fn transfer_writes_two_snapshots() {
        let conn = setup_db().await;
        let checking = insert_account(&conn, "Checking", "checking").await;
        let savings = insert_account(&conn, "Savings", "savings").await;
        insert_balance(&conn, checking, 1000.0, "2024-01-01").await;
        insert_balance(&conn, savings, 500.0, "2024-01-01").await;

        let rows = vec![make_transfer(checking, savings, 300.0, "2024-01-15")];
        bulk_create_transactions_with_snapshots(&conn, &rows).await.unwrap();

        let sums = read_txn_summaries(&conn).await;
        assert_eq!(sums.len(), 1);
        assert_eq!(sums[0].4, Some(700.0)); // checking: 1000 - 300
        assert_eq!(sums[0].5, Some(800.0)); // savings:  500 + 300
    }

    #[tokio::test]
    async fn manual_balance_in_date_range_acts_as_anchor() {
        let conn = setup_db().await;
        let acc = insert_account(&conn, "Checking", "checking").await;
        // Manual balance at 2024-02-01 — between the two import rows.
        insert_balance(&conn, acc, 2000.0, "2024-02-01").await;

        let rows = vec![
            make_txn(acc, "income", 100.0, "2024-01-15"), // before the manual balance
            make_txn(acc, "income", 200.0, "2024-03-01"), // after  the manual balance
        ];
        bulk_create_transactions_with_snapshots(&conn, &rows).await.unwrap();

        let sums = read_txn_summaries(&conn).await;
        assert_eq!(sums.len(), 2);
        // Row at 2024-01-15: no snap before this date → base 0, balance after = 100
        assert_eq!(sums[0].4, Some(100.0));
        // Row at 2024-03-01: latest snap at or before this date is the manual 2000
        // (trumps the 100 snap at 2024-01-15), balance after = 2000 + 200 = 2200
        assert_eq!(sums[1].4, Some(2200.0));
    }

    #[tokio::test]
    async fn liability_expense_increases_balance() {
        let conn = setup_db().await;
        let credit = insert_account(&conn, "Credit Card", "liability").await;

        let rows = vec![
            make_txn(credit, "expense", 200.0, "2024-01-15"),
            make_txn(credit, "income", 50.0, "2024-01-20"), // payment / refund
        ];
        bulk_create_transactions_with_snapshots(&conn, &rows).await.unwrap();

        let sums = read_txn_summaries(&conn).await;
        // Expense raises liability balance: 0 + 200 = 200
        assert_eq!(sums[0].4, Some(200.0));
        // Income (payment) lowers liability balance: 200 - 50 = 150
        assert_eq!(sums[1].4, Some(150.0));
    }

    #[tokio::test]
    async fn batch_matches_row_by_row_output() {
        // Run the old reference path on conn1, the new batched path on conn2,
        // then assert that every txn's linked balance values are identical.
        let conn1 = setup_db().await;
        let c1 = insert_account(&conn1, "Checking", "checking").await;
        let s1 = insert_account(&conn1, "Savings", "savings").await;
        let cr1 = insert_account(&conn1, "Credit Card", "liability").await;
        insert_balance(&conn1, c1, 5000.0, "2024-01-01").await;
        insert_balance(&conn1, s1, 2000.0, "2024-01-01").await;

        let conn2 = setup_db().await;
        let c2 = insert_account(&conn2, "Checking", "checking").await;
        let s2 = insert_account(&conn2, "Savings", "savings").await;
        let cr2 = insert_account(&conn2, "Credit Card", "liability").await;
        insert_balance(&conn2, c2, 5000.0, "2024-01-01").await;
        insert_balance(&conn2, s2, 2000.0, "2024-01-01").await;

        // Account IDs are deterministic (1, 2, 3) in both fresh DBs.
        assert_eq!((c1, s1, cr1), (c2, s2, cr2));

        let rows1 = vec![
            make_txn(c1, "income", 3000.0, "2024-01-15"),
            make_txn(c1, "expense", 150.0, "2024-01-20"),
            make_transfer(c1, s1, 500.0, "2024-01-25"),
            make_txn(cr1, "expense", 200.0, "2024-02-01"),
            make_txn(c1, "income", 100.0, "2024-02-05"),
        ];
        let rows2 = rows1.clone();

        row_by_row_with_snapshots(&conn1, &rows1).await.unwrap();
        bulk_create_transactions_with_snapshots(&conn2, &rows2).await.unwrap();

        let sums1 = read_txn_summaries(&conn1).await;
        let sums2 = read_txn_summaries(&conn2).await;

        assert_eq!(sums1.len(), sums2.len(), "row count mismatch");
        for (i, (a, b)) in sums1.iter().zip(sums2.iter()).enumerate() {
            assert_eq!(a, b, "txn {i} differs:\n  row-by-row: {a:?}\n  batched:    {b:?}");
        }
    }
}
