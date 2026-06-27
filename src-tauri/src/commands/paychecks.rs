use crate::db::Db;
use crate::models::{Paycheck, PaycheckDeduction, EmployerMatchItem};
use libsql::{params, Connection};
use serde::Deserialize;
use tauri::State;

use super::transactions::{clear_generated, materialize_snapshots, reproject_accounts};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewPaycheck {
    pub pay_date: String,
    pub employer: String,
    pub pay_period: String,
    pub gross_amount: f64,
    pub net_amount: f64,
    pub federal_tax: f64,
    pub state_tax: f64,
    pub local_tax: f64,
    pub social_security_tax: f64,
    pub medicare_tax: f64,
    pub deductions: Vec<PaycheckDeduction>,
    pub employer_match: Vec<EmployerMatchItem>,
    pub income_account_id: Option<i32>,
    pub update_balance: bool,
    pub created_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePaycheck {
    pub id: i32,
    pub pay_date: String,
    pub employer: String,
    pub pay_period: String,
    pub gross_amount: f64,
    pub net_amount: f64,
    pub federal_tax: f64,
    pub state_tax: f64,
    pub local_tax: f64,
    pub social_security_tax: f64,
    pub medicare_tax: f64,
    pub deductions: Vec<PaycheckDeduction>,
    pub employer_match: Vec<EmployerMatchItem>,
    pub income_account_id: Option<i32>,
    pub update_balance: bool,
    pub updated_at: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PaycheckFilter {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub employer: Option<String>,
}

const COLS: &str = "id, pay_date, employer, pay_period, gross_amount, net_amount, \
    federal_tax, state_tax, local_tax, social_security_tax, medicare_tax, \
    deductions, employer_match, income_account_id, import_source, created_at, updated_at";

fn row_to_paycheck(row: &libsql::Row) -> Result<Paycheck, String> {
    let deductions_json: String = row.get(11).map_err(|e| e.to_string())?;
    let employer_match_json: String = row.get(12).map_err(|e| e.to_string())?;
    Ok(Paycheck {
        id: row.get(0).map_err(|e| e.to_string())?,
        pay_date: row.get(1).map_err(|e| e.to_string())?,
        employer: row.get(2).map_err(|e| e.to_string())?,
        pay_period: row.get(3).map_err(|e| e.to_string())?,
        gross_amount: row.get(4).map_err(|e| e.to_string())?,
        net_amount: row.get(5).map_err(|e| e.to_string())?,
        federal_tax: row.get(6).map_err(|e| e.to_string())?,
        state_tax: row.get(7).map_err(|e| e.to_string())?,
        local_tax: row.get(8).map_err(|e| e.to_string())?,
        social_security_tax: row.get(9).map_err(|e| e.to_string())?,
        medicare_tax: row.get(10).map_err(|e| e.to_string())?,
        deductions: serde_json::from_str(&deductions_json).map_err(|e| e.to_string())?,
        employer_match: serde_json::from_str(&employer_match_json).map_err(|e| e.to_string())?,
        income_account_id: row.get(13).map_err(|e| e.to_string())?,
        import_source: row.get(14).map_err(|e| e.to_string())?,
        created_at: row.get(15).map_err(|e| e.to_string())?,
        updated_at: row.get(16).map_err(|e| e.to_string())?,
    })
}

async fn auto_create_contributions(
    conn: &Connection,
    paycheck_id: i32,
    pay_date: &str,
    deductions: &[PaycheckDeduction],
    employer_match: &[EmployerMatchItem],
    now: &str,
) -> Result<(), String> {
    for ded in deductions {
        if ded.contribution_account_type.as_deref().is_some_and(|s| !s.is_empty()) {
            if let Some(account_id) = ded.account_id {
                conn.execute(
                    "INSERT INTO txn (account_id, transfer_account_id, amount, description, date, \
                     type, category, is_contribution, import_source, paycheck_id, \
                     generated_balance_id, generated_balance_to_id, created_at, updated_at) \
                     VALUES (?1, NULL, ?2, ?3, ?4, 'income', 'savings', 1, 'paycheck', ?5, \
                     NULL, NULL, ?6, ?6)",
                    params![account_id, ded.amount, ded.label.clone(), pay_date, paycheck_id, now],
                )
                .await
                .map_err(|e| e.to_string())?;
            }
        }
    }
    for em in employer_match {
        if let Some(account_id) = em.account_id {
            conn.execute(
                "INSERT INTO txn (account_id, transfer_account_id, amount, description, date, \
                 type, category, is_contribution, import_source, paycheck_id, \
                 generated_balance_id, generated_balance_to_id, created_at, updated_at) \
                 VALUES (?1, NULL, ?2, ?3, ?4, 'income', 'savings', 1, 'paycheck', ?5, \
                 NULL, NULL, ?6, ?6)",
                params![account_id, em.amount, em.label.clone(), pay_date, paycheck_id, now],
            )
            .await
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Create the income transaction for a paycheck deposit account.
/// When `update_balance` is true, materializes a balance snapshot first and
/// wires it into the row via `generated_balance_id`, matching the transaction
/// command's create path. Reprojection is the caller's responsibility.
async fn auto_create_income_txn(
    conn: &Connection,
    paycheck_id: i32,
    income_account_id: Option<i32>,
    net_amount: f64,
    employer: &str,
    pay_date: &str,
    now: &str,
    update_balance: bool,
) -> Result<(), String> {
    let Some(account_id) = income_account_id else {
        return Ok(());
    };
    let description = format!("Paycheck – {}", employer);

    // Materialize before INSERT so the ID is available for the row in one write.
    let gen_id: Option<i32> = if update_balance {
        let (id, _) = materialize_snapshots(conn, account_id, None, net_amount, "income", pay_date).await?;
        id
    } else {
        None
    };

    conn.execute(
        "INSERT INTO txn (account_id, transfer_account_id, amount, description, date, \
         type, category, is_contribution, import_source, paycheck_id, \
         generated_balance_id, generated_balance_to_id, created_at, updated_at) \
         VALUES (?1, NULL, ?2, ?3, ?4, 'income', 'fixed', 0, 'paycheck', ?5, \
         ?6, NULL, ?7, ?7)",
        params![account_id, net_amount, description, pay_date, paycheck_id, gen_id, now],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Collect and clear any balance snapshots owned by this paycheck's transactions.
/// Must be called before the bulk `DELETE FROM txn WHERE paycheck_id = ?` so the
/// snapshot rows are removed before their referencing txn rows disappear.
async fn clear_paycheck_snapshots(conn: &Connection, paycheck_id: i32) -> Result<(), String> {
    let mut rows = conn
        .query(
            "SELECT generated_balance_id, generated_balance_to_id FROM txn WHERE paycheck_id = ?1",
            params![paycheck_id],
        )
        .await
        .map_err(|e| e.to_string())?;
    let mut snap_ids: Vec<(Option<i32>, Option<i32>)> = Vec::new();
    while let Some(r) = rows.next().await.map_err(|e| e.to_string())? {
        snap_ids.push((
            r.get(0).map_err(|e| e.to_string())?,
            r.get(1).map_err(|e| e.to_string())?,
        ));
    }
    for ids in snap_ids {
        clear_generated(conn, ids).await?;
    }
    Ok(())
}

pub async fn get_paycheck(conn: &Connection, id: i32) -> Result<Paycheck, String> {
    let sql = format!("SELECT {COLS} FROM paycheck WHERE id = ?1");
    let mut rows = conn.query(&sql, params![id]).await.map_err(|e| e.to_string())?;
    match rows.next().await.map_err(|e| e.to_string())? {
        Some(row) => row_to_paycheck(&row),
        None => Err(format!("paycheck {id} not found")),
    }
}

pub async fn list_paychecks(conn: &Connection, f: &PaycheckFilter) -> Result<Vec<Paycheck>, String> {
    let mut where_clauses: Vec<String> = Vec::new();
    let mut bind_params: Vec<libsql::Value> = Vec::new();

    if let Some(s) = &f.start_date {
        where_clauses.push("pay_date >= ?".into());
        bind_params.push(libsql::Value::Text(s.clone()));
    }
    if let Some(e) = &f.end_date {
        where_clauses.push("pay_date <= ?".into());
        bind_params.push(libsql::Value::Text(e.clone()));
    }
    if let Some(emp) = &f.employer {
        where_clauses.push("employer LIKE ?".into());
        bind_params.push(libsql::Value::Text(format!("%{}%", emp)));
    }

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    let sql = format!("SELECT {COLS} FROM paycheck {where_sql} ORDER BY pay_date DESC, id DESC");
    let mut rows = conn
        .query(&sql, libsql::params_from_iter(bind_params))
        .await
        .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(row_to_paycheck(&row)?);
    }
    Ok(out)
}

pub async fn create_paycheck(conn: &Connection, p: &NewPaycheck) -> Result<Paycheck, String> {
    let deductions_json = serde_json::to_string(&p.deductions).map_err(|e| e.to_string())?;
    let employer_match_json = serde_json::to_string(&p.employer_match).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO paycheck (pay_date, employer, pay_period, gross_amount, net_amount, \
         federal_tax, state_tax, local_tax, social_security_tax, medicare_tax, \
         deductions, employer_match, income_account_id, import_source, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 'manual', ?14, ?14)",
        params![
            p.pay_date.clone(), p.employer.clone(), p.pay_period.clone(),
            p.gross_amount, p.net_amount,
            p.federal_tax, p.state_tax, p.local_tax, p.social_security_tax, p.medicare_tax,
            deductions_json, employer_match_json, p.income_account_id, p.created_at.clone()
        ],
    )
    .await
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid() as i32;
    auto_create_contributions(conn, id, &p.pay_date, &p.deductions, &p.employer_match, &p.created_at).await?;
    auto_create_income_txn(conn, id, p.income_account_id, p.net_amount, &p.employer, &p.pay_date, &p.created_at, p.update_balance).await?;

    // Ripple the new snapshot forward through any later transaction-tied snapshots.
    if p.update_balance {
        reproject_accounts(conn, &[p.income_account_id], &p.pay_date).await?;
    }

    get_paycheck(conn, id).await
}

pub async fn update_paycheck(conn: &Connection, p: &UpdatePaycheck) -> Result<Paycheck, String> {
    let old = get_paycheck(conn, p.id).await?;

    // Remove any balance snapshots the old income txn wrote before deleting txns.
    // This ensures the chain heals correctly whether or not we'll create a new snapshot.
    clear_paycheck_snapshots(conn, p.id).await?;

    // Delete ALL txns previously created by this paycheck (contributions + income).
    conn.execute("DELETE FROM txn WHERE paycheck_id = ?1", params![p.id])
        .await
        .map_err(|e| e.to_string())?;

    let deductions_json = serde_json::to_string(&p.deductions).map_err(|e| e.to_string())?;
    let employer_match_json = serde_json::to_string(&p.employer_match).map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE paycheck SET pay_date=?1, employer=?2, pay_period=?3, gross_amount=?4, \
         net_amount=?5, federal_tax=?6, state_tax=?7, local_tax=?8, social_security_tax=?9, \
         medicare_tax=?10, deductions=?11, employer_match=?12, income_account_id=?13, \
         updated_at=?14 WHERE id=?15",
        params![
            p.pay_date.clone(), p.employer.clone(), p.pay_period.clone(),
            p.gross_amount, p.net_amount,
            p.federal_tax, p.state_tax, p.local_tax, p.social_security_tax, p.medicare_tax,
            deductions_json, employer_match_json, p.income_account_id, p.updated_at.clone(), p.id
        ],
    )
    .await
    .map_err(|e| e.to_string())?;

    auto_create_contributions(conn, p.id, &p.pay_date, &p.deductions, &p.employer_match, &p.updated_at).await?;
    auto_create_income_txn(conn, p.id, p.income_account_id, p.net_amount, &p.employer, &p.pay_date, &p.updated_at, p.update_balance).await?;

    // Reproject from min(old_date, new_date) for both old and new deposit accounts.
    // Runs unconditionally: we may have deleted a snapshot (heal the chain) or created a
    // new one (propagate it forward), and accounts or dates may have changed.
    let from_date = old.pay_date.as_str().min(p.pay_date.as_str());
    reproject_accounts(conn, &[old.income_account_id, p.income_account_id], from_date).await?;

    get_paycheck(conn, p.id).await
}

pub async fn delete_paycheck(conn: &Connection, id: i32) -> Result<(), String> {
    let old = get_paycheck(conn, id).await?;

    // Remove any balance snapshots owned by this paycheck's txns.
    clear_paycheck_snapshots(conn, id).await?;

    // Explicitly delete contribution transactions before deleting the paycheck.
    // PRAGMA foreign_keys is NOT enabled in this project, so ON DELETE CASCADE does not fire.
    conn.execute("DELETE FROM txn WHERE paycheck_id = ?1", params![id])
        .await
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM paycheck WHERE id = ?1", params![id])
        .await
        .map_err(|e| e.to_string())?;

    // Heal later snapshots that were computed off the deleted income txn's snapshot.
    reproject_accounts(conn, &[old.income_account_id], &old.pay_date).await?;

    Ok(())
}

// ---- thin command wrappers ----

#[tauri::command]
pub async fn list_paychecks_cmd(
    db: State<'_, Db>,
    filter: PaycheckFilter,
) -> Result<Vec<Paycheck>, String> {
    let conn = db.conn().await?;
    list_paychecks(&conn, &filter).await
}

#[tauri::command]
pub async fn get_paycheck_cmd(db: State<'_, Db>, id: i32) -> Result<Paycheck, String> {
    let conn = db.conn().await?;
    get_paycheck(&conn, id).await
}

#[tauri::command]
pub async fn create_paycheck_cmd(
    db: State<'_, Db>,
    paycheck: NewPaycheck,
) -> Result<Paycheck, String> {
    let conn = db.conn().await?;
    create_paycheck(&conn, &paycheck).await
}

#[tauri::command]
pub async fn update_paycheck_cmd(
    db: State<'_, Db>,
    paycheck: UpdatePaycheck,
) -> Result<Paycheck, String> {
    let conn = db.conn().await?;
    update_paycheck(&conn, &paycheck).await
}

#[tauri::command]
pub async fn delete_paycheck_cmd(db: State<'_, Db>, id: i32) -> Result<(), String> {
    let conn = db.conn().await?;
    delete_paycheck(&conn, id).await
}
