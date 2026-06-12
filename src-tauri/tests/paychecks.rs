use libsql::Builder;
use trackmyfi_app_lib::commands::accounts::{self, NewAccount};
use trackmyfi_app_lib::commands::paychecks::{self, NewPaycheck, PaycheckFilter};
use trackmyfi_app_lib::models::{PaycheckDeduction, EmployerMatchItem};
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
        created_at: "2026-01-01".into(),
    }).await.unwrap()
}

fn base_paycheck(employer: &str, pay_date: &str) -> NewPaycheck {
    NewPaycheck {
        pay_date: pay_date.into(),
        employer: employer.into(),
        pay_period: "biweekly".into(),
        gross_amount: 5000.0,
        net_amount: 3200.0,
        federal_tax: 800.0,
        state_tax: 250.0,
        local_tax: 0.0,
        social_security_tax: 310.0,
        medicare_tax: 72.5,
        deductions: vec![],
        employer_match: vec![],
        created_at: "2026-06-01T00:00:00Z".into(),
    }
}

async fn txn_count(conn: &libsql::Connection) -> i64 {
    let mut rows = conn.query("SELECT COUNT(*) FROM txn", ()).await.unwrap();
    rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap()
}

async fn contribution_count_for(conn: &libsql::Connection, paycheck_id: i32) -> i64 {
    let mut rows = conn.query(
        "SELECT COUNT(*) FROM txn WHERE paycheck_id = ?1 AND is_contribution = 1",
        libsql::params![paycheck_id],
    ).await.unwrap();
    rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap()
}

#[tokio::test]
async fn create_and_get_paycheck() {
    let conn = setup().await;
    let p = paychecks::create_paycheck(&conn, &base_paycheck("Acme", "2026-06-15")).await.unwrap();
    assert!(p.id >= 1);
    assert_eq!(p.employer, "Acme");
    assert_eq!(p.gross_amount, 5000.0);
    assert_eq!(p.deductions.len(), 0);
    assert_eq!(p.employer_match.len(), 0);

    let fetched = paychecks::get_paycheck(&conn, p.id).await.unwrap();
    assert_eq!(fetched.id, p.id);
    assert_eq!(fetched.pay_date, "2026-06-15");
}

#[tokio::test]
async fn create_with_contribution_deduction_creates_txn() {
    let conn = setup().await;
    let acct = make_account(&conn, "Fidelity 401k", "401k").await;

    let mut p = base_paycheck("Acme", "2026-06-15");
    p.deductions = vec![
        PaycheckDeduction {
            label: "401k".into(),
            amount: 750.0,
            pre_tax: true,
            contribution_account_type: Some("401k".into()),
            account_id: Some(acct),
        },
    ];

    let created = paychecks::create_paycheck(&conn, &p).await.unwrap();
    assert_eq!(contribution_count_for(&conn, created.id).await, 1);

    // Verify the contribution txn has correct fields
    let mut rows = conn.query(
        "SELECT amount, date, type, category, is_contribution, import_source, account_id \
         FROM txn WHERE paycheck_id = ?1",
        libsql::params![created.id],
    ).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<f64>(0).unwrap(), 750.0);
    assert_eq!(row.get::<String>(1).unwrap(), "2026-06-15");
    assert_eq!(row.get::<String>(2).unwrap(), "income");
    assert_eq!(row.get::<String>(3).unwrap(), "savings");
    assert_eq!(row.get::<i64>(4).unwrap(), 1);
    assert_eq!(row.get::<String>(5).unwrap(), "paycheck");
    assert_eq!(row.get::<i32>(6).unwrap(), acct);
}

#[tokio::test]
async fn deduction_without_account_id_creates_no_txn() {
    let conn = setup().await;
    let mut p = base_paycheck("Acme", "2026-06-15");
    p.deductions = vec![
        PaycheckDeduction {
            label: "401k".into(),
            amount: 750.0,
            pre_tax: true,
            contribution_account_type: Some("401k".into()),
            account_id: None,
        },
    ];
    let created = paychecks::create_paycheck(&conn, &p).await.unwrap();
    assert_eq!(txn_count(&conn).await, 0);
    assert_eq!(contribution_count_for(&conn, created.id).await, 0);
}

#[tokio::test]
async fn employer_match_with_account_creates_txn() {
    let conn = setup().await;
    let acct = make_account(&conn, "Fidelity 401k", "401k").await;

    let mut p = base_paycheck("Acme", "2026-06-15");
    p.employer_match = vec![
        EmployerMatchItem { label: "401k Match".into(), amount: 375.0, account_id: Some(acct) },
    ];

    let created = paychecks::create_paycheck(&conn, &p).await.unwrap();
    assert_eq!(contribution_count_for(&conn, created.id).await, 1);
}

#[tokio::test]
async fn list_paychecks_date_filter() {
    let conn = setup().await;
    paychecks::create_paycheck(&conn, &base_paycheck("Acme", "2026-01-15")).await.unwrap();
    paychecks::create_paycheck(&conn, &base_paycheck("Acme", "2026-06-15")).await.unwrap();

    let all = paychecks::list_paychecks(&conn, &PaycheckFilter::default()).await.unwrap();
    assert_eq!(all.len(), 2);

    let filtered = paychecks::list_paychecks(&conn, &PaycheckFilter {
        start_date: Some("2026-06-01".into()),
        ..Default::default()
    }).await.unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].pay_date, "2026-06-15");
}

#[tokio::test]
async fn list_paychecks_end_date_filter() {
    let conn = setup().await;
    paychecks::create_paycheck(&conn, &base_paycheck("Acme", "2026-01-15")).await.unwrap();
    paychecks::create_paycheck(&conn, &base_paycheck("Acme", "2026-06-15")).await.unwrap();

    let filtered = paychecks::list_paychecks(&conn, &PaycheckFilter {
        end_date: Some("2026-03-01".into()),
        ..Default::default()
    }).await.unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].pay_date, "2026-01-15");
}

#[tokio::test]
async fn get_paycheck_missing_id_errors() {
    let conn = setup().await;
    assert!(paychecks::get_paycheck(&conn, 9999).await.is_err());
}
