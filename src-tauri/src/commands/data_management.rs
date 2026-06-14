use crate::db::Db;
use crate::models::DeletionPreview;
use libsql::params;
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DeletionRange {
    Days { value: u32 },
    Months { value: u32 },
    All,
}

impl DeletionRange {
    fn modifier(&self) -> Option<String> {
        match self {
            DeletionRange::Days { value } => Some(format!("-{} days", value)),
            DeletionRange::Months { value } => Some(format!("-{} months", value)),
            DeletionRange::All => None,
        }
    }
}

#[tauri::command]
pub async fn preview_data_deletion(
    db: State<'_, Db>,
    range: DeletionRange,
) -> Result<DeletionPreview, String> {
    let conn = db.conn().await?;

    let (transactions, paychecks, balance_snapshots, budget_months) =
        match range.modifier() {
            Some(mod_str) => {
                let mut rows = conn
                    .query(
                        "SELECT COUNT(*) FROM txn WHERE date >= date('now', ?1)",
                        params![mod_str.clone()],
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                let transactions: i64 = rows
                    .next()
                    .await
                    .map_err(|e| e.to_string())?
                    .ok_or("no result")?
                    .get(0)
                    .map_err(|e| e.to_string())?;

                let mut rows = conn
                    .query(
                        "SELECT COUNT(*) FROM paycheck WHERE pay_date >= date('now', ?1)",
                        params![mod_str.clone()],
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                let paychecks: i64 = rows
                    .next()
                    .await
                    .map_err(|e| e.to_string())?
                    .ok_or("no result")?
                    .get(0)
                    .map_err(|e| e.to_string())?;

                let mut rows = conn
                    .query(
                        "SELECT COUNT(*) FROM account_balance WHERE date(recorded_at) >= date('now', ?1)",
                        params![mod_str.clone()],
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                let balance_snapshots: i64 = rows
                    .next()
                    .await
                    .map_err(|e| e.to_string())?
                    .ok_or("no result")?
                    .get(0)
                    .map_err(|e| e.to_string())?;

                let mut rows = conn
                    .query(
                        "SELECT COUNT(*) FROM budget_month \
                         WHERE year * 100 + month >= CAST(strftime('%Y%m', date('now', ?1)) AS INTEGER)",
                        params![mod_str],
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                let budget_months: i64 = rows
                    .next()
                    .await
                    .map_err(|e| e.to_string())?
                    .ok_or("no result")?
                    .get(0)
                    .map_err(|e| e.to_string())?;

                (transactions, paychecks, balance_snapshots, budget_months)
            }
            None => {
                let mut rows = conn
                    .query("SELECT COUNT(*) FROM txn", ())
                    .await
                    .map_err(|e| e.to_string())?;
                let transactions: i64 = rows
                    .next()
                    .await
                    .map_err(|e| e.to_string())?
                    .ok_or("no result")?
                    .get(0)
                    .map_err(|e| e.to_string())?;

                let mut rows = conn
                    .query("SELECT COUNT(*) FROM paycheck", ())
                    .await
                    .map_err(|e| e.to_string())?;
                let paychecks: i64 = rows
                    .next()
                    .await
                    .map_err(|e| e.to_string())?
                    .ok_or("no result")?
                    .get(0)
                    .map_err(|e| e.to_string())?;

                let mut rows = conn
                    .query("SELECT COUNT(*) FROM account_balance", ())
                    .await
                    .map_err(|e| e.to_string())?;
                let balance_snapshots: i64 = rows
                    .next()
                    .await
                    .map_err(|e| e.to_string())?
                    .ok_or("no result")?
                    .get(0)
                    .map_err(|e| e.to_string())?;

                let mut rows = conn
                    .query("SELECT COUNT(*) FROM budget_month", ())
                    .await
                    .map_err(|e| e.to_string())?;
                let budget_months: i64 = rows
                    .next()
                    .await
                    .map_err(|e| e.to_string())?
                    .ok_or("no result")?
                    .get(0)
                    .map_err(|e| e.to_string())?;

                (transactions, paychecks, balance_snapshots, budget_months)
            }
        };

    Ok(DeletionPreview {
        transactions,
        paychecks,
        balance_snapshots,
        budget_months,
    })
}

#[tauri::command]
pub async fn delete_data(
    db: State<'_, Db>,
    range: DeletionRange,
    reset_profile: bool,
) -> Result<(), String> {
    let conn = db.conn().await?;
    let is_all = matches!(range, DeletionRange::All);

    match range.modifier() {
        Some(mod_str) => {
            conn.execute(
                "DELETE FROM txn WHERE date >= date('now', ?1)",
                params![mod_str.clone()],
            )
            .await
            .map_err(|e| e.to_string())?;

            conn.execute(
                "DELETE FROM paycheck WHERE pay_date >= date('now', ?1)",
                params![mod_str.clone()],
            )
            .await
            .map_err(|e| e.to_string())?;

            conn.execute(
                "DELETE FROM account_balance WHERE date(recorded_at) >= date('now', ?1)",
                params![mod_str.clone()],
            )
            .await
            .map_err(|e| e.to_string())?;

            conn.execute(
                "DELETE FROM budget_month \
                 WHERE year * 100 + month >= CAST(strftime('%Y%m', date('now', ?1)) AS INTEGER)",
                params![mod_str],
            )
            .await
            .map_err(|e| e.to_string())?;
        }
        None => {
            conn.execute("DELETE FROM txn", ()).await.map_err(|e| e.to_string())?;
            conn.execute("DELETE FROM paycheck", ()).await.map_err(|e| e.to_string())?;
            conn.execute("DELETE FROM account_balance", ()).await.map_err(|e| e.to_string())?;
            conn.execute("DELETE FROM budget_month", ()).await.map_err(|e| e.to_string())?;
        }
    }

    if is_all {
        if reset_profile {
            conn.execute(
                "UPDATE fire_profile SET \
                 current_age = 30, target_retirement_age = 50, annual_expenses_target = 40000, \
                 lean_fire_annual_expenses = NULL, fat_fire_annual_expenses = NULL, \
                 annual_income = 80000, expected_return_rate = 0.07, inflation_rate = 0.03, \
                 hsa_coverage = 'self', onboarding_completed = 0 WHERE id = 1",
                (),
            )
            .await
            .map_err(|e| e.to_string())?;
        } else {
            conn.execute(
                "UPDATE fire_profile SET onboarding_completed = 0 WHERE id = 1",
                (),
            )
            .await
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}
