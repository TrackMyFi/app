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
