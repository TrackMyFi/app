use crate::db::Db;
use crate::models::{Paycheck, PaycheckDeduction, EmployerMatchItem};
use libsql::{params, Connection};
use serde::Deserialize;
use tauri::State;

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
    pub created_at: String,
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
    deductions, employer_match, import_source, created_at, updated_at";

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
        import_source: row.get(13).map_err(|e| e.to_string())?,
        created_at: row.get(14).map_err(|e| e.to_string())?,
        updated_at: row.get(15).map_err(|e| e.to_string())?,
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
         deductions, employer_match, import_source, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'manual', ?13, ?13)",
        params![
            p.pay_date.clone(), p.employer.clone(), p.pay_period.clone(),
            p.gross_amount, p.net_amount,
            p.federal_tax, p.state_tax, p.local_tax, p.social_security_tax, p.medicare_tax,
            deductions_json, employer_match_json, p.created_at.clone()
        ],
    )
    .await
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid() as i32;
    auto_create_contributions(conn, id, &p.pay_date, &p.deductions, &p.employer_match, &p.created_at).await?;
    get_paycheck(conn, id).await
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
