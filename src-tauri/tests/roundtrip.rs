use libsql::Builder;
use trackmyfi_app_lib::commands::accounts::{self, NewAccount, NewBalance};
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

#[tokio::test]
async fn account_and_balance_roundtrip() {
    let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();
    migrations::run(&conn).await.unwrap();

    let id = accounts::create_account(
        &conn,
        &NewAccount {
            name: "Brokerage".into(),
            r#type: "brokerage".into(),
            institution: Some("Fidelity".into()),
            include_in_fire_calculations: true,
            created_at: "2026-01-01".into(),
        },
    )
    .await
    .unwrap();
    assert!(id >= 1);

    let list = accounts::list_accounts(&conn).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "Brokerage");
    assert_eq!(list[0].is_active, true);
    assert_eq!(list[0].include_in_fire_calculations, true);
    assert_eq!(list[0].institution.as_deref(), Some("Fidelity"));

    accounts::add_balance(
        &conn,
        &NewBalance {
            account_id: id,
            balance: 12345.67,
            recorded_at: "2026-01-01".into(),
        },
    )
    .await
    .unwrap();
    accounts::add_balance(
        &conn,
        &NewBalance {
            account_id: id,
            balance: 13000.0,
            recorded_at: "2026-02-01".into(),
        },
    )
    .await
    .unwrap();

    let bals = accounts::list_account_balances(&conn, id).await.unwrap();
    assert_eq!(bals.len(), 2);
    assert_eq!(bals[0].balance, 12345.67); // ordered by recorded_at
    let all = accounts::list_all_balances(&conn).await.unwrap();
    assert_eq!(all.len(), 2);

    accounts::archive_account(&conn, id).await.unwrap();
    let after = accounts::list_accounts(&conn).await.unwrap();
    assert_eq!(after[0].is_active, false);

    // restore (unarchive)
    accounts::unarchive_account(&conn, id).await.unwrap();
    let restored = accounts::list_accounts(&conn).await.unwrap();
    assert_eq!(restored[0].is_active, true);

    // permanent delete removes the account AND its balance snapshots
    accounts::delete_account(&conn, id).await.unwrap();
    assert_eq!(accounts::list_accounts(&conn).await.unwrap().len(), 0);
    assert_eq!(accounts::list_all_balances(&conn).await.unwrap().len(), 0);
}
