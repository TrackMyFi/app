use crate::db::Db;
use crate::models::FireProfile;
use libsql::Connection;
use tauri::State;

pub async fn get_profile(conn: &Connection) -> Result<FireProfile, String> {
    let mut rows = conn
        .query(
            "SELECT current_age, target_retirement_age, annual_expenses_target, \
             lean_fire_annual_expenses, fat_fire_annual_expenses, annual_income, \
             expected_return_rate, inflation_rate FROM fire_profile WHERE id = 1",
            (),
        )
        .await
        .map_err(|e| e.to_string())?;
    let row = rows
        .next()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "fire_profile row missing".to_string())?;
    Ok(FireProfile {
        current_age: row.get(0).map_err(|e| e.to_string())?,
        target_retirement_age: row.get(1).map_err(|e| e.to_string())?,
        annual_expenses_target: row.get(2).map_err(|e| e.to_string())?,
        lean_fire_annual_expenses: row.get(3).map_err(|e| e.to_string())?,
        fat_fire_annual_expenses: row.get(4).map_err(|e| e.to_string())?,
        annual_income: row.get(5).map_err(|e| e.to_string())?,
        expected_return_rate: row.get(6).map_err(|e| e.to_string())?,
        inflation_rate: row.get(7).map_err(|e| e.to_string())?,
    })
}

pub async fn upsert_profile(conn: &Connection, p: &FireProfile) -> Result<(), String> {
    conn.execute(
        "UPDATE fire_profile SET current_age=?1, target_retirement_age=?2, \
         annual_expenses_target=?3, lean_fire_annual_expenses=?4, fat_fire_annual_expenses=?5, \
         annual_income=?6, expected_return_rate=?7, inflation_rate=?8 WHERE id = 1",
        libsql::params![
            p.current_age,
            p.target_retirement_age,
            p.annual_expenses_target,
            p.lean_fire_annual_expenses,
            p.fat_fire_annual_expenses,
            p.annual_income,
            p.expected_return_rate,
            p.inflation_rate
        ],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_fire_profile(db: State<'_, Db>) -> Result<FireProfile, String> {
    let conn = db.conn().await?;
    get_profile(&conn).await
}

#[tauri::command]
pub async fn upsert_fire_profile(
    db: State<'_, Db>,
    profile: FireProfile,
) -> Result<(), String> {
    let conn = db.conn().await?;
    upsert_profile(&conn, &profile).await
}
