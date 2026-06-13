use libsql::Builder;
use trackmyfi_app_lib::commands::accounts::{self, NewAccount};
use trackmyfi_app_lib::commands::contributions;
use trackmyfi_app_lib::commands::transactions::{self, NewTransaction};
use trackmyfi_app_lib::migrations;

async fn setup() -> libsql::Connection {
    let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();
    migrations::run(&conn).await.unwrap();
    conn
}

async fn make_account(conn: &libsql::Connection, name: &str, ty: &str) -> i32 {
    accounts::create_account(conn, &NewAccount {
        name: name.into(),
        r#type: ty.into(),
        institution: None,
        include_in_fire_calculations: true,
        created_at: "2025-01-01".into(),
    }).await.unwrap()
}

async fn add_contribution(conn: &libsql::Connection, account_id: i32, amount: f64, date: &str) {
    transactions::create_transaction(conn, &NewTransaction {
        account_id,
        transfer_account_id: None,
        amount,
        description: "Contribution".into(),
        date: date.into(),
        r#type: "income".into(),
        category: "savings".into(),
        is_contribution: true,
        import_source: "manual".into(),
        update_balance: false,
        created_at: format!("{date}T00:00:00Z"),
    }).await.unwrap();
}

async fn add_non_contribution(conn: &libsql::Connection, account_id: i32, amount: f64, date: &str) {
    transactions::create_transaction(conn, &NewTransaction {
        account_id,
        transfer_account_id: None,
        amount,
        description: "Paycheck".into(),
        date: date.into(),
        r#type: "income".into(),
        category: "uncategorized".into(),
        is_contribution: false,
        import_source: "manual".into(),
        update_balance: false,
        created_at: format!("{date}T00:00:00Z"),
    }).await.unwrap();
}

#[tokio::test]
async fn list_contribution_txns_returns_selected_and_prior_year() {
    let conn = setup().await;
    let acct = make_account(&conn, "401k", "401k").await;
    add_contribution(&conn, acct, 1000.0, "2026-03-01").await;
    add_contribution(&conn, acct, 800.0, "2025-03-01").await;
    add_contribution(&conn, acct, 500.0, "2024-03-01").await; // outside window

    let rows = contributions::list_contribution_txns(&conn, 2026).await.unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|t| t.date.starts_with("2026") || t.date.starts_with("2025")));
    // Ordered date DESC: 2026 row precedes 2025 row.
    assert_eq!(rows[0].date, "2026-03-01");
    assert_eq!(rows[1].date, "2025-03-01");
}

#[tokio::test]
async fn list_contribution_txns_excludes_non_contributions() {
    let conn = setup().await;
    let acct = make_account(&conn, "401k", "401k").await;
    add_contribution(&conn, acct, 1000.0, "2026-03-01").await;
    add_non_contribution(&conn, acct, 9999.0, "2026-03-02").await;

    let rows = contributions::list_contribution_txns(&conn, 2026).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].amount, 1000.0);
}

#[tokio::test]
async fn list_contribution_txns_empty_when_none() {
    let conn = setup().await;
    let rows = contributions::list_contribution_txns(&conn, 2026).await.unwrap();
    assert!(rows.is_empty());
}

#[tokio::test]
async fn list_contribution_years_returns_distinct_desc() {
    let conn = setup().await;
    let acct = make_account(&conn, "401k", "401k").await;
    add_contribution(&conn, acct, 1000.0, "2026-03-01").await;
    add_contribution(&conn, acct, 800.0, "2025-03-01").await;
    add_contribution(&conn, acct, 700.0, "2025-06-01").await; // same year, deduped
    add_non_contribution(&conn, acct, 9999.0, "2020-01-01").await; // excluded

    let years = contributions::list_contribution_years(&conn).await.unwrap();
    assert_eq!(years, vec!["2026".to_string(), "2025".to_string()]);
}
