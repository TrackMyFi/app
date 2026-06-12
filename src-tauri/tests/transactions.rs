use libsql::Builder;
use trackmyfi_app_lib::commands::accounts::{self, NewAccount};
use trackmyfi_app_lib::commands::transactions::{
    self, NewTransaction, TransactionFilter, UpdateTransaction,
};
use trackmyfi_app_lib::migrations;

async fn setup() -> libsql::Connection {
    let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();
    migrations::run(&conn).await.unwrap();
    conn
}

fn new_txn(account_id: i32, amount: f64, ty: &str) -> NewTransaction {
    NewTransaction {
        account_id,
        transfer_account_id: None,
        amount,
        description: "test".into(),
        date: "2026-03-01".into(),
        r#type: ty.into(),
        category: "uncategorized".into(),
        is_contribution: false,
        import_source: "manual".into(),
        update_balance: false,
        created_at: "2026-03-01".into(),
    }
}

#[tokio::test]
async fn transaction_crud_and_totals() {
    let conn = setup().await;
    let acct = accounts::create_account(
        &conn,
        &NewAccount {
            name: "Checking".into(),
            r#type: "checking".into(),
            institution: None,
            include_in_fire_calculations: false,
            created_at: "2026-01-01".into(),
        },
    )
    .await
    .unwrap();

    let id = transactions::create_transaction(&conn, &new_txn(acct, 1000.0, "income"))
        .await
        .unwrap();
    assert!(id >= 1);
    transactions::create_transaction(&conn, &new_txn(acct, 40.0, "expense"))
        .await
        .unwrap();
    transactions::create_transaction(&conn, &new_txn(acct, 60.0, "expense"))
        .await
        .unwrap();

    let page = transactions::list_transactions(&conn, &TransactionFilter::default())
        .await
        .unwrap();
    assert_eq!(page.rows.len(), 3);
    assert_eq!(page.total_count, 3);
    assert_eq!(page.total_income, 1000.0);
    assert_eq!(page.total_expense, 100.0);
    assert_eq!(page.net, 900.0);

    // filter by type
    let only_expense = transactions::list_transactions(
        &conn,
        &TransactionFilter { r#type: Some("expense".into()), ..Default::default() },
    )
    .await
    .unwrap();
    assert_eq!(only_expense.rows.len(), 2);

    // update one
    transactions::update_transaction(
        &conn,
        &UpdateTransaction {
            id,
            account_id: acct,
            transfer_account_id: None,
            amount: 1200.0,
            description: "raise".into(),
            date: "2026-03-02".into(),
            r#type: "income".into(),
            category: "savings".into(),
            is_contribution: false,
            update_balance: false,
            updated_at: "2026-03-02".into(),
        },
    )
    .await
    .unwrap();
    let after = transactions::list_transactions(&conn, &TransactionFilter::default())
        .await
        .unwrap();
    assert_eq!(after.total_income, 1200.0);

    // delete one expense
    let expense_id = only_expense.rows[0].id;
    transactions::delete_transaction(&conn, expense_id).await.unwrap();
    let final_page = transactions::list_transactions(&conn, &TransactionFilter::default())
        .await
        .unwrap();
    assert_eq!(final_page.rows.len(), 2);
}

#[tokio::test]
async fn transfers_excluded_from_totals() {
    let conn = setup().await;
    let a = accounts::create_account(&conn, &NewAccount {
        name: "A".into(), r#type: "checking".into(), institution: None,
        include_in_fire_calculations: false, created_at: "2026-01-01".into() }).await.unwrap();
    let b = accounts::create_account(&conn, &NewAccount {
        name: "B".into(), r#type: "savings".into(), institution: None,
        include_in_fire_calculations: false, created_at: "2026-01-01".into() }).await.unwrap();

    let mut t = new_txn(a, 500.0, "transfer");
    t.transfer_account_id = Some(b);
    transactions::create_transaction(&conn, &t).await.unwrap();

    let page = transactions::list_transactions(&conn, &TransactionFilter::default()).await.unwrap();
    assert_eq!(page.rows.len(), 1);
    assert_eq!(page.total_income, 0.0);
    assert_eq!(page.total_expense, 0.0);
    assert_eq!(page.net, 0.0);

    // filtering by either side returns the transfer
    let by_dest = transactions::list_transactions(&conn,
        &TransactionFilter { account_id: Some(b), ..Default::default() }).await.unwrap();
    assert_eq!(by_dest.rows.len(), 1);
}

async fn latest_balance(conn: &libsql::Connection, account_id: i32) -> f64 {
    let mut rows = conn
        .query(
            "SELECT balance FROM account_balance WHERE account_id = ?1 \
             ORDER BY recorded_at DESC, id DESC LIMIT 1",
            libsql::params![account_id],
        )
        .await
        .unwrap();
    match rows.next().await.unwrap() {
        Some(r) => r.get::<f64>(0).unwrap(),
        None => 0.0,
    }
}

async fn balance_count(conn: &libsql::Connection) -> i64 {
    let mut rows = conn.query("SELECT COUNT(*) FROM account_balance", ()).await.unwrap();
    rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap()
}

#[tokio::test]
async fn balance_switch_creates_and_links_snapshot() {
    let conn = setup().await;
    let acct = accounts::create_account(&conn, &NewAccount {
        name: "Checking".into(), r#type: "checking".into(), institution: None,
        include_in_fire_calculations: false, created_at: "2026-01-01".into() }).await.unwrap();
    accounts::add_balance(&conn, &trackmyfi_app_lib::commands::accounts::NewBalance {
        account_id: acct, balance: 1000.0, recorded_at: "2026-02-01".into() }).await.unwrap();

    // expense of 40 with switch ON → new snapshot 960
    let mut t = new_txn(acct, 40.0, "expense");
    t.update_balance = true;
    let id = transactions::create_transaction(&conn, &t).await.unwrap();
    assert_eq!(latest_balance(&conn, acct).await, 960.0);

    let page = transactions::list_transactions(&conn, &TransactionFilter::default()).await.unwrap();
    assert!(page.rows[0].generated_balance_id.is_some());

    // delete the transaction → its generated snapshot is removed (back to 1000)
    transactions::delete_transaction(&conn, id).await.unwrap();
    assert_eq!(latest_balance(&conn, acct).await, 1000.0);
}

#[tokio::test]
async fn balance_switch_off_writes_no_snapshot() {
    let conn = setup().await;
    let acct = accounts::create_account(&conn, &NewAccount {
        name: "Checking".into(), r#type: "checking".into(), institution: None,
        include_in_fire_calculations: false, created_at: "2026-01-01".into() }).await.unwrap();
    let t = new_txn(acct, 40.0, "expense"); // update_balance defaults false
    transactions::create_transaction(&conn, &t).await.unwrap();
    assert_eq!(balance_count(&conn).await, 0);
}

#[tokio::test]
async fn transfer_switch_writes_two_snapshots() {
    let conn = setup().await;
    let a = accounts::create_account(&conn, &NewAccount {
        name: "A".into(), r#type: "checking".into(), institution: None,
        include_in_fire_calculations: false, created_at: "2026-01-01".into() }).await.unwrap();
    let b = accounts::create_account(&conn, &NewAccount {
        name: "B".into(), r#type: "savings".into(), institution: None,
        include_in_fire_calculations: false, created_at: "2026-01-01".into() }).await.unwrap();
    accounts::add_balance(&conn, &trackmyfi_app_lib::commands::accounts::NewBalance {
        account_id: a, balance: 1000.0, recorded_at: "2026-02-01".into() }).await.unwrap();
    accounts::add_balance(&conn, &trackmyfi_app_lib::commands::accounts::NewBalance {
        account_id: b, balance: 200.0, recorded_at: "2026-02-01".into() }).await.unwrap();

    let mut t = new_txn(a, 300.0, "transfer");
    t.transfer_account_id = Some(b);
    t.update_balance = true;
    transactions::create_transaction(&conn, &t).await.unwrap();

    assert_eq!(latest_balance(&conn, a).await, 700.0);  // 1000 - 300
    assert_eq!(latest_balance(&conn, b).await, 500.0);  // 200 + 300
}

#[tokio::test]
async fn editing_amount_reapplies_linked_snapshot() {
    let conn = setup().await;
    let acct = accounts::create_account(&conn, &NewAccount {
        name: "Checking".into(), r#type: "checking".into(), institution: None,
        include_in_fire_calculations: false, created_at: "2026-01-01".into() }).await.unwrap();
    accounts::add_balance(&conn, &trackmyfi_app_lib::commands::accounts::NewBalance {
        account_id: acct, balance: 1000.0, recorded_at: "2026-02-01".into() }).await.unwrap();

    let mut t = new_txn(acct, 40.0, "expense");
    t.update_balance = true;
    let id = transactions::create_transaction(&conn, &t).await.unwrap();
    assert_eq!(latest_balance(&conn, acct).await, 960.0);

    transactions::update_transaction(&conn, &UpdateTransaction {
        id, account_id: acct, transfer_account_id: None, amount: 100.0,
        description: "test".into(), date: "2026-03-01".into(), r#type: "expense".into(),
        category: "uncategorized".into(), is_contribution: false,
        update_balance: true, updated_at: "2026-03-02".into() }).await.unwrap();
    assert_eq!(latest_balance(&conn, acct).await, 900.0); // re-applied: 1000 - 100
    assert_eq!(balance_count(&conn).await, 2); // original seed + one generated (not stacked)
}
