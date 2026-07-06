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
        is_withdrawal: false,
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
            include_in_fire_calculations: false, count_payments_as_expense: false,
            created_at: "2026-01-01".into(), traditional_pct: None,
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
        &TransactionFilter { types: vec!["expense".into()], ..Default::default() },
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
            is_withdrawal: false,
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
        include_in_fire_calculations: false, count_payments_as_expense: false, created_at: "2026-01-01".into(), traditional_pct: None, }).await.unwrap();
    let b = accounts::create_account(&conn, &NewAccount {
        name: "B".into(), r#type: "savings".into(), institution: None,
        include_in_fire_calculations: false, count_payments_as_expense: false, created_at: "2026-01-01".into(), traditional_pct: None, }).await.unwrap();

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
        &TransactionFilter { account_ids: vec![b], ..Default::default() }).await.unwrap();
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
        include_in_fire_calculations: false, count_payments_as_expense: false, created_at: "2026-01-01".into(), traditional_pct: None, }).await.unwrap();
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
        include_in_fire_calculations: false, count_payments_as_expense: false, created_at: "2026-01-01".into(), traditional_pct: None, }).await.unwrap();
    let t = new_txn(acct, 40.0, "expense"); // update_balance defaults false
    transactions::create_transaction(&conn, &t).await.unwrap();
    assert_eq!(balance_count(&conn).await, 0);
}

#[tokio::test]
async fn transfer_switch_writes_two_snapshots() {
    let conn = setup().await;
    let a = accounts::create_account(&conn, &NewAccount {
        name: "A".into(), r#type: "checking".into(), institution: None,
        include_in_fire_calculations: false, count_payments_as_expense: false, created_at: "2026-01-01".into(), traditional_pct: None, }).await.unwrap();
    let b = accounts::create_account(&conn, &NewAccount {
        name: "B".into(), r#type: "savings".into(), institution: None,
        include_in_fire_calculations: false, count_payments_as_expense: false, created_at: "2026-01-01".into(), traditional_pct: None, }).await.unwrap();
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
async fn transfer_into_liability_reduces_its_debt() {
    // A credit-card payment: money flows from a checking account (source) into a
    // liability card (destination). The liability balance stores debt owed, so
    // receiving a payment must DECREASE it, while the asset source decreases too.
    let conn = setup().await;
    let pnc = accounts::create_account(&conn, &NewAccount {
        name: "PNC".into(), r#type: "checking".into(), institution: None,
        include_in_fire_calculations: false, count_payments_as_expense: false, created_at: "2026-01-01".into(), traditional_pct: None, }).await.unwrap();
    let card = accounts::create_account(&conn, &NewAccount {
        name: "Citi".into(), r#type: "liability".into(), institution: None,
        include_in_fire_calculations: false, count_payments_as_expense: false, created_at: "2026-01-01".into(), traditional_pct: None, }).await.unwrap();
    accounts::add_balance(&conn, &trackmyfi_app_lib::commands::accounts::NewBalance {
        account_id: pnc, balance: 1000.0, recorded_at: "2026-02-01".into() }).await.unwrap();
    accounts::add_balance(&conn, &trackmyfi_app_lib::commands::accounts::NewBalance {
        account_id: card, balance: 500.0, recorded_at: "2026-02-01".into() }).await.unwrap();

    // source = PNC, destination = Citi card (after the import-layer swap)
    let mut t = new_txn(pnc, 300.0, "transfer");
    t.transfer_account_id = Some(card);
    t.update_balance = true;
    transactions::create_transaction(&conn, &t).await.unwrap();

    assert_eq!(latest_balance(&conn, pnc).await, 700.0);  // asset source: 1000 - 300
    assert_eq!(latest_balance(&conn, card).await, 200.0); // liability dest: 500 - 300 (less debt)
}

#[tokio::test]
async fn income_and_expense_on_liability_move_debt_correctly() {
    // On a credit card (liability, balance = debt owed), a purchase (expense) raises
    // what you owe and a refund/redemption (income) lowers it — the opposite of an asset.
    let conn = setup().await;
    let card = accounts::create_account(&conn, &NewAccount {
        name: "Citi".into(), r#type: "liability".into(), institution: None,
        include_in_fire_calculations: false, count_payments_as_expense: false, created_at: "2026-01-01".into(), traditional_pct: None, }).await.unwrap();
    accounts::add_balance(&conn, &trackmyfi_app_lib::commands::accounts::NewBalance {
        account_id: card, balance: 500.0, recorded_at: "2026-02-01".into() }).await.unwrap();

    // expense (purchase) of 40 → debt rises to 540
    let mut purchase = new_txn(card, 40.0, "expense");
    purchase.date = "2026-02-02".into();
    purchase.update_balance = true;
    transactions::create_transaction(&conn, &purchase).await.unwrap();
    assert_eq!(latest_balance(&conn, card).await, 540.0);

    // income (refund) of 100 → debt falls to 440
    let mut refund = new_txn(card, 100.0, "income");
    refund.date = "2026-02-03".into();
    refund.update_balance = true;
    transactions::create_transaction(&conn, &refund).await.unwrap();
    assert_eq!(latest_balance(&conn, card).await, 440.0);
}

#[tokio::test]
async fn transfer_out_of_liability_increases_its_debt() {
    // A cash advance: money flows out of the liability card (source) into checking.
    // Drawing against the card increases debt owed; the asset destination rises.
    let conn = setup().await;
    let card = accounts::create_account(&conn, &NewAccount {
        name: "Citi".into(), r#type: "liability".into(), institution: None,
        include_in_fire_calculations: false, count_payments_as_expense: false, created_at: "2026-01-01".into(), traditional_pct: None, }).await.unwrap();
    let pnc = accounts::create_account(&conn, &NewAccount {
        name: "PNC".into(), r#type: "checking".into(), institution: None,
        include_in_fire_calculations: false, count_payments_as_expense: false, created_at: "2026-01-01".into(), traditional_pct: None, }).await.unwrap();
    accounts::add_balance(&conn, &trackmyfi_app_lib::commands::accounts::NewBalance {
        account_id: card, balance: 500.0, recorded_at: "2026-02-01".into() }).await.unwrap();
    accounts::add_balance(&conn, &trackmyfi_app_lib::commands::accounts::NewBalance {
        account_id: pnc, balance: 1000.0, recorded_at: "2026-02-01".into() }).await.unwrap();

    let mut t = new_txn(card, 300.0, "transfer");
    t.transfer_account_id = Some(pnc);
    t.update_balance = true;
    transactions::create_transaction(&conn, &t).await.unwrap();

    assert_eq!(latest_balance(&conn, card).await, 800.0); // liability source: 500 + 300 (more debt)
    assert_eq!(latest_balance(&conn, pnc).await, 1300.0); // asset dest: 1000 + 300
}

#[tokio::test]
async fn editing_amount_reapplies_linked_snapshot() {
    let conn = setup().await;
    let acct = accounts::create_account(&conn, &NewAccount {
        name: "Checking".into(), r#type: "checking".into(), institution: None,
        include_in_fire_calculations: false, count_payments_as_expense: false, created_at: "2026-01-01".into(), traditional_pct: None, }).await.unwrap();
    accounts::add_balance(&conn, &trackmyfi_app_lib::commands::accounts::NewBalance {
        account_id: acct, balance: 1000.0, recorded_at: "2026-02-01".into() }).await.unwrap();

    let mut t = new_txn(acct, 40.0, "expense");
    t.update_balance = true;
    let id = transactions::create_transaction(&conn, &t).await.unwrap();
    assert_eq!(latest_balance(&conn, acct).await, 960.0);

    transactions::update_transaction(&conn, &UpdateTransaction {
        id, account_id: acct, transfer_account_id: None, amount: 100.0,
        description: "test".into(), date: "2026-03-01".into(), r#type: "expense".into(),
        category: "uncategorized".into(), is_contribution: false, is_withdrawal: false,
        update_balance: true, updated_at: "2026-03-02".into() }).await.unwrap();
    assert_eq!(latest_balance(&conn, acct).await, 900.0); // re-applied: 1000 - 100
    assert_eq!(balance_count(&conn).await, 2); // original seed + one generated (not stacked)
}

#[tokio::test]
async fn bulk_create_writes_no_snapshots() {
    let conn = setup().await;
    let acct = accounts::create_account(&conn, &NewAccount {
        name: "Checking".into(), r#type: "checking".into(), institution: None,
        include_in_fire_calculations: false, count_payments_as_expense: false, created_at: "2026-01-01".into(), traditional_pct: None, }).await.unwrap();

    let rows = vec![
        new_txn(acct, 40.0, "expense"),
        new_txn(acct, 1500.0, "income"),
    ];
    let n = transactions::bulk_create_transactions(&conn, &rows).await.unwrap();
    assert_eq!(n, 2);

    let page = transactions::list_transactions(&conn, &TransactionFilter::default()).await.unwrap();
    assert_eq!(page.rows.len(), 2);
    assert_eq!(balance_count(&conn).await, 0); // never writes snapshots
    assert!(page.rows.iter().all(|r| r.import_source == "csv"));
}

#[tokio::test]
async fn balances_expose_linked_transaction_id() {
    let conn = setup().await;
    let acct = accounts::create_account(&conn, &NewAccount {
        name: "Checking".into(), r#type: "checking".into(), institution: None,
        include_in_fire_calculations: false, count_payments_as_expense: false, created_at: "2026-01-01".into(), traditional_pct: None, }).await.unwrap();

    // A manually-entered balance has no linking transaction.
    accounts::add_balance(&conn, &accounts::NewBalance {
        account_id: acct, balance: 1000.0, recorded_at: "2026-02-01".into() }).await.unwrap();

    // A transaction with the balance switch ON generates a linked snapshot (1000 - 40 = 960).
    let mut t = new_txn(acct, 40.0, "expense");
    t.update_balance = true;
    let txn_id = transactions::create_transaction(&conn, &t).await.unwrap();

    let balances = accounts::list_all_balances(&conn).await.unwrap();
    let manual = balances.iter().find(|b| b.balance == 1000.0).unwrap();
    let generated = balances.iter().find(|b| b.balance == 960.0).unwrap();
    assert_eq!(manual.linked_transaction_id, None);
    assert_eq!(generated.linked_transaction_id, Some(txn_id));
}

#[tokio::test]
async fn transfer_balances_link_to_same_transaction() {
    let conn = setup().await;
    let a = accounts::create_account(&conn, &NewAccount {
        name: "A".into(), r#type: "checking".into(), institution: None,
        include_in_fire_calculations: false, count_payments_as_expense: false, created_at: "2026-01-01".into(), traditional_pct: None, }).await.unwrap();
    let b = accounts::create_account(&conn, &NewAccount {
        name: "B".into(), r#type: "savings".into(), institution: None,
        include_in_fire_calculations: false, count_payments_as_expense: false, created_at: "2026-01-01".into(), traditional_pct: None, }).await.unwrap();
    accounts::add_balance(&conn, &accounts::NewBalance {
        account_id: a, balance: 1000.0, recorded_at: "2026-02-01".into() }).await.unwrap();
    accounts::add_balance(&conn, &accounts::NewBalance {
        account_id: b, balance: 200.0, recorded_at: "2026-02-01".into() }).await.unwrap();

    let mut t = new_txn(a, 300.0, "transfer");
    t.transfer_account_id = Some(b);
    t.update_balance = true;
    let txn_id = transactions::create_transaction(&conn, &t).await.unwrap();

    let balances = accounts::list_all_balances(&conn).await.unwrap();
    // Source snapshot: 1000 - 300 = 700; destination: 200 + 300 = 500.
    let src = balances.iter().find(|x| x.account_id == a && x.balance == 700.0).unwrap();
    let dst = balances.iter().find(|x| x.account_id == b && x.balance == 500.0).unwrap();
    assert_eq!(src.linked_transaction_id, Some(txn_id));
    assert_eq!(dst.linked_transaction_id, Some(txn_id));
}

#[tokio::test]
async fn get_transaction_returns_row() {
    let conn = setup().await;
    let acct = accounts::create_account(&conn, &NewAccount {
        name: "Checking".into(), r#type: "checking".into(), institution: None,
        include_in_fire_calculations: false, count_payments_as_expense: false, created_at: "2026-01-01".into(), traditional_pct: None, }).await.unwrap();
    let id = transactions::create_transaction(&conn, &new_txn(acct, 1000.0, "income"))
        .await.unwrap();

    let txn = transactions::get_transaction(&conn, id).await.unwrap();
    assert_eq!(txn.id, id);
    assert_eq!(txn.amount, 1000.0);
    assert_eq!(txn.r#type, "income");
}

#[tokio::test]
async fn get_transaction_missing_id_errors() {
    let conn = setup().await;
    let result = transactions::get_transaction(&conn, 9999).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn bulk_create_with_snapshots_sequential_balances() {
    let conn = setup().await;
    let acct = accounts::create_account(&conn, &NewAccount {
        name: "Checking".into(), r#type: "checking".into(), institution: None,
        include_in_fire_calculations: false, count_payments_as_expense: false, created_at: "2026-01-01".into(), traditional_pct: None,
    }).await.unwrap();

    // Seed balance: $1,000 on Jan 1 — gives base_balance something to start from.
    accounts::add_balance(&conn, &accounts::NewBalance {
        account_id: acct, balance: 1000.0, recorded_at: "2026-01-01".into(),
    }).await.unwrap();

    // Three rows in ascending date order, as the frontend sorts before calling.
    let rows = vec![
        NewTransaction {
            account_id: acct, transfer_account_id: None,
            amount: 100.0, description: "expense 1".into(),
            date: "2026-01-02".into(), r#type: "expense".into(),
            category: "uncategorized".into(), is_contribution: false,
            is_withdrawal: false,
            import_source: "csv".into(), update_balance: true,
            created_at: "2026-01-02".into(),
        },
        NewTransaction {
            account_id: acct, transfer_account_id: None,
            amount: 200.0, description: "expense 2".into(),
            date: "2026-01-03".into(), r#type: "expense".into(),
            category: "uncategorized".into(), is_contribution: false,
            is_withdrawal: false,
            import_source: "csv".into(), update_balance: true,
            created_at: "2026-01-03".into(),
        },
        NewTransaction {
            account_id: acct, transfer_account_id: None,
            amount: 500.0, description: "income".into(),
            date: "2026-01-04".into(), r#type: "income".into(),
            category: "uncategorized".into(), is_contribution: false,
            is_withdrawal: false,
            import_source: "csv".into(), update_balance: true,
            created_at: "2026-01-04".into(),
        },
    ];

    let n = transactions::bulk_create_transactions_with_snapshots(&conn, &rows).await.unwrap();
    assert_eq!(n, 3);

    // All three transactions were inserted.
    let page = transactions::list_transactions(&conn, &TransactionFilter::default()).await.unwrap();
    assert_eq!(page.rows.len(), 3);

    // Every transaction must have a generated_balance_id — snapshot was materialized.
    assert!(page.rows.iter().all(|r| r.generated_balance_id.is_some()));

    // Running balances chain correctly:
    //   seed 1000 → -100 → 900 → -200 → 700 → +500 → 1200
    assert_eq!(latest_balance(&conn, acct).await, 1200.0);

    // 1 seed + 3 generated = 4 balance rows total.
    assert_eq!(balance_count(&conn).await, 4);
}

// Balance of the snapshot recorded exactly on `date` (latest id wins for ties).
async fn balance_on(conn: &libsql::Connection, account_id: i32, date: &str) -> f64 {
    let mut rows = conn
        .query(
            "SELECT balance FROM account_balance WHERE account_id = ?1 AND recorded_at = ?2 \
             ORDER BY id DESC LIMIT 1",
            libsql::params![account_id, date],
        )
        .await
        .unwrap();
    rows.next().await.unwrap().unwrap().get::<f64>(0).unwrap()
}

fn checking(name: &str) -> NewAccount {
    NewAccount {
        name: name.into(),
        r#type: "checking".into(),
        institution: None,
        include_in_fire_calculations: false, count_payments_as_expense: false,
        created_at: "2026-01-01".into(), traditional_pct: None,
    }
}

fn dated_txn(account_id: i32, amount: f64, ty: &str, date: &str) -> NewTransaction {
    let mut t = new_txn(account_id, amount, ty);
    t.date = date.into();
    t.created_at = date.into();
    t.update_balance = true;
    t
}

// Inserting an out-of-order past transaction ripples its delta forward through
// later transaction-tied snapshots, while a manual snapshot anchors the chain and
// is left untouched.
#[tokio::test]
async fn inserting_past_transaction_propagates_to_later_linked_snapshots() {
    let conn = setup().await;
    let acct = accounts::create_account(&conn, &checking("Checking")).await.unwrap();
    accounts::add_balance(&conn, &trackmyfi_app_lib::commands::accounts::NewBalance {
        account_id: acct, balance: 1000.0, recorded_at: "2026-06-01".into() }).await.unwrap();

    // Two transaction-tied snapshots, in order: +100 on the 12th, +50 on the 15th.
    transactions::create_transaction_synced(&conn, &dated_txn(acct, 100.0, "income", "2026-06-12")).await.unwrap();
    transactions::create_transaction_synced(&conn, &dated_txn(acct, 50.0, "income", "2026-06-15")).await.unwrap();
    assert_eq!(balance_on(&conn, acct, "2026-06-12").await, 1100.0);
    assert_eq!(balance_on(&conn, acct, "2026-06-15").await, 1150.0);

    // A manual snapshot on the 20th — must stay put as an anchor.
    accounts::add_balance(&conn, &trackmyfi_app_lib::commands::accounts::NewBalance {
        account_id: acct, balance: 9999.0, recorded_at: "2026-06-20".into() }).await.unwrap();

    // Now go back and add a -40 expense on the 14th.
    transactions::create_transaction_synced(&conn, &dated_txn(acct, 40.0, "expense", "2026-06-14")).await.unwrap();

    assert_eq!(balance_on(&conn, acct, "2026-06-12").await, 1100.0); // before the insert: unchanged
    assert_eq!(balance_on(&conn, acct, "2026-06-14").await, 1060.0); // 1100 - 40
    assert_eq!(balance_on(&conn, acct, "2026-06-15").await, 1110.0); // 1150 shifted down by 40
    assert_eq!(balance_on(&conn, acct, "2026-06-20").await, 9999.0); // manual anchor: untouched
}

// Deleting a past transaction removes its contribution from later linked snapshots.
#[tokio::test]
async fn deleting_past_transaction_propagates_to_later_linked_snapshots() {
    let conn = setup().await;
    let acct = accounts::create_account(&conn, &checking("Checking")).await.unwrap();
    accounts::add_balance(&conn, &trackmyfi_app_lib::commands::accounts::NewBalance {
        account_id: acct, balance: 1000.0, recorded_at: "2026-06-01".into() }).await.unwrap();

    transactions::create_transaction_synced(&conn, &dated_txn(acct, 100.0, "income", "2026-06-12")).await.unwrap();
    let mid = transactions::create_transaction_synced(&conn, &dated_txn(acct, 40.0, "expense", "2026-06-14")).await.unwrap();
    transactions::create_transaction_synced(&conn, &dated_txn(acct, 50.0, "income", "2026-06-15")).await.unwrap();
    assert_eq!(balance_on(&conn, acct, "2026-06-15").await, 1110.0); // 1000 +100 -40 +50

    transactions::delete_transaction_synced(&conn, mid).await.unwrap();
    assert_eq!(balance_on(&conn, acct, "2026-06-15").await, 1150.0); // the -40 is gone: 1000 +100 +50
}

// Editing a past transaction's amount ripples the difference forward.
#[tokio::test]
async fn editing_past_transaction_amount_propagates_forward() {
    let conn = setup().await;
    let acct = accounts::create_account(&conn, &checking("Checking")).await.unwrap();
    accounts::add_balance(&conn, &trackmyfi_app_lib::commands::accounts::NewBalance {
        account_id: acct, balance: 1000.0, recorded_at: "2026-06-01".into() }).await.unwrap();

    transactions::create_transaction_synced(&conn, &dated_txn(acct, 100.0, "income", "2026-06-12")).await.unwrap();
    let mid = transactions::create_transaction_synced(&conn, &dated_txn(acct, 40.0, "expense", "2026-06-14")).await.unwrap();
    transactions::create_transaction_synced(&conn, &dated_txn(acct, 50.0, "income", "2026-06-15")).await.unwrap();

    // Bump the 14th expense from 40 to 90 (50 more spent).
    transactions::update_transaction_synced(&conn, &UpdateTransaction {
        id: mid, account_id: acct, transfer_account_id: None, amount: 90.0,
        description: "test".into(), date: "2026-06-14".into(), r#type: "expense".into(),
        category: "uncategorized".into(), is_contribution: false, is_withdrawal: false,
        update_balance: true, updated_at: "2026-06-16".into() }).await.unwrap();

    assert_eq!(balance_on(&conn, acct, "2026-06-14").await, 1010.0); // 1100 - 90
    assert_eq!(balance_on(&conn, acct, "2026-06-15").await, 1060.0); // 1010 + 50
}
