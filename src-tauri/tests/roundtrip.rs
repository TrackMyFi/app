use libsql::Builder;
use trackmyfi_app_lib::commands::fire_profile::{get_profile, upsert_profile};
use trackmyfi_app_lib::migrations;
use trackmyfi_app_lib::models::FireProfile;

#[tokio::test]
async fn fire_profile_roundtrip() {
    let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();
    migrations::run(&conn).await.unwrap();

    // seeded defaults
    let p = get_profile(&conn).await.unwrap();
    assert_eq!(p.current_age, 30);
    assert_eq!(p.annual_expenses_target, 40000.0);
    assert_eq!(p.lean_fire_annual_expenses, None);

    // update and read back
    let updated = FireProfile {
        current_age: 35,
        target_retirement_age: 55,
        annual_expenses_target: 50000.0,
        lean_fire_annual_expenses: Some(35000.0),
        fat_fire_annual_expenses: None,
        annual_income: 90000.0,
        expected_return_rate: 0.06,
        inflation_rate: 0.025,
    };
    upsert_profile(&conn, &updated).await.unwrap();
    let p2 = get_profile(&conn).await.unwrap();
    assert_eq!(p2.current_age, 35);
    assert_eq!(p2.annual_expenses_target, 50000.0);
    assert_eq!(p2.lean_fire_annual_expenses, Some(35000.0));
    assert_eq!(p2.fat_fire_annual_expenses, None);
}
