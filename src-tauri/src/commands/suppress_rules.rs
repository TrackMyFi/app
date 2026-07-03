use crate::db::Db;
use crate::models::SuppressRule;
use libsql::{params, Connection, Value};
use tauri::State;

/// The noise kinds a rule may stamp. Mirrors SUPPRESS_KINDS in
/// src/lib/transactions/constants.ts.
const KINDS: &[&str] = &["investment_activity", "fee", "interest"];

fn validate_kind(kind: &str) -> Result<(), String> {
    if KINDS.contains(&kind) {
        Ok(())
    } else {
        Err(format!("unknown suppress kind '{kind}'"))
    }
}

fn row_to_rule(row: &libsql::Row) -> Result<SuppressRule, String> {
    Ok(SuppressRule {
        id: row.get(0).map_err(|e| e.to_string())?,
        keyword: row.get(1).map_err(|e| e.to_string())?,
        kind: row.get(2).map_err(|e| e.to_string())?,
        account_id: row.get(3).map_err(|e| e.to_string())?,
        created_at: row.get(4).map_err(|e| e.to_string())?,
    })
}

pub async fn list_suppress_rules(conn: &Connection) -> Result<Vec<SuppressRule>, String> {
    let mut rows = conn
        .query(
            "SELECT id, keyword, kind, account_id, created_at FROM suppress_rules ORDER BY id",
            (),
        )
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(row_to_rule(&row)?);
    }
    Ok(out)
}

/// Re-derive txn.suppressed_as for every income/expense row from the current
/// rule set. suppressed_as is purely rule-derived (no manual per-row override),
/// so a full wipe-and-reapply is always correct and keeps every entry path —
/// SimpleFIN sync, CSV import, manual add — consistent without each one
/// duplicating the matching logic. First matching rule wins in id order,
/// mirroring category rules; matching is a case-insensitive substring test on
/// the description (SQLite LIKE is case-insensitive for ASCII).
///
/// Transfers are never suppressed: they are already cash-flow neutral, and a
/// keyword accidentally matching one side of a transfer pair would silently
/// unbalance the collapse heuristics. Paycheck-generated rows and contributions
/// are also exempt — a broad keyword must not silently drop savings or salary
/// from the FIRE numbers.
pub async fn apply_suppress_rules(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "UPDATE txn SET suppressed_as = NULL WHERE suppressed_as IS NOT NULL",
        (),
    )
    .await
    .map_err(|e| e.to_string())?;

    for rule in list_suppress_rules(conn).await? {
        let mut sql = String::from(
            "UPDATE txn SET suppressed_as = ? \
             WHERE suppressed_as IS NULL \
               AND type IN ('income', 'expense') \
               AND paycheck_id IS NULL \
               AND is_contribution = 0 \
               AND description LIKE ?",
        );
        let mut params: Vec<Value> = vec![
            Value::Text(rule.kind.clone()),
            Value::Text(format!("%{}%", rule.keyword)),
        ];
        if let Some(account_id) = rule.account_id {
            sql.push_str(" AND account_id = ?");
            params.push(Value::Integer(account_id as i64));
        }
        conn.execute(&sql, libsql::params_from_iter(params))
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Number of rows currently suppressed by any rule — surfaced in toasts so a
/// new rule's retroactive reach is visible.
async fn suppressed_count(conn: &Connection) -> Result<i64, String> {
    let mut rows = conn
        .query("SELECT COUNT(*) FROM txn WHERE suppressed_as IS NOT NULL", ())
        .await
        .map_err(|e| e.to_string())?;
    match rows.next().await.map_err(|e| e.to_string())? {
        Some(r) => r.get::<i64>(0).map_err(|e| e.to_string()),
        None => Ok(0),
    }
}

#[tauri::command]
pub async fn list_suppress_rules_cmd(db: State<'_, Db>) -> Result<Vec<SuppressRule>, String> {
    let conn = db.conn().await?;
    list_suppress_rules(&conn).await
}

/// Creates the rule and re-derives suppression. Returns (rule, total suppressed rows).
#[tauri::command]
pub async fn create_suppress_rule_cmd(
    db: State<'_, Db>,
    keyword: String,
    kind: String,
    account_id: Option<i32>,
    created_at: String,
) -> Result<(SuppressRule, i64), String> {
    validate_kind(&kind)?;
    let conn = db.conn().await?;
    conn.execute(
        "INSERT INTO suppress_rules (keyword, kind, account_id, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![keyword.clone(), kind.clone(), account_id, created_at.clone()],
    )
    .await
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid() as i32;
    apply_suppress_rules(&conn).await?;
    let count = suppressed_count(&conn).await?;
    Ok((SuppressRule { id, keyword, kind, account_id, created_at }, count))
}

#[tauri::command]
pub async fn update_suppress_rule_cmd(
    db: State<'_, Db>,
    id: i32,
    keyword: String,
    kind: String,
    account_id: Option<i32>,
) -> Result<(), String> {
    validate_kind(&kind)?;
    let conn = db.conn().await?;
    conn.execute(
        "UPDATE suppress_rules SET keyword = ?1, kind = ?2, account_id = ?3 WHERE id = ?4",
        params![keyword, kind, account_id, id],
    )
    .await
    .map_err(|e| e.to_string())?;
    apply_suppress_rules(&conn).await
}

#[tauri::command]
pub async fn delete_suppress_rule_cmd(db: State<'_, Db>, id: i32) -> Result<(), String> {
    let conn = db.conn().await?;
    conn.execute("DELETE FROM suppress_rules WHERE id = ?1", params![id])
        .await
        .map_err(|e| e.to_string())?;
    apply_suppress_rules(&conn).await
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_db() -> Connection {
        let db = libsql::Builder::new_local(":memory:").build().await.unwrap();
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

    async fn insert_txn(conn: &Connection, account_id: i32, ty: &str, desc: &str) -> i32 {
        conn.execute(
            "INSERT INTO txn (account_id, amount, description, date, type, category, \
             is_contribution, is_withdrawal, import_source, created_at, updated_at) \
             VALUES (?1, 10.0, ?2, '2024-01-15', ?3, 'uncategorized', 0, 0, 'manual', '', '')",
            params![account_id, desc, ty],
        )
        .await
        .unwrap();
        conn.last_insert_rowid() as i32
    }

    async fn insert_rule(conn: &Connection, keyword: &str, kind: &str, account_id: Option<i32>) {
        conn.execute(
            "INSERT INTO suppress_rules (keyword, kind, account_id, created_at) VALUES (?1, ?2, ?3, '')",
            params![keyword, kind, account_id],
        )
        .await
        .unwrap();
    }

    async fn suppressed_of(conn: &Connection, id: i32) -> Option<String> {
        let mut rows = conn
            .query("SELECT suppressed_as FROM txn WHERE id = ?1", params![id])
            .await
            .unwrap();
        rows.next().await.unwrap().unwrap().get(0).unwrap()
    }

    #[tokio::test]
    async fn matches_case_insensitively_and_scopes_to_account() {
        let conn = setup_db().await;
        let fidelity = insert_account(&conn, "Fidelity 401k", "401k").await;
        let checking = insert_account(&conn, "Checking", "checking").await;

        let noise = insert_txn(&conn, fidelity, "expense", "FEES").await;
        let real = insert_txn(&conn, checking, "expense", "Bank fees").await;

        insert_rule(&conn, "fees", "fee", Some(fidelity)).await;
        apply_suppress_rules(&conn).await.unwrap();

        assert_eq!(suppressed_of(&conn, noise).await.as_deref(), Some("fee"));
        assert_eq!(suppressed_of(&conn, real).await, None, "scoped rule must not leak to other accounts");
    }

    #[tokio::test]
    async fn never_suppresses_transfers_and_rederives_on_rule_removal() {
        let conn = setup_db().await;
        let acc = insert_account(&conn, "Fidelity 401k", "401k").await;

        let gain = insert_txn(&conn, acc, "income", "Realizedgainloss").await;
        let transfer = insert_txn(&conn, acc, "transfer", "Realizedgainloss move").await;

        insert_rule(&conn, "realizedgainloss", "investment_activity", None).await;
        apply_suppress_rules(&conn).await.unwrap();
        assert_eq!(suppressed_of(&conn, gain).await.as_deref(), Some("investment_activity"));
        assert_eq!(suppressed_of(&conn, transfer).await, None);

        conn.execute("DELETE FROM suppress_rules", ()).await.unwrap();
        apply_suppress_rules(&conn).await.unwrap();
        assert_eq!(suppressed_of(&conn, gain).await, None, "removing the rule must unsuppress");
    }

    #[tokio::test]
    async fn first_matching_rule_wins_in_id_order() {
        let conn = setup_db().await;
        let acc = insert_account(&conn, "Fidelity 401k", "401k").await;
        let t = insert_txn(&conn, acc, "expense", "Advisory fees").await;

        insert_rule(&conn, "advisory", "investment_activity", None).await;
        insert_rule(&conn, "fees", "fee", None).await;
        apply_suppress_rules(&conn).await.unwrap();

        assert_eq!(suppressed_of(&conn, t).await.as_deref(), Some("investment_activity"));
    }
}
