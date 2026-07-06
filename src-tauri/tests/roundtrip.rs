use libsql::Builder;
use trackmyfi_app_lib::commands::accounts::{self, NewAccount, NewBalance, UpdateBalance};
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
    assert_eq!(p.date_of_birth, None);
    assert_eq!(p.annual_expenses_target, 40000.0);
    assert_eq!(p.lean_fire_annual_expenses, None);
    assert_eq!(p.hsa_coverage, "self");
    assert_eq!(p.withdrawal_rate, 0.04); // migration default

    // update and read back
    let updated = FireProfile {
        date_of_birth: Some("1990-06-15".into()),
        target_retirement_age: 55,
        annual_expenses_target: 50000.0,
        lean_fire_annual_expenses: Some(35000.0),
        fat_fire_annual_expenses: None,
        annual_income: 90000.0,
        expected_return_rate: 0.06,
        inflation_rate: 0.025,
        withdrawal_rate: 0.035,
        hsa_coverage: "family".into(),
        onboarding_completed: false,
    };
    upsert_profile(&conn, &updated).await.unwrap();
    let p2 = get_profile(&conn).await.unwrap();
    assert_eq!(p2.date_of_birth, Some("1990-06-15".into()));
    assert_eq!(p2.annual_expenses_target, 50000.0);
    assert_eq!(p2.lean_fire_annual_expenses, Some(35000.0));
    assert_eq!(p2.fat_fire_annual_expenses, None);
    assert_eq!(p2.hsa_coverage, "family");
    assert_eq!(p2.withdrawal_rate, 0.035);
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
            include_in_fire_calculations: true, count_payments_as_expense: false,
            created_at: "2026-01-01".into(), traditional_pct: None,
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

    // edit account: editable fields change, is_active is preserved
    accounts::update_account(
        &conn,
        id,
        &NewAccount {
            name: "Brokerage (edited)".into(),
            r#type: "roth_ira".into(),
            institution: None,
            include_in_fire_calculations: false, count_payments_as_expense: false,
            created_at: "2025-12-15".into(), traditional_pct: None,
        },
    )
    .await
    .unwrap();
    let edited = accounts::list_accounts(&conn).await.unwrap();
    assert_eq!(edited[0].name, "Brokerage (edited)");
    assert_eq!(edited[0].r#type, "roth_ira");
    assert_eq!(edited[0].institution, None);
    assert_eq!(edited[0].include_in_fire_calculations, false);
    assert_eq!(edited[0].created_at, "2025-12-15");
    assert_eq!(edited[0].is_active, true); // unchanged by edit

    // edit a single balance snapshot
    let bals_before = accounts::list_account_balances(&conn, id).await.unwrap();
    let target = bals_before[0].id; // the 2026-01-01 / 12345.67 row
    accounts::update_balance(
        &conn,
        &UpdateBalance {
            id: target,
            balance: 99999.99,
            recorded_at: "2026-01-15".into(),
        },
    )
    .await
    .unwrap();
    let bals_after = accounts::list_account_balances(&conn, id).await.unwrap();
    let edited_bal = bals_after.iter().find(|b| b.id == target).unwrap();
    assert_eq!(edited_bal.balance, 99999.99);
    assert_eq!(edited_bal.recorded_at, "2026-01-15");
    assert_eq!(bals_after.len(), 2); // still two rows

    // delete one snapshot: target gone, sibling intact
    accounts::delete_balance(&conn, target).await.unwrap();
    let bals_final = accounts::list_account_balances(&conn, id).await.unwrap();
    assert_eq!(bals_final.len(), 1);
    assert!(bals_final.iter().all(|b| b.id != target));

    // permanent delete removes the account AND its balance snapshots
    accounts::delete_account(&conn, id).await.unwrap();
    assert_eq!(accounts::list_accounts(&conn).await.unwrap().len(), 0);
    assert_eq!(accounts::list_all_balances(&conn).await.unwrap().len(), 0);
}
