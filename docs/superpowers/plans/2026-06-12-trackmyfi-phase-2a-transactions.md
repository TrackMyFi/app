# Phase 2a — Transactions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Transactions ledger (manual entry + CSV import) to TrackMyFI, with an opt-in switch that materializes a transaction's effect as a linked balance snapshot.

**Architecture:** Transactions are an informational ledger parallel to the Phase 1 snapshot model. Account balances stay snapshot-only; a transaction can *optionally* write a linked `account_balance` snapshot (`generatedBalanceId` / `generatedBalanceToId`) when its "Update balance" switch is on. Transfers are single rows with a `transferAccountId`. CSV import is a generic column-mapping wizard with saved named mappings and duplicate detection; it never writes snapshots. Pure logic (signed deltas, totals, balance preview, CSV parse/map/dedup) lives in framework-free TypeScript and is unit-tested; the Rust layer mirrors the Phase 1 typed-command pattern.

**Tech Stack:** Tauri 2.x · Rust + libsql 0.9.30 · ts-rs 10 · Vue 3 + NuxtUI 4 · Pinia · Luxon · Vitest · papaparse

**⚠️ Naming note:** `TRANSACTION` is a SQLite keyword, so the physical table is named **`txn`** to avoid quoting every query. The entity/types stay `Transaction` everywhere in Rust and TypeScript. This is the only deviation from the spec's `transaction` table name.

**Spec:** `docs/superpowers/specs/2026-06-12-trackmyfi-phase-2a-transactions-design.md`

**Conventions carried from Phase 1 (read before starting):**
- Rust commands: testable inner `async fn(conn: &Connection, …)` + thin `#[tauri::command]` wrapper suffixed `_cmd`. Map rows MANUALLY by column index. Booleans stored INTEGER → read `row.get::<i64>(i)? != 0`.
- Models in `src-tauri/src/models.rs` derive `Serialize, Deserialize, TS, Clone` + `#[serde(rename_all = "camelCase")]` + `#[ts(export, export_to = "../../src/lib/types/")]`. IDs are `i32` so ts-rs emits `number`.
- Run Rust tests: `cd src-tauri && cargo test`. Run TS tests: `npm test` (vitest). Node is fnm-managed — if `npm`/`node` not found, prepend `~/Library/Application Support/fnm/node-versions/v24.12.0/installation/bin` to PATH.
- `ts-rs` regenerates `src/lib/types/*.ts` when Rust tests run (`cargo test`). Commit the regenerated `.ts` files.

---

## SUB-SLICE 1 — Schema, Rust CRUD, generated types

### Task 1: Migration `0002_transactions`

**Files:**
- Create: `src-tauri/migrations/0002_transactions.sql`
- Modify: `src-tauri/src/migrations.rs:11-15` (add Migration entry)
- Test: `src-tauri/tests/migrations.rs:17-22` (assert new tables)

- [ ] **Step 1: Write the migration SQL**

Create `src-tauri/migrations/0002_transactions.sql`:

```sql
CREATE TABLE txn (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  account_id INTEGER NOT NULL REFERENCES account(id) ON DELETE CASCADE,
  transfer_account_id INTEGER REFERENCES account(id) ON DELETE CASCADE,
  amount REAL NOT NULL,
  description TEXT NOT NULL,
  date TEXT NOT NULL,
  type TEXT NOT NULL,
  category TEXT NOT NULL DEFAULT 'uncategorized',
  is_contribution INTEGER NOT NULL DEFAULT 0,
  import_source TEXT NOT NULL DEFAULT 'manual',
  generated_balance_id INTEGER REFERENCES account_balance(id) ON DELETE SET NULL,
  generated_balance_to_id INTEGER REFERENCES account_balance(id) ON DELETE SET NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX idx_txn_account ON txn(account_id, date);
CREATE INDEX idx_txn_date ON txn(date);

CREATE TABLE import_mapping (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE,
  config TEXT NOT NULL,
  created_at TEXT NOT NULL
);
```

- [ ] **Step 2: Register the migration**

In `src-tauri/src/migrations.rs`, change the `MIGRATIONS` const (currently lines 11-15) to:

```rust
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "init",
        sql: include_str!("../migrations/0001_init.sql"),
    },
    Migration {
        version: 2,
        name: "transactions",
        sql: include_str!("../migrations/0002_transactions.sql"),
    },
];
```

- [ ] **Step 3: Extend the migration test**

In `src-tauri/tests/migrations.rs`, add `"txn"` and `"import_mapping"` to the asserted table list (the array starting at line 17):

```rust
    for t in [
        "fire_profile",
        "account",
        "account_balance",
        "txn",
        "import_mapping",
        "schema_migrations",
    ] {
        assert!(names.contains(t), "missing table {t}");
    }
```

- [ ] **Step 4: Run the migration test — verify it passes**

Run: `cd src-tauri && cargo test --test migrations`
Expected: PASS (`migrations_create_all_tables`).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/migrations/0002_transactions.sql src-tauri/src/migrations.rs src-tauri/tests/migrations.rs
git commit -m "feat: migration 0002 — txn + import_mapping tables"
```

---

### Task 2: Rust models + ts-rs types

**Files:**
- Modify: `src-tauri/src/models.rs` (append `Transaction`, `ImportMapping`)
- Generated (committed): `src/lib/types/Transaction.ts`, `src/lib/types/ImportMapping.ts`

- [ ] **Step 1: Add the model structs**

Append to `src-tauri/src/models.rs`:

```rust
#[derive(Serialize, Deserialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct Transaction {
    pub id: i32,
    pub account_id: i32,
    pub transfer_account_id: Option<i32>,
    pub amount: f64,
    pub description: String,
    pub date: String,
    pub r#type: String,
    pub category: String,
    pub is_contribution: bool,
    pub import_source: String,
    pub generated_balance_id: Option<i32>,
    pub generated_balance_to_id: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct ImportMapping {
    pub id: i32,
    pub name: String,
    pub config: String,
    pub created_at: String,
}
```

- [ ] **Step 2: Generate the TS types**

Run: `cd src-tauri && cargo test`
Expected: existing tests still PASS; `src/lib/types/Transaction.ts` and `src/lib/types/ImportMapping.ts` are written.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/models.rs src/lib/types/Transaction.ts src/lib/types/ImportMapping.ts
git commit -m "feat: Transaction + ImportMapping models with ts-rs types"
```

---

### Task 3: Transactions Rust commands (CRUD + filter + totals, no balance switch yet)

**Files:**
- Create: `src-tauri/src/commands/transactions.rs`
- Modify: `src-tauri/src/commands/mod.rs` (add `pub mod transactions;`)
- Test: `src-tauri/tests/transactions.rs`

> The "Update balance" switch logic is added in Task 10. This task ignores `update_balance` (rows insert with both `generated_*_id` NULL).

- [ ] **Step 1: Write the failing round-trip test**

Create `src-tauri/tests/transactions.rs`:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test --test transactions`
Expected: FAIL — `commands::transactions` does not exist (compile error).

- [ ] **Step 3: Implement the commands**

Create `src-tauri/src/commands/transactions.rs`:

```rust
use crate::db::Db;
use crate::models::Transaction;
use libsql::{params, Connection, Value};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use tauri::State;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewTransaction {
    pub account_id: i32,
    pub transfer_account_id: Option<i32>,
    pub amount: f64,
    pub description: String,
    pub date: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub category: String,
    pub is_contribution: bool,
    pub import_source: String,
    pub update_balance: bool,
    pub created_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTransaction {
    pub id: i32,
    pub account_id: i32,
    pub transfer_account_id: Option<i32>,
    pub amount: f64,
    pub description: String,
    pub date: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub category: String,
    pub is_contribution: bool,
    pub update_balance: bool,
    pub updated_at: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TransactionFilter {
    pub account_id: Option<i32>,
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    pub category: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct TransactionPage {
    pub rows: Vec<Transaction>,
    pub total_count: i32, // i32 so ts-rs emits `number`, not `bigint`
    pub total_income: f64,
    pub total_expense: f64,
    pub net: f64,
}

const COLS: &str = "id, account_id, transfer_account_id, amount, description, date, type, \
    category, is_contribution, import_source, generated_balance_id, generated_balance_to_id, \
    created_at, updated_at";

fn row_to_txn(row: &libsql::Row) -> Result<Transaction, String> {
    Ok(Transaction {
        id: row.get(0).map_err(|e| e.to_string())?,
        account_id: row.get(1).map_err(|e| e.to_string())?,
        transfer_account_id: row.get(2).map_err(|e| e.to_string())?,
        amount: row.get(3).map_err(|e| e.to_string())?,
        description: row.get(4).map_err(|e| e.to_string())?,
        date: row.get(5).map_err(|e| e.to_string())?,
        r#type: row.get(6).map_err(|e| e.to_string())?,
        category: row.get(7).map_err(|e| e.to_string())?,
        is_contribution: row.get::<i64>(8).map_err(|e| e.to_string())? != 0,
        import_source: row.get(9).map_err(|e| e.to_string())?,
        generated_balance_id: row.get(10).map_err(|e| e.to_string())?,
        generated_balance_to_id: row.get(11).map_err(|e| e.to_string())?,
        created_at: row.get(12).map_err(|e| e.to_string())?,
        updated_at: row.get(13).map_err(|e| e.to_string())?,
    })
}

// Build the WHERE clause + positional params from a filter.
fn build_where(f: &TransactionFilter, params: &mut Vec<Value>) -> String {
    let mut clauses: Vec<String> = Vec::new();
    if let Some(a) = f.account_id {
        clauses.push("(account_id = ? OR transfer_account_id = ?)".into());
        params.push(Value::Integer(a as i64));
        params.push(Value::Integer(a as i64));
    }
    if let Some(t) = &f.r#type {
        clauses.push("type = ?".into());
        params.push(Value::Text(t.clone()));
    }
    if let Some(c) = &f.category {
        clauses.push("category = ?".into());
        params.push(Value::Text(c.clone()));
    }
    if let Some(s) = &f.start_date {
        clauses.push("date >= ?".into());
        params.push(Value::Text(s.clone()));
    }
    if let Some(e) = &f.end_date {
        clauses.push("date <= ?".into());
        params.push(Value::Text(e.clone()));
    }
    if let Some(q) = &f.search {
        clauses.push("description LIKE ?".into());
        params.push(Value::Text(format!("%{}%", q)));
    }
    if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    }
}

pub async fn list_transactions(
    conn: &Connection,
    f: &TransactionFilter,
) -> Result<TransactionPage, String> {
    // page rows
    let mut row_params: Vec<Value> = Vec::new();
    let where_sql = build_where(f, &mut row_params);
    let limit = f.limit.unwrap_or(200);
    let offset = f.offset.unwrap_or(0);
    row_params.push(Value::Integer(limit));
    row_params.push(Value::Integer(offset));
    let sql = format!(
        "SELECT {COLS} FROM txn {where_sql} ORDER BY date DESC, id DESC LIMIT ? OFFSET ?"
    );
    let mut rows = conn
        .query(&sql, libsql::params_from_iter(row_params))
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(row_to_txn(&row)?);
    }

    // totals over the full filter (transfers excluded)
    let mut agg_params: Vec<Value> = Vec::new();
    let agg_where = build_where(f, &mut agg_params);
    // CAST(... AS REAL) is required: SQLite SUM can return an Integer value, which
    // libsql's f64 reader rejects ("invalid value type").
    let agg_sql = format!(
        "SELECT \
           COUNT(*), \
           CAST(COALESCE(SUM(CASE WHEN type='income' THEN amount ELSE 0 END), 0) AS REAL), \
           CAST(COALESCE(SUM(CASE WHEN type='expense' THEN amount ELSE 0 END), 0) AS REAL) \
         FROM txn {agg_where}"
    );
    let mut agg = conn
        .query(&agg_sql, libsql::params_from_iter(agg_params))
        .await
        .map_err(|e| e.to_string())?;
    let arow = agg
        .next()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "aggregate row missing".to_string())?;
    let total_count: i64 = arow.get(0).map_err(|e| e.to_string())?;
    let total_income: f64 = arow.get(1).map_err(|e| e.to_string())?;
    let total_expense: f64 = arow.get(2).map_err(|e| e.to_string())?;

    Ok(TransactionPage {
        rows: out,
        total_count: total_count as i32,
        total_income,
        total_expense,
        net: total_income - total_expense,
    })
}

pub async fn create_transaction(conn: &Connection, t: &NewTransaction) -> Result<i32, String> {
    // Balance materialization is added in Task 10; for now generated ids are NULL.
    conn.execute(
        "INSERT INTO txn (account_id, transfer_account_id, amount, description, date, type, \
         category, is_contribution, import_source, generated_balance_id, \
         generated_balance_to_id, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL, NULL, ?10, ?10)",
        params![
            t.account_id,
            t.transfer_account_id,
            t.amount,
            t.description.clone(),
            t.date.clone(),
            t.r#type.clone(),
            t.category.clone(),
            t.is_contribution,
            t.import_source.clone(),
            t.created_at.clone()
        ],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid() as i32)
}

pub async fn update_transaction(conn: &Connection, t: &UpdateTransaction) -> Result<(), String> {
    // Balance re-materialization is added in Task 10.
    conn.execute(
        "UPDATE txn SET account_id=?1, transfer_account_id=?2, amount=?3, description=?4, \
         date=?5, type=?6, category=?7, is_contribution=?8, updated_at=?9 WHERE id=?10",
        params![
            t.account_id,
            t.transfer_account_id,
            t.amount,
            t.description.clone(),
            t.date.clone(),
            t.r#type.clone(),
            t.category.clone(),
            t.is_contribution,
            t.updated_at.clone(),
            t.id
        ],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn delete_transaction(conn: &Connection, id: i32) -> Result<(), String> {
    // Generated-snapshot cleanup is added in Task 10.
    conn.execute("DELETE FROM txn WHERE id = ?1", params![id])
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ---- thin command wrappers ----

#[tauri::command]
pub async fn list_transactions_cmd(
    db: State<'_, Db>,
    filter: TransactionFilter,
) -> Result<TransactionPage, String> {
    let conn = db.conn().await?;
    list_transactions(&conn, &filter).await
}

#[tauri::command]
pub async fn create_transaction_cmd(
    db: State<'_, Db>,
    transaction: NewTransaction,
) -> Result<i32, String> {
    let conn = db.conn().await?;
    create_transaction(&conn, &transaction).await
}

#[tauri::command]
pub async fn update_transaction_cmd(
    db: State<'_, Db>,
    transaction: UpdateTransaction,
) -> Result<(), String> {
    let conn = db.conn().await?;
    update_transaction(&conn, &transaction).await
}

#[tauri::command]
pub async fn delete_transaction_cmd(db: State<'_, Db>, id: i32) -> Result<(), String> {
    let conn = db.conn().await?;
    delete_transaction(&conn, id).await
}
```

Add to `src-tauri/src/commands/mod.rs`:

```rust
pub mod accounts;
pub mod fire_profile;
pub mod transactions;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test --test transactions`
Expected: PASS — `transaction_crud_and_totals`, `transfers_excluded_from_totals`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/transactions.rs src-tauri/src/commands/mod.rs src-tauri/tests/transactions.rs src/lib/types/TransactionPage.ts
git commit -m "feat: transaction CRUD commands with filter + totals"
```

---

### Task 4: Import-mapping Rust commands

**Files:**
- Create: `src-tauri/src/commands/import_mappings.rs`
- Modify: `src-tauri/src/commands/mod.rs` (add `pub mod import_mappings;`)
- Test: `src-tauri/tests/import_mappings.rs`

- [ ] **Step 1: Write the failing round-trip test**

Create `src-tauri/tests/import_mappings.rs`:

```rust
use libsql::Builder;
use trackmyfi_app_lib::commands::import_mappings::{self, NewImportMapping};
use trackmyfi_app_lib::migrations;

#[tokio::test]
async fn import_mapping_roundtrip() {
    let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();
    migrations::run(&conn).await.unwrap();

    let id = import_mappings::create_import_mapping(
        &conn,
        &NewImportMapping {
            name: "Chase Checking".into(),
            config: "{\"date\":\"Posting Date\"}".into(),
            created_at: "2026-03-01".into(),
        },
    )
    .await
    .unwrap();
    assert!(id >= 1);

    let list = import_mappings::list_import_mappings(&conn).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "Chase Checking");
    assert_eq!(list[0].config, "{\"date\":\"Posting Date\"}");

    import_mappings::delete_import_mapping(&conn, id).await.unwrap();
    assert_eq!(import_mappings::list_import_mappings(&conn).await.unwrap().len(), 0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test --test import_mappings`
Expected: FAIL — module does not exist.

- [ ] **Step 3: Implement the commands**

Create `src-tauri/src/commands/import_mappings.rs`:

```rust
use crate::db::Db;
use crate::models::ImportMapping;
use libsql::{params, Connection};
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewImportMapping {
    pub name: String,
    pub config: String,
    pub created_at: String,
}

fn row_to_mapping(row: &libsql::Row) -> Result<ImportMapping, String> {
    Ok(ImportMapping {
        id: row.get(0).map_err(|e| e.to_string())?,
        name: row.get(1).map_err(|e| e.to_string())?,
        config: row.get(2).map_err(|e| e.to_string())?,
        created_at: row.get(3).map_err(|e| e.to_string())?,
    })
}

pub async fn list_import_mappings(conn: &Connection) -> Result<Vec<ImportMapping>, String> {
    let mut rows = conn
        .query(
            "SELECT id, name, config, created_at FROM import_mapping ORDER BY name",
            (),
        )
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(row_to_mapping(&row)?);
    }
    Ok(out)
}

pub async fn create_import_mapping(
    conn: &Connection,
    m: &NewImportMapping,
) -> Result<i32, String> {
    conn.execute(
        "INSERT INTO import_mapping (name, config, created_at) VALUES (?1, ?2, ?3)",
        params![m.name.clone(), m.config.clone(), m.created_at.clone()],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid() as i32)
}

pub async fn delete_import_mapping(conn: &Connection, id: i32) -> Result<(), String> {
    conn.execute("DELETE FROM import_mapping WHERE id = ?1", params![id])
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn list_import_mappings_cmd(db: State<'_, Db>) -> Result<Vec<ImportMapping>, String> {
    let conn = db.conn().await?;
    list_import_mappings(&conn).await
}

#[tauri::command]
pub async fn create_import_mapping_cmd(
    db: State<'_, Db>,
    mapping: NewImportMapping,
) -> Result<i32, String> {
    let conn = db.conn().await?;
    create_import_mapping(&conn, &mapping).await
}

#[tauri::command]
pub async fn delete_import_mapping_cmd(db: State<'_, Db>, id: i32) -> Result<(), String> {
    let conn = db.conn().await?;
    delete_import_mapping(&conn, id).await
}
```

Add `pub mod import_mappings;` to `src-tauri/src/commands/mod.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test --test import_mappings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/import_mappings.rs src-tauri/src/commands/mod.rs src-tauri/tests/import_mappings.rs
git commit -m "feat: import_mapping CRUD commands"
```

---

### Task 5: Register all new commands

**Files:**
- Modify: `src-tauri/src/lib.rs:20-33` (extend `generate_handler!`)

- [ ] **Step 1: Add the handlers**

In `src-tauri/src/lib.rs`, add these lines inside `tauri::generate_handler![ … ]` after the existing accounts entries:

```rust
            commands::transactions::list_transactions_cmd,
            commands::transactions::create_transaction_cmd,
            commands::transactions::update_transaction_cmd,
            commands::transactions::delete_transaction_cmd,
            commands::import_mappings::list_import_mappings_cmd,
            commands::import_mappings::create_import_mapping_cmd,
            commands::import_mappings::delete_import_mapping_cmd,
```

- [ ] **Step 2: Verify it compiles**

Run: `cd src-tauri && cargo build`
Expected: builds clean (no warnings about unused commands).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: register transaction + import-mapping commands"
```

---

## SUB-SLICE 2 — Pure helpers, API, store, list workbench, manual form

### Task 6: Transaction constants + pure helpers (signed delta, totals)

**Files:**
- Create: `src/lib/transactions/constants.ts`
- Create: `src/lib/transactions/totals.ts`
- Test: `src/lib/transactions/totals.test.ts`

- [ ] **Step 1: Write the constants**

Create `src/lib/transactions/constants.ts`:

```ts
export const TRANSACTION_TYPES = ['income', 'expense', 'transfer'] as const
export type TransactionType = typeof TRANSACTION_TYPES[number]

export const CATEGORIES = ['savings', 'fixed', 'discretionary', 'uncategorized'] as const
export type Category = typeof CATEGORIES[number]

/** Signed effect of a transaction on its PRIMARY account's balance. */
export function signedDelta(type: string, amount: number): number {
  if (type === 'income') return amount
  if (type === 'expense') return -amount
  return -amount // transfer: primary (source) account decreases
}
```

- [ ] **Step 2: Write the failing totals test**

Create `src/lib/transactions/totals.test.ts`:

```ts
import { describe, it, expect } from 'vitest'
import { runningTotals } from './totals'

describe('runningTotals', () => {
  it('sums income and expense, ignores transfers, computes net', () => {
    const rows = [
      { type: 'income', amount: 1000 },
      { type: 'expense', amount: 40 },
      { type: 'expense', amount: 60 },
      { type: 'transfer', amount: 500 },
    ]
    expect(runningTotals(rows)).toEqual({ income: 1000, expense: 100, net: 900 })
  })

  it('returns zeros for an empty set', () => {
    expect(runningTotals([])).toEqual({ income: 0, expense: 0, net: 0 })
  })
})
```

- [ ] **Step 3: Run test to verify it fails**

Run: `npm test -- totals`
Expected: FAIL — `./totals` not found.

- [ ] **Step 4: Implement `runningTotals`**

Create `src/lib/transactions/totals.ts`:

```ts
export interface AmountRow { type: string; amount: number }

export function runningTotals(rows: AmountRow[]): {
  income: number; expense: number; net: number
} {
  let income = 0
  let expense = 0
  for (const r of rows) {
    if (r.type === 'income') income += r.amount
    else if (r.type === 'expense') expense += r.amount
  }
  return { income, expense, net: income - expense }
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `npm test -- totals`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/lib/transactions/constants.ts src/lib/transactions/totals.ts src/lib/transactions/totals.test.ts
git commit -m "feat: transaction constants + runningTotals helper"
```

---

### Task 7: API layer + Pinia store

**Files:**
- Create: `src/lib/api/transactions.ts`
- Create: `src/lib/api/importMappings.ts`
- Create: `src/stores/transactions.ts`

- [ ] **Step 1: Write the transactions API wrapper**

Create `src/lib/api/transactions.ts`:

```ts
import { invoke } from '@tauri-apps/api/core'
import type { TransactionPage } from '../types/TransactionPage'

export interface TransactionFilter {
  accountId?: number | null
  type?: string | null
  category?: string | null
  startDate?: string | null
  endDate?: string | null
  search?: string | null
  limit?: number | null
  offset?: number | null
}

export interface NewTransaction {
  accountId: number
  transferAccountId: number | null
  amount: number
  description: string
  date: string
  type: string
  category: string
  isContribution: boolean
  importSource: string
  updateBalance: boolean
  createdAt: string
}

export interface UpdateTransaction {
  id: number
  accountId: number
  transferAccountId: number | null
  amount: number
  description: string
  date: string
  type: string
  category: string
  isContribution: boolean
  updateBalance: boolean
  updatedAt: string
}

export const listTransactions = (filter: TransactionFilter = {}) =>
  invoke<TransactionPage>('list_transactions_cmd', { filter })
export const createTransaction = (transaction: NewTransaction) =>
  invoke<number>('create_transaction_cmd', { transaction })
export const updateTransaction = (transaction: UpdateTransaction) =>
  invoke<void>('update_transaction_cmd', { transaction })
export const deleteTransaction = (id: number) =>
  invoke<void>('delete_transaction_cmd', { id })
```

- [ ] **Step 2: Write the import-mappings API wrapper**

Create `src/lib/api/importMappings.ts`:

```ts
import { invoke } from '@tauri-apps/api/core'
import type { ImportMapping } from '../types/ImportMapping'

export interface NewImportMapping {
  name: string
  config: string
  createdAt: string
}

export const listImportMappings = () =>
  invoke<ImportMapping[]>('list_import_mappings_cmd')
export const createImportMapping = (mapping: NewImportMapping) =>
  invoke<number>('create_import_mapping_cmd', { mapping })
export const deleteImportMapping = (id: number) =>
  invoke<void>('delete_import_mapping_cmd', { id })
```

- [ ] **Step 3: Write the Pinia store**

Create `src/stores/transactions.ts`:

```ts
import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { TransactionPage } from '../lib/types/TransactionPage'
import type { Transaction } from '../lib/types/Transaction'
import * as api from '../lib/api/transactions'

const EMPTY: TransactionPage = { rows: [], totalCount: 0, totalIncome: 0, totalExpense: 0, net: 0 }

export const useTransactionsStore = defineStore('transactions', () => {
  const page = ref<TransactionPage>(EMPTY)
  const filter = ref<api.TransactionFilter>({ limit: 200, offset: 0 })

  async function load() {
    page.value = await api.listTransactions(filter.value)
  }
  async function setFilter(patch: Partial<api.TransactionFilter>) {
    filter.value = { ...filter.value, ...patch, offset: 0 }
    await load()
  }
  async function create(t: api.NewTransaction) { await api.createTransaction(t); await load() }
  async function update(t: api.UpdateTransaction) { await api.updateTransaction(t); await load() }
  async function remove(id: number) { await api.deleteTransaction(id); await load() }

  return { page, filter, load, setFilter, create, update, remove }
})

export type { Transaction }
```

- [ ] **Step 4: Verify it type-checks**

Run: `npx vue-tsc --noEmit`
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src/lib/api/transactions.ts src/lib/api/importMappings.ts src/stores/transactions.ts
git commit -m "feat: transactions api + store"
```

---

### Task 8: Transactions page (list workbench) + route + nav

**Files:**
- Create: `src/pages/Transactions.vue`
- Modify: `src/router.ts` (add route)
- Modify: `src/App.vue:5` (enable nav link)

> The form modal opened by this page is built in Task 9. For this task, the "Add transaction" button and row edit just toggle a ref; the `<TransactionForm>` element is added in Task 9. Build the table, filters, and totals now.

- [ ] **Step 1: Add the route**

In `src/router.ts`, add to the `routes` array (after the accounts line):

```ts
  { path: '/transactions', name: 'transactions', component: () => import('./pages/Transactions.vue') },
```

- [ ] **Step 2: Enable the nav link**

In `src/App.vue`, replace the Transactions link (line 5) with:

```ts
  { label: 'Transactions', to: '/transactions', icon: 'i-lucide-receipt' },
```

- [ ] **Step 3: Build the page**

Create `src/pages/Transactions.vue`:

```vue
<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useTransactionsStore } from '../stores/transactions'
import { useAccountsStore } from '../stores/accounts'
import { TRANSACTION_TYPES, CATEGORIES } from '../lib/transactions/constants'
import type { Transaction } from '../lib/types/Transaction'

const store = useTransactionsStore()
const accountsStore = useAccountsStore()

const accountId = ref<number | undefined>(undefined)
const type = ref<string | undefined>(undefined)
const category = ref<string | undefined>(undefined)
const search = ref('')

function accountName(id: number | null): string {
  if (id == null) return '—'
  return accountsStore.accounts.find((a) => a.id === id)?.name ?? `#${id}`
}

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}

async function applyFilters() {
  await store.setFilter({
    accountId: accountId.value ?? null,
    type: type.value ?? null,
    category: category.value ?? null,
    search: search.value || null,
  })
}

const rows = computed(() => store.page.rows)

onMounted(async () => {
  await accountsStore.load()
  await store.load()
})
</script>

<template>
  <div class="p-6 space-y-4">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-semibold">Transactions</h1>
    </div>

    <div class="flex flex-wrap gap-2 items-end">
      <USelect
        v-model="accountId"
        :items="accountsStore.accounts.map((a) => ({ label: a.name, value: a.id }))"
        placeholder="All accounts"
        class="w-44"
      />
      <USelect
        v-model="type"
        :items="TRANSACTION_TYPES.map((t) => ({ label: t, value: t }))"
        placeholder="All types"
        class="w-36"
      />
      <USelect
        v-model="category"
        :items="CATEGORIES.map((c) => ({ label: c, value: c }))"
        placeholder="All categories"
        class="w-40"
      />
      <UInput v-model="search" placeholder="Search description" class="w-52" />
      <UButton @click="applyFilters">Apply</UButton>
    </div>

    <div class="flex gap-6 text-sm">
      <span>Income: <strong class="text-green-600">{{ money(store.page.totalIncome) }}</strong></span>
      <span>Expense: <strong class="text-red-600">{{ money(store.page.totalExpense) }}</strong></span>
      <span>Net: <strong>{{ money(store.page.net) }}</strong></span>
      <span class="text-muted">{{ store.page.totalCount }} rows</span>
    </div>

    <table class="w-full text-sm">
      <thead class="text-left text-muted border-b border-default">
        <tr>
          <th class="py-2">Date</th>
          <th>Description</th>
          <th>Account</th>
          <th>Type</th>
          <th>Category</th>
          <th class="text-right">Amount</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="t in rows" :key="t.id" class="border-b border-default/50">
          <td class="py-2">{{ t.date }}</td>
          <td>{{ t.description }}</td>
          <td>
            {{ accountName(t.accountId) }}
            <span v-if="t.type === 'transfer'"> → {{ accountName(t.transferAccountId) }}</span>
          </td>
          <td>{{ t.type }}</td>
          <td>{{ t.category }}</td>
          <td class="text-right tabular-nums">{{ money(t.amount) }}</td>
        </tr>
        <tr v-if="!rows.length">
          <td colspan="6" class="py-6 text-center text-muted">No transactions yet.</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
```

- [ ] **Step 4: Verify it type-checks**

Run: `npx vue-tsc --noEmit`
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src/pages/Transactions.vue src/router.ts src/App.vue
git commit -m "feat: transactions list workbench page with filters + totals"
```

---

### Task 9: Transaction form (manual entry/edit, no balance switch yet)

**Files:**
- Create: `src/components/TransactionForm.vue`
- Modify: `src/pages/Transactions.vue` (wire up Add/Edit/Delete + modal)

> The balance-update switch is added in Task 12. Build the rest of the form now.

- [ ] **Step 1: Build the form component**

Create `src/components/TransactionForm.vue`:

```vue
<script setup lang="ts">
import { computed, reactive, watch } from 'vue'
import { DateTime } from 'luxon'
import { useTransactionsStore } from '../stores/transactions'
import { useAccountsStore } from '../stores/accounts'
import { TRANSACTION_TYPES, CATEGORIES } from '../lib/transactions/constants'
import DateInput from './DateInput.vue'
import type { Transaction } from '../lib/types/Transaction'

const props = defineProps<{ editing: Transaction | null }>()
const emit = defineEmits<{ saved: [] }>()

const store = useTransactionsStore()
const accountsStore = useAccountsStore()

const today = DateTime.now().toISODate()!

const form = reactive({
  accountId: undefined as number | undefined,
  transferAccountId: null as number | null,
  amount: 0,
  description: '',
  date: today,
  type: 'expense',
  category: 'uncategorized',
  isContribution: false,
})

watch(
  () => props.editing,
  (e) => {
    if (e) {
      form.accountId = e.accountId
      form.transferAccountId = e.transferAccountId
      form.amount = e.amount
      form.description = e.description
      form.date = e.date
      form.type = e.type
      form.category = e.category
      form.isContribution = e.isContribution
    } else {
      form.accountId = undefined
      form.transferAccountId = null
      form.amount = 0
      form.description = ''
      form.date = today
      form.type = 'expense'
      form.category = 'uncategorized'
      form.isContribution = false
    }
  },
  { immediate: true },
)

const isTransfer = computed(() => form.type === 'transfer')
const accountItems = computed(() =>
  accountsStore.accounts.map((a) => ({ label: a.name, value: a.id })),
)

async function save() {
  if (form.accountId == null) return
  const now = DateTime.now().toISO()!
  if (props.editing) {
    await store.update({
      id: props.editing.id,
      accountId: form.accountId,
      transferAccountId: isTransfer.value ? form.transferAccountId : null,
      amount: form.amount,
      description: form.description,
      date: form.date,
      type: form.type,
      category: form.category,
      isContribution: form.isContribution,
      updateBalance: false,
      updatedAt: now,
    })
  } else {
    await store.create({
      accountId: form.accountId,
      transferAccountId: isTransfer.value ? form.transferAccountId : null,
      amount: form.amount,
      description: form.description,
      date: form.date,
      type: form.type,
      category: form.category,
      isContribution: form.isContribution,
      importSource: 'manual',
      updateBalance: false,
      createdAt: now,
    })
  }
  emit('saved')
}
</script>

<template>
  <form class="space-y-3" @submit.prevent="save">
    <USelect v-model="form.type" :items="TRANSACTION_TYPES.map((t) => ({ label: t, value: t }))" />
    <USelect v-model="form.accountId" :items="accountItems" :placeholder="isTransfer ? 'From account' : 'Account'" />
    <USelect v-if="isTransfer" v-model="form.transferAccountId" :items="accountItems" placeholder="To account" />
    <UInput v-model.number="form.amount" type="number" step="0.01" placeholder="Amount" />
    <UInput v-model="form.description" placeholder="Description" />
    <DateInput v-model="form.date" />
    <USelect v-if="!isTransfer" v-model="form.category" :items="CATEGORIES.map((c) => ({ label: c, value: c }))" />
    <UCheckbox v-if="!isTransfer" v-model="form.isContribution" label="Counts as an investment contribution" />
    <div class="flex justify-end gap-2 pt-2">
      <UButton type="submit">{{ props.editing ? 'Save' : 'Add' }}</UButton>
    </div>
  </form>
</template>
```

- [ ] **Step 2: Wire the form into the page**

In `src/pages/Transactions.vue`, add to the `<script setup>` block:

```ts
import TransactionForm from '../components/TransactionForm.vue'
import { confirm } from '@tauri-apps/plugin-dialog'

const isModalOpen = ref(false)
const editing = ref<Transaction | null>(null)

function openAdd() { editing.value = null; isModalOpen.value = true }
function openEdit(t: Transaction) { editing.value = t; isModalOpen.value = true }
function onSaved() { isModalOpen.value = false }

async function removeRow(t: Transaction) {
  const ok = await confirm(`Delete "${t.description}"?`, { title: 'Delete transaction' })
  if (ok) await store.remove(t.id)
}
```

In the template, change the header button row to include an Add button:

```vue
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-semibold">Transactions</h1>
      <UButton icon="i-lucide-plus" @click="openAdd">Add transaction</UButton>
    </div>
```

Add an actions column header (`<th></th>` at the end of the `<tr>` in `<thead>`) and per-row actions cell at the end of each body `<tr>`:

```vue
          <td class="text-right">
            <UButton size="xs" variant="ghost" icon="i-lucide-pencil" @click="openEdit(t)" />
            <UButton size="xs" variant="ghost" color="error" icon="i-lucide-trash-2" @click="removeRow(t)" />
          </td>
```

And add the modal at the end of the root `<div>`:

```vue
    <UModal v-model:open="isModalOpen" :title="editing ? 'Edit transaction' : 'Add transaction'">
      <template #body>
        <TransactionForm :editing="editing" @saved="onSaved" />
      </template>
    </UModal>
```

- [ ] **Step 3: Verify it type-checks**

Run: `npx vue-tsc --noEmit`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/components/TransactionForm.vue src/pages/Transactions.vue
git commit -m "feat: transaction form with manual add/edit/delete"
```

---

## SUB-SLICE 3 — Balance-update switch + linked snapshots

### Task 10: Rust — materialize linked snapshots on create/update/delete

**Files:**
- Modify: `src-tauri/src/commands/transactions.rs` (create/update/delete bodies + helpers)
- Test: `src-tauri/tests/transactions.rs` (add balance-switch cases)

- [ ] **Step 1: Write the failing tests**

Append to `src-tauri/tests/transactions.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test --test transactions`
Expected: FAIL — new assertions (snapshots not written / not linked).

- [ ] **Step 3: Implement snapshot materialization**

In `src-tauri/src/commands/transactions.rs`, add these helpers above `create_transaction`:

```rust
async fn base_balance(conn: &Connection, account_id: i32, date: &str) -> Result<f64, String> {
    let mut rows = conn
        .query(
            "SELECT balance FROM account_balance WHERE account_id = ?1 AND recorded_at <= ?2 \
             ORDER BY recorded_at DESC, id DESC LIMIT 1",
            params![account_id, date],
        )
        .await
        .map_err(|e| e.to_string())?;
    match rows.next().await.map_err(|e| e.to_string())? {
        Some(r) => r.get::<f64>(0).map_err(|e| e.to_string()),
        None => Ok(0.0),
    }
}

async fn insert_snapshot(
    conn: &Connection,
    account_id: i32,
    balance: f64,
    recorded_at: &str,
) -> Result<i32, String> {
    conn.execute(
        "INSERT INTO account_balance (account_id, balance, recorded_at) VALUES (?1, ?2, ?3)",
        params![account_id, balance, recorded_at],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid() as i32)
}

async fn delete_snapshot(conn: &Connection, id: i32) -> Result<(), String> {
    conn.execute("DELETE FROM account_balance WHERE id = ?1", params![id])
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// Returns (generated_balance_id, generated_balance_to_id) for a materialized txn.
async fn materialize_snapshots(
    conn: &Connection,
    account_id: i32,
    transfer_account_id: Option<i32>,
    amount: f64,
    ty: &str,
    date: &str,
) -> Result<(Option<i32>, Option<i32>), String> {
    if ty == "transfer" {
        let to = transfer_account_id.ok_or("transfer requires transferAccountId")?;
        let src_base = base_balance(conn, account_id, date).await?;
        let dst_base = base_balance(conn, to, date).await?;
        let gen = insert_snapshot(conn, account_id, src_base - amount, date).await?;
        let gen_to = insert_snapshot(conn, to, dst_base + amount, date).await?;
        Ok((Some(gen), Some(gen_to)))
    } else {
        let delta = if ty == "income" { amount } else { -amount };
        let base = base_balance(conn, account_id, date).await?;
        let gen = insert_snapshot(conn, account_id, base + delta, date).await?;
        Ok((Some(gen), None))
    }
}

// Reads the current generated ids for a txn (used before re-materializing or deleting).
async fn generated_ids(conn: &Connection, txn_id: i32) -> Result<(Option<i32>, Option<i32>), String> {
    let mut rows = conn
        .query(
            "SELECT generated_balance_id, generated_balance_to_id FROM txn WHERE id = ?1",
            params![txn_id],
        )
        .await
        .map_err(|e| e.to_string())?;
    match rows.next().await.map_err(|e| e.to_string())? {
        Some(r) => Ok((
            r.get(0).map_err(|e| e.to_string())?,
            r.get(1).map_err(|e| e.to_string())?,
        )),
        None => Ok((None, None)),
    }
}

async fn clear_generated(conn: &Connection, ids: (Option<i32>, Option<i32>)) -> Result<(), String> {
    if let Some(id) = ids.0 {
        delete_snapshot(conn, id).await?;
    }
    if let Some(id) = ids.1 {
        delete_snapshot(conn, id).await?;
    }
    Ok(())
}
```

Replace the body of `create_transaction` with:

```rust
pub async fn create_transaction(conn: &Connection, t: &NewTransaction) -> Result<i32, String> {
    let (gen_id, gen_to_id) = if t.update_balance {
        materialize_snapshots(
            conn,
            t.account_id,
            t.transfer_account_id,
            t.amount,
            &t.r#type,
            &t.date,
        )
        .await?
    } else {
        (None, None)
    };

    conn.execute(
        "INSERT INTO txn (account_id, transfer_account_id, amount, description, date, type, \
         category, is_contribution, import_source, generated_balance_id, \
         generated_balance_to_id, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
        params![
            t.account_id,
            t.transfer_account_id,
            t.amount,
            t.description.clone(),
            t.date.clone(),
            t.r#type.clone(),
            t.category.clone(),
            t.is_contribution,
            t.import_source.clone(),
            gen_id,
            gen_to_id,
            t.created_at.clone()
        ],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid() as i32)
}
```

Replace the body of `update_transaction` with (delete-and-recreate the linked snapshots so the result reflects the new amount/date/account):

```rust
pub async fn update_transaction(conn: &Connection, t: &UpdateTransaction) -> Result<(), String> {
    // Remove any previously generated snapshots first.
    clear_generated(conn, generated_ids(conn, t.id).await?).await?;

    let (gen_id, gen_to_id) = if t.update_balance {
        materialize_snapshots(
            conn,
            t.account_id,
            t.transfer_account_id,
            t.amount,
            &t.r#type,
            &t.date,
        )
        .await?
    } else {
        (None, None)
    };

    conn.execute(
        "UPDATE txn SET account_id=?1, transfer_account_id=?2, amount=?3, description=?4, \
         date=?5, type=?6, category=?7, is_contribution=?8, generated_balance_id=?9, \
         generated_balance_to_id=?10, updated_at=?11 WHERE id=?12",
        params![
            t.account_id,
            t.transfer_account_id,
            t.amount,
            t.description.clone(),
            t.date.clone(),
            t.r#type.clone(),
            t.category.clone(),
            t.is_contribution,
            gen_id,
            gen_to_id,
            t.updated_at.clone(),
            t.id
        ],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}
```

Replace the body of `delete_transaction` with:

```rust
pub async fn delete_transaction(conn: &Connection, id: i32) -> Result<(), String> {
    clear_generated(conn, generated_ids(conn, id).await?).await?;
    conn.execute("DELETE FROM txn WHERE id = ?1", params![id])
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test --test transactions`
Expected: PASS — all six tests, including the original two from Task 3.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/transactions.rs src-tauri/tests/transactions.rs
git commit -m "feat: materialize linked balance snapshots on the update-balance switch"
```

---

### Task 11: Balance-preview pure helper

**Files:**
- Create: `src/lib/transactions/balancePreview.ts`
- Test: `src/lib/transactions/balancePreview.test.ts`

- [ ] **Step 1: Write the failing test**

Create `src/lib/transactions/balancePreview.test.ts`:

```ts
import { describe, it, expect } from 'vitest'
import { balancePreview } from './balancePreview'
import type { AccountBalance } from '../types/AccountBalance'

const balances: AccountBalance[] = [
  { id: 1, accountId: 10, balance: 1000, recordedAt: '2026-02-01' },
  { id: 2, accountId: 20, balance: 200, recordedAt: '2026-02-01' },
]

describe('balancePreview', () => {
  it('previews an expense against the latest on/before date', () => {
    const p = balancePreview(balances, { type: 'expense', amount: 40, accountId: 10, transferAccountId: null, date: '2026-03-01' })
    expect(p).toEqual([{ accountId: 10, from: 1000, to: 960 }])
  })

  it('previews income', () => {
    const p = balancePreview(balances, { type: 'income', amount: 500, accountId: 10, transferAccountId: null, date: '2026-03-01' })
    expect(p).toEqual([{ accountId: 10, from: 1000, to: 1500 }])
  })

  it('previews both sides of a transfer', () => {
    const p = balancePreview(balances, { type: 'transfer', amount: 300, accountId: 10, transferAccountId: 20, date: '2026-03-01' })
    expect(p).toEqual([
      { accountId: 10, from: 1000, to: 700 },
      { accountId: 20, from: 200, to: 500 },
    ])
  })

  it('uses base 0 when no prior snapshot exists', () => {
    const p = balancePreview(balances, { type: 'expense', amount: 40, accountId: 99, transferAccountId: null, date: '2026-03-01' })
    expect(p).toEqual([{ accountId: 99, from: 0, to: -40 }])
  })

  it('ignores snapshots dated after the transaction', () => {
    const future: AccountBalance[] = [{ id: 3, accountId: 10, balance: 5000, recordedAt: '2026-12-01' }]
    const p = balancePreview([...balances, ...future], { type: 'expense', amount: 40, accountId: 10, transferAccountId: null, date: '2026-03-01' })
    expect(p).toEqual([{ accountId: 10, from: 1000, to: 960 }])
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- balancePreview`
Expected: FAIL — `./balancePreview` not found.

- [ ] **Step 3: Implement the helper**

Create `src/lib/transactions/balancePreview.ts`:

```ts
import type { AccountBalance } from '../types/AccountBalance'
import { signedDelta } from './constants'

export interface PreviewInput {
  type: string
  amount: number
  accountId: number
  transferAccountId: number | null
  date: string
}

export interface PreviewLine {
  accountId: number
  from: number
  to: number
}

function baseBalance(balances: AccountBalance[], accountId: number, date: string): number {
  const candidates = balances
    .filter((b) => b.accountId === accountId && b.recordedAt <= date)
    .sort((a, b) => (a.recordedAt < b.recordedAt ? 1 : a.recordedAt > b.recordedAt ? -1 : b.id - a.id))
  return candidates.length ? candidates[0].balance : 0
}

/** Mirrors the Rust `materialize_snapshots` math for the form preview. */
export function balancePreview(balances: AccountBalance[], input: PreviewInput): PreviewLine[] {
  if (input.type === 'transfer') {
    if (input.transferAccountId == null) return []
    const srcBase = baseBalance(balances, input.accountId, input.date)
    const dstBase = baseBalance(balances, input.transferAccountId, input.date)
    return [
      { accountId: input.accountId, from: srcBase, to: srcBase - input.amount },
      { accountId: input.transferAccountId, from: dstBase, to: dstBase + input.amount },
    ]
  }
  const base = baseBalance(balances, input.accountId, input.date)
  return [{ accountId: input.accountId, from: base, to: base + signedDelta(input.type, input.amount) }]
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- balancePreview`
Expected: PASS (all five cases).

- [ ] **Step 5: Commit**

```bash
git add src/lib/transactions/balancePreview.ts src/lib/transactions/balancePreview.test.ts
git commit -m "feat: balancePreview helper mirroring Rust snapshot math"
```

---

### Task 12: Wire the balance-update switch into the form

**Files:**
- Modify: `src/components/TransactionForm.vue`

- [ ] **Step 1: Add the switch + preview**

In `src/components/TransactionForm.vue` `<script setup>`, add imports and state:

```ts
import { balancePreview } from '../lib/transactions/balancePreview'
import { isInvestment } from '../lib/accountTypes'
```

Add a reactive field `updateBalance` to the `form` object (default computed from the selected account's type). Add after the `form` definition:

```ts
// Default the switch on for cash/liability accounts, off for investment accounts.
function defaultUpdateBalance(accountId: number | undefined): boolean {
  if (accountId == null) return false
  const acct = accountsStore.accounts.find((a) => a.id === accountId)
  return acct ? !isInvestment(acct.type) : false
}
const updateBalance = ref(false)
watch(() => form.accountId, (id) => { updateBalance.value = defaultUpdateBalance(id) })

const preview = computed(() =>
  form.accountId == null
    ? []
    : balancePreview(accountsStore.allBalances, {
        type: form.type,
        amount: form.amount || 0,
        accountId: form.accountId,
        transferAccountId: form.transferAccountId,
        date: form.date,
      }),
)

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}
function accountName(id: number): string {
  return accountsStore.accounts.find((a) => a.id === id)?.name ?? `#${id}`
}
```

> Add `import { ref } from 'vue'` to the existing `vue` import if not already present.

In both `store.update(...)` and `store.create(...)` calls inside `save()`, change `updateBalance: false` to `updateBalance: updateBalance.value`.

- [ ] **Step 2: Add the switch + preview panel to the template**

In `src/components/TransactionForm.vue`, add before the submit-button row:

```vue
    <div class="rounded border border-default p-3 space-y-2">
      <USwitch v-model="updateBalance" label="Update account balance" />
      <p class="text-xs text-muted">
        Writes a new balance snapshot reflecting this transaction, so the change shows up in your
        net-worth history. Leave off to record the transaction without touching balances.
      </p>
      <div v-if="updateBalance" class="text-sm space-y-1">
        <div v-for="line in preview" :key="line.accountId" class="tabular-nums">
          {{ accountName(line.accountId) }}: {{ money(line.from) }} → <strong>{{ money(line.to) }}</strong>
        </div>
      </div>
    </div>
```

- [ ] **Step 3: Verify it type-checks**

Run: `npx vue-tsc --noEmit`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/components/TransactionForm.vue
git commit -m "feat: balance-update switch with live preview in transaction form"
```

---

## SUB-SLICE 4 — CSV import

### Task 13: CSV parse / map / dedup pure library

**Files:**
- Modify: `package.json` (add `papaparse` + `@types/papaparse`)
- Create: `src/lib/csv/parse.ts`
- Create: `src/lib/csv/mapping.ts`
- Test: `src/lib/csv/mapping.test.ts`

- [ ] **Step 1: Install papaparse**

Run: `npm install papaparse && npm install -D @types/papaparse`
Expected: both added to `package.json`.

- [ ] **Step 2: Write the parse wrapper**

Create `src/lib/csv/parse.ts`:

```ts
import Papa from 'papaparse'

export interface ParsedCsv {
  headers: string[]
  rows: Record<string, string>[]
}

/** Parse CSV text into headers + objects keyed by header. */
export function parseCsv(text: string): ParsedCsv {
  const result = Papa.parse<Record<string, string>>(text.trim(), {
    header: true,
    skipEmptyLines: true,
  })
  const headers = result.meta.fields ?? []
  return { headers, rows: result.data }
}
```

- [ ] **Step 3: Write the failing mapping test**

Create `src/lib/csv/mapping.test.ts`:

```ts
import { describe, it, expect } from 'vitest'
import { applyMapping, detectDuplicates, type MappingConfig } from './mapping'

const config: MappingConfig = {
  dateColumn: 'Posting Date',
  amountColumn: 'Amount',
  descriptionColumn: 'Description',
  dateFormat: 'MM/dd/yyyy',
  amountSign: 'negative-is-expense',
  defaultCategory: 'uncategorized',
}

const rows = [
  { 'Posting Date': '03/01/2026', Amount: '-40.00', Description: 'Coffee' },
  { 'Posting Date': '03/02/2026', Amount: '1500.00', Description: 'Paycheck' },
]

describe('applyMapping', () => {
  it('maps rows to parsed transactions with inferred type and ISO date', () => {
    expect(applyMapping(rows, config)).toEqual([
      { date: '2026-03-01', amount: 40, description: 'Coffee', type: 'expense', category: 'uncategorized' },
      { date: '2026-03-02', amount: 1500, description: 'Paycheck', type: 'income', category: 'uncategorized' },
    ])
  })

  it('flips inference when amountSign is positive-is-expense', () => {
    const flipped = applyMapping(
      [{ 'Posting Date': '03/01/2026', Amount: '40.00', Description: 'Coffee' }],
      { ...config, amountSign: 'positive-is-expense' },
    )
    expect(flipped[0]).toMatchObject({ type: 'expense', amount: 40 })
  })
})

describe('detectDuplicates', () => {
  it('flags parsed rows matching an existing transaction on account+date+amount+description', () => {
    const parsed = applyMapping(rows, config)
    const existing = [{ accountId: 7, date: '2026-03-01', amount: 40, description: 'Coffee' }]
    const flags = detectDuplicates(parsed, existing, 7)
    expect(flags).toEqual([true, false])
  })

  it('does not flag when the account differs', () => {
    const parsed = applyMapping(rows, config)
    const existing = [{ accountId: 99, date: '2026-03-01', amount: 40, description: 'Coffee' }]
    expect(detectDuplicates(parsed, existing, 7)).toEqual([false, false])
  })
})
```

- [ ] **Step 4: Run test to verify it fails**

Run: `npm test -- mapping`
Expected: FAIL — `./mapping` not found.

- [ ] **Step 5: Implement the mapping library**

Create `src/lib/csv/mapping.ts`:

```ts
import { DateTime } from 'luxon'

export type AmountSign = 'negative-is-expense' | 'positive-is-expense'

export interface MappingConfig {
  dateColumn: string
  amountColumn: string
  descriptionColumn: string
  dateFormat: string
  amountSign: AmountSign
  defaultCategory: string
}

export interface ParsedTransaction {
  date: string
  amount: number
  description: string
  type: 'income' | 'expense'
  category: string
}

export interface ExistingRef {
  accountId: number
  date: string
  amount: number
  description: string
}

function parseAmount(raw: string): number {
  return Number(raw.replace(/[$,\s]/g, ''))
}

/** Transform raw CSV objects into parsed transactions using a mapping config. */
export function applyMapping(
  rows: Record<string, string>[],
  config: MappingConfig,
): ParsedTransaction[] {
  return rows.map((row) => {
    const signed = parseAmount(row[config.amountColumn] ?? '0')
    const isExpense =
      config.amountSign === 'negative-is-expense' ? signed < 0 : signed > 0
    const iso =
      DateTime.fromFormat(row[config.dateColumn] ?? '', config.dateFormat).toISODate() ??
      (row[config.dateColumn] ?? '')
    return {
      date: iso,
      amount: Math.abs(signed),
      description: row[config.descriptionColumn] ?? '',
      type: isExpense ? 'expense' : 'income',
      category: config.defaultCategory,
    }
  })
}

/** Return a parallel array: true where the parsed row duplicates an existing transaction. */
export function detectDuplicates(
  parsed: ParsedTransaction[],
  existing: ExistingRef[],
  accountId: number,
): boolean[] {
  const key = (date: string, amount: number, description: string) =>
    `${date}|${amount}|${description}`
  const seen = new Set(
    existing
      .filter((e) => e.accountId === accountId)
      .map((e) => key(e.date, e.amount, e.description)),
  )
  return parsed.map((p) => seen.has(key(p.date, p.amount, p.description)))
}
```

- [ ] **Step 6: Run test to verify it passes**

Run: `npm test -- mapping`
Expected: PASS (all four cases).

- [ ] **Step 7: Commit**

```bash
git add package.json package-lock.json src/lib/csv/parse.ts src/lib/csv/mapping.ts src/lib/csv/mapping.test.ts
git commit -m "feat: CSV parse + mapping + duplicate detection library"
```

---

### Task 14: Bulk-insert Rust command

**Files:**
- Modify: `src-tauri/src/commands/transactions.rs` (add `bulk_create_transactions`)
- Modify: `src-tauri/src/lib.rs` (register `bulk_create_transactions_cmd`)
- Test: `src-tauri/tests/transactions.rs` (add bulk test)

- [ ] **Step 1: Write the failing test**

Append to `src-tauri/tests/transactions.rs`:

```rust
#[tokio::test]
async fn bulk_create_writes_no_snapshots() {
    let conn = setup().await;
    let acct = accounts::create_account(&conn, &NewAccount {
        name: "Checking".into(), r#type: "checking".into(), institution: None,
        include_in_fire_calculations: false, created_at: "2026-01-01".into() }).await.unwrap();

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
```

> Note: `new_txn` sets `import_source = "manual"`. `bulk_create_transactions` MUST force `import_source = "csv"` regardless of the incoming value, so the assertion above holds.

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test --test transactions`
Expected: FAIL — `bulk_create_transactions` not found.

- [ ] **Step 3: Implement the bulk command**

Add to `src-tauri/src/commands/transactions.rs` (inner fn + wrapper):

```rust
/// Insert many transactions in one batch. Never materializes balance snapshots,
/// and forces import_source = "csv".
pub async fn bulk_create_transactions(
    conn: &Connection,
    rows: &[NewTransaction],
) -> Result<i64, String> {
    let mut count = 0i64;
    for t in rows {
        conn.execute(
            "INSERT INTO txn (account_id, transfer_account_id, amount, description, date, type, \
             category, is_contribution, import_source, generated_balance_id, \
             generated_balance_to_id, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'csv', NULL, NULL, ?9, ?9)",
            params![
                t.account_id,
                t.transfer_account_id,
                t.amount,
                t.description.clone(),
                t.date.clone(),
                t.r#type.clone(),
                t.category.clone(),
                t.is_contribution,
                t.created_at.clone()
            ],
        )
        .await
        .map_err(|e| e.to_string())?;
        count += 1;
    }
    Ok(count)
}

#[tauri::command]
pub async fn bulk_create_transactions_cmd(
    db: State<'_, Db>,
    transactions: Vec<NewTransaction>,
) -> Result<i64, String> {
    let conn = db.conn().await?;
    bulk_create_transactions(&conn, &transactions).await
}
```

Register it in `src-tauri/src/lib.rs` inside `generate_handler!`:

```rust
            commands::transactions::bulk_create_transactions_cmd,
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test --test transactions`
Expected: PASS — `bulk_create_writes_no_snapshots` plus all prior tests.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/transactions.rs src-tauri/src/lib.rs src-tauri/tests/transactions.rs
git commit -m "feat: bulk_create_transactions command (csv, no snapshots)"
```

---

### Task 15: Import wizard + saved mappings

**Files:**
- Create: `src/components/ImportWizard.vue`
- Modify: `src/lib/api/transactions.ts` (add `bulkCreateTransactions`)
- Modify: `src/pages/Transactions.vue` (add Import button + wizard modal)

- [ ] **Step 1: Add the bulk API wrapper**

Append to `src/lib/api/transactions.ts`:

```ts
export const bulkCreateTransactions = (transactions: NewTransaction[]) =>
  invoke<number>('bulk_create_transactions_cmd', { transactions })
```

- [ ] **Step 2: Build the wizard component**

Create `src/components/ImportWizard.vue`:

```vue
<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { DateTime } from 'luxon'
import { parseCsv } from '../lib/csv/parse'
import { applyMapping, detectDuplicates, type MappingConfig } from '../lib/csv/mapping'
import { bulkCreateTransactions } from '../lib/api/transactions'
import * as mappingApi from '../lib/api/importMappings'
import { useAccountsStore } from '../stores/accounts'
import { useTransactionsStore } from '../stores/transactions'
import type { ImportMapping } from '../lib/types/ImportMapping'

const emit = defineEmits<{ done: [] }>()
const accountsStore = useAccountsStore()
const txnStore = useTransactionsStore()

const step = ref<1 | 2 | 3>(1)
const accountId = ref<number | undefined>(undefined)
const headers = ref<string[]>([])
const rawRows = ref<Record<string, string>[]>([])
const savedMappings = ref<ImportMapping[]>([])
const newMappingName = ref('')

const config = ref<MappingConfig>({
  dateColumn: '',
  amountColumn: '',
  descriptionColumn: '',
  dateFormat: 'MM/dd/yyyy',
  amountSign: 'negative-is-expense',
  defaultCategory: 'uncategorized',
})

const headerItems = computed(() => headers.value.map((h) => ({ label: h, value: h })))

const parsed = computed(() =>
  step.value === 3 ? applyMapping(rawRows.value, config.value) : [],
)
const dupes = computed(() =>
  accountId.value == null
    ? []
    : detectDuplicates(
        parsed.value,
        txnStore.page.rows.map((r) => ({
          accountId: r.accountId, date: r.date, amount: r.amount, description: r.description,
        })),
        accountId.value,
      ),
)
const include = ref<boolean[]>([])

onMounted(async () => {
  await accountsStore.load()
  savedMappings.value = await mappingApi.listImportMappings()
})

async function onFile(event: Event) {
  const file = (event.target as HTMLInputElement).files?.[0]
  if (!file) return
  const text = await file.text()
  const result = parseCsv(text)
  headers.value = result.headers
  rawRows.value = result.rows
  step.value = 2
}

function applySavedMapping(m: ImportMapping) {
  config.value = { ...config.value, ...JSON.parse(m.config) }
}

function goToPreview() {
  // default-uncheck duplicates
  step.value = 3
  include.value = parsed.value.map((_, i) => !dupes.value[i])
}

async function saveMapping() {
  if (!newMappingName.value) return
  await mappingApi.createImportMapping({
    name: newMappingName.value,
    config: JSON.stringify(config.value),
    createdAt: DateTime.now().toISO()!,
  })
  savedMappings.value = await mappingApi.listImportMappings()
  newMappingName.value = ''
}

async function confirmImport() {
  if (accountId.value == null) return
  const now = DateTime.now().toISO()!
  const toInsert = parsed.value
    .filter((_, i) => include.value[i])
    .map((p) => ({
      accountId: accountId.value!,
      transferAccountId: null,
      amount: p.amount,
      description: p.description,
      date: p.date,
      type: p.type,
      category: p.category,
      isContribution: false,
      importSource: 'csv',
      updateBalance: false,
      createdAt: now,
    }))
  await bulkCreateTransactions(toInsert)
  await txnStore.load()
  emit('done')
}
</script>

<template>
  <div class="space-y-4">
    <!-- Step 1: file + account -->
    <div v-if="step === 1" class="space-y-3">
      <USelect
        v-model="accountId"
        :items="accountsStore.accounts.map((a) => ({ label: a.name, value: a.id }))"
        placeholder="Destination account"
      />
      <input type="file" accept=".csv" :disabled="accountId == null" @change="onFile" />
      <div v-if="savedMappings.length" class="text-sm">
        <p class="text-muted mb-1">Saved mappings:</p>
        <UButton v-for="m in savedMappings" :key="m.id" size="xs" variant="soft"
          class="mr-1" @click="applySavedMapping(m)">{{ m.name }}</UButton>
      </div>
    </div>

    <!-- Step 2: map columns -->
    <div v-else-if="step === 2" class="space-y-3">
      <USelect v-model="config.dateColumn" :items="headerItems" placeholder="Date column" />
      <USelect v-model="config.amountColumn" :items="headerItems" placeholder="Amount column" />
      <USelect v-model="config.descriptionColumn" :items="headerItems" placeholder="Description column" />
      <UInput v-model="config.dateFormat" placeholder="Date format (e.g. MM/dd/yyyy)" />
      <USelect
        v-model="config.amountSign"
        :items="[
          { label: 'Negative amounts are expenses', value: 'negative-is-expense' },
          { label: 'Positive amounts are expenses', value: 'positive-is-expense' },
        ]"
      />
      <div class="flex gap-2 items-center">
        <UInput v-model="newMappingName" placeholder="Save this mapping as…" class="w-52" />
        <UButton size="sm" variant="soft" :disabled="!newMappingName" @click="saveMapping">Save mapping</UButton>
      </div>
      <div class="flex justify-end">
        <UButton :disabled="!config.dateColumn || !config.amountColumn" @click="goToPreview">Preview</UButton>
      </div>
    </div>

    <!-- Step 3: preview + dedup -->
    <div v-else class="space-y-3">
      <p class="text-sm text-muted">
        {{ include.filter(Boolean).length }} of {{ parsed.length }} rows selected
        ({{ dupes.filter(Boolean).length }} possible duplicates unchecked).
      </p>
      <table class="w-full text-sm">
        <thead class="text-left text-muted border-b border-default">
          <tr><th></th><th>Date</th><th>Description</th><th>Type</th><th class="text-right">Amount</th></tr>
        </thead>
        <tbody>
          <tr v-for="(p, i) in parsed" :key="i" class="border-b border-default/50"
            :class="{ 'opacity-50': dupes[i] }">
            <td><UCheckbox v-model="include[i]" /></td>
            <td>{{ p.date }}</td>
            <td>{{ p.description }} <span v-if="dupes[i]" class="text-xs text-amber-600">(dup)</span></td>
            <td>{{ p.type }}</td>
            <td class="text-right tabular-nums">{{ p.amount }}</td>
          </tr>
        </tbody>
      </table>
      <div class="flex justify-end">
        <UButton :disabled="!include.some(Boolean)" @click="confirmImport">Import selected</UButton>
      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 3: Add the Import button + modal to the page**

In `src/pages/Transactions.vue` `<script setup>`, add:

```ts
import ImportWizard from '../components/ImportWizard.vue'
const isImportOpen = ref(false)
function onImportDone() { isImportOpen.value = false }
```

In the header button row, add an Import button next to "Add transaction":

```vue
      <div class="flex gap-2">
        <UButton variant="soft" icon="i-lucide-upload" @click="isImportOpen = true">Import CSV</UButton>
        <UButton icon="i-lucide-plus" @click="openAdd">Add transaction</UButton>
      </div>
```

Add the wizard modal near the form modal:

```vue
    <UModal v-model:open="isImportOpen" title="Import transactions from CSV">
      <template #body>
        <ImportWizard @done="onImportDone" />
      </template>
    </UModal>
```

- [ ] **Step 4: Verify it type-checks + full test suite**

Run: `npx vue-tsc --noEmit && npm test && cd src-tauri && cargo test`
Expected: type-check clean; all vitest + all cargo tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/components/ImportWizard.vue src/lib/api/transactions.ts src/pages/Transactions.vue
git commit -m "feat: CSV import wizard with saved mappings + dedup"
```

---

## Final verification

### Task 16: Full build + manual GUI smoke test

- [ ] **Step 1: Full automated suite**

Run: `npm test && npx vue-tsc --noEmit && cd src-tauri && cargo test && cargo build`
Expected: all green.

- [ ] **Step 2: Manual GUI smoke test** (cannot run headless)

Run: `npm run tauri dev`
Verify:
1. **Transactions** nav item is enabled and routes to the page.
2. Add an income, an expense, and a transfer manually; they appear in the table; income/expense/net totals are correct; the transfer is excluded from totals.
3. On an account with an existing balance, add an expense with **Update balance ON** — the preview shows `from → to`, and the Accounts page / Dashboard net-worth reflects the new snapshot. Edit its amount; the snapshot updates. Delete it; the snapshot is removed.
4. Filter by account/type/category and search by description.
5. **Import CSV**: pick an account, drop a small bank CSV, map columns, save the mapping, preview (duplicates unchecked), import. Rows appear with `csv` source and no new snapshots. Re-open import, re-select the saved mapping by name, and confirm dupes are flagged.

- [ ] **Step 3: Update the project memory**

Update `project-trackmyfi-design` memory: mark Phase 2a (Transactions) built, note the `txn` table name (SQLite keyword workaround), the linked-snapshot mechanism, and that 2b–2d remain.

- [ ] **Step 4: Finish the branch**

Use the `superpowers:finishing-a-development-branch` skill to decide merge/PR.

---

## Self-review notes (coverage check)

- **Informational ledger / no implicit balance change** → Tasks 3, 9 (switch defaults false on manual create without opt-in).
- **Opt-in balance switch + linked snapshots (create/edit/delete)** → Tasks 10 (Rust), 11–12 (preview + UI).
- **Default on for cash/liability, off for investment** → Task 12 (`defaultUpdateBalance` via `isInvestment`).
- **Single-row transfers, two snapshots, excluded from totals** → Tasks 3 (totals), 10 (two snapshots).
- **Signed-delta convention** → Task 6 (`signedDelta`), mirrored in Rust Task 10.
- **CSV generic mapping + saved mappings + dedup + defaults; no snapshots on import** → Tasks 13, 14, 15.
- **Fixed category enum** → Task 6 constants.
- **List workbench: filters, search, totals, edit/delete** → Tasks 8, 9.
- **Schema + migration** → Task 1. **Commands registered** → Tasks 5, 14.
