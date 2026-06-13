# Phase 2d: Anti-Budget Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Anti-Budget monthly view — a horizontal formula row (Income − Savings − Fixed = Free Money) with an expandable transaction detail panel, optional per-month savings target with most-recent fallback, and paycheck → income transaction auto-creation.

**Architecture:** New `budget_month` SQL table stores optional per-month savings targets; a new `income_account_id` column on `paycheck` triggers auto-creation of a linked income transaction on save (same pattern as deduction→contribution). Four new Rust commands expose budget data; pure-TS `buildBudgetMonth` partitions transactions into line items; `Budget.vue` renders the horizontal formula with a swappable detail panel below.

**Tech Stack:** Rust/libSQL (commands), Vue 3 + NuxtUI + Pinia (frontend), Vitest (TS tests), `#[tokio::test]` (Rust tests), Luxon (date formatting), ts-rs (type generation).

---

### Task 1: Migration 0005

**Files:**
- Create: `src-tauri/migrations/0005_budget.sql`
- Modify: `src-tauri/src/migrations.rs`

- [ ] **Step 1: Write the SQL migration**

```sql
-- src-tauri/migrations/0005_budget.sql
CREATE TABLE budget_month (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  year           INTEGER NOT NULL,
  month          INTEGER NOT NULL,
  savings_target REAL    NOT NULL,
  UNIQUE(year, month)
);

ALTER TABLE paycheck ADD COLUMN income_account_id INTEGER REFERENCES account(id);
```

- [ ] **Step 2: Register the migration in migrations.rs**

Add a new entry to the `MIGRATIONS` array (after the `hsa_coverage` entry):

```rust
Migration {
    version: 5,
    name: "budget",
    sql: include_str!("../migrations/0005_budget.sql"),
},
```

- [ ] **Step 3: Run cargo build to verify the migration compiles**

```bash
cd src-tauri && cargo build 2>&1 | grep -E "^error"
```

Expected: no output (no errors).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/migrations/0005_budget.sql src-tauri/src/migrations.rs
git commit -m "feat: migration 0005 — budget_month table + paycheck.income_account_id"
```

---

### Task 2: Rust budget commands

**Files:**
- Create: `src-tauri/src/commands/budget.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing Rust tests**

Create `src-tauri/src/commands/budget.rs` with the test module first:

```rust
use crate::db::Db;
use crate::models::Transaction;
use libsql::{params, Connection};
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetMonth {
    pub year: i32,
    pub month: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetMonthTarget {
    pub savings_target: f64,
    pub source_year: i32,
    pub source_month: i32,
}

const TXN_COLS: &str = "id, account_id, transfer_account_id, amount, description, date, type, \
    category, is_contribution, import_source, generated_balance_id, generated_balance_to_id, \
    paycheck_id, created_at, updated_at";

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
        paycheck_id: row.get(12).map_err(|e| e.to_string())?,
        created_at: row.get(13).map_err(|e| e.to_string())?,
        updated_at: row.get(14).map_err(|e| e.to_string())?,
    })
}

pub async fn list_budget_months(conn: &Connection) -> Result<Vec<BudgetMonth>, String> {
    todo!()
}

pub async fn list_budget_txns(conn: &Connection, year: i32, month: i32) -> Result<Vec<Transaction>, String> {
    todo!()
}

pub async fn get_budget_month_target(conn: &Connection, year: i32, month: i32) -> Result<Option<BudgetMonthTarget>, String> {
    todo!()
}

pub async fn set_budget_month_target(conn: &Connection, year: i32, month: i32, savings_target: f64) -> Result<(), String> {
    todo!()
}

// ---- thin command wrappers ----

#[tauri::command]
pub async fn list_budget_months_cmd(db: State<'_, Db>) -> Result<Vec<BudgetMonth>, String> {
    let conn = db.conn().await?;
    list_budget_months(&conn).await
}

#[tauri::command]
pub async fn list_budget_txns_cmd(db: State<'_, Db>, year: i32, month: i32) -> Result<Vec<Transaction>, String> {
    let conn = db.conn().await?;
    list_budget_txns(&conn, year, month).await
}

#[tauri::command]
pub async fn get_budget_month_target_cmd(db: State<'_, Db>, year: i32, month: i32) -> Result<Option<BudgetMonthTarget>, String> {
    let conn = db.conn().await?;
    get_budget_month_target(&conn, year, month).await
}

#[tauri::command]
pub async fn set_budget_month_target_cmd(db: State<'_, Db>, year: i32, month: i32, savings_target: f64) -> Result<(), String> {
    let conn = db.conn().await?;
    set_budget_month_target(&conn, year, month, savings_target).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use libsql::Builder;

    async fn test_conn() -> Connection {
        let db = Builder::new_local(":memory:").build().await.unwrap();
        let conn = db.connect().unwrap();
        crate::migrations::run(&conn).await.unwrap();
        conn
    }

    async fn seed_txn(conn: &Connection, date: &str) {
        conn.execute(
            "INSERT INTO account (name, type, is_active, include_in_fire_calculations, created_at) \
             VALUES ('Checking', 'checking', 1, 0, '2025-01-01')",
            (),
        ).await.unwrap();
        let acct_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO txn (account_id, transfer_account_id, amount, description, date, \
             type, category, is_contribution, import_source, generated_balance_id, \
             generated_balance_to_id, paycheck_id, created_at, updated_at) \
             VALUES (?1, NULL, 100.0, 'Test', ?2, 'income', 'fixed', 0, 'manual', \
             NULL, NULL, NULL, ?2, ?2)",
            params![acct_id, date],
        ).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_budget_month_target_none() {
        let conn = test_conn().await;
        let result = get_budget_month_target(&conn, 2025, 6).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_budget_month_target_exact() {
        let conn = test_conn().await;
        set_budget_month_target(&conn, 2025, 6, 2000.0).await.unwrap();
        let result = get_budget_month_target(&conn, 2025, 6).await.unwrap().unwrap();
        assert_eq!(result.savings_target, 2000.0);
        assert_eq!(result.source_year, 2025);
        assert_eq!(result.source_month, 6);
    }

    #[tokio::test]
    async fn test_get_budget_month_target_fallback() {
        let conn = test_conn().await;
        set_budget_month_target(&conn, 2025, 1, 1500.0).await.unwrap();
        // Request June 2025 — no record for that month, should fall back to Jan 2025
        let result = get_budget_month_target(&conn, 2025, 6).await.unwrap().unwrap();
        assert_eq!(result.savings_target, 1500.0);
        assert_eq!(result.source_year, 2025);
        assert_eq!(result.source_month, 1);
    }

    #[tokio::test]
    async fn test_get_budget_month_target_no_future_fallback() {
        let conn = test_conn().await;
        // A record exists for July — requesting June should NOT use July
        set_budget_month_target(&conn, 2025, 7, 1500.0).await.unwrap();
        let result = get_budget_month_target(&conn, 2025, 6).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_set_budget_month_target_upsert() {
        let conn = test_conn().await;
        set_budget_month_target(&conn, 2025, 6, 2000.0).await.unwrap();
        set_budget_month_target(&conn, 2025, 6, 2500.0).await.unwrap();
        let result = get_budget_month_target(&conn, 2025, 6).await.unwrap().unwrap();
        assert_eq!(result.savings_target, 2500.0);
    }

    #[tokio::test]
    async fn test_list_budget_months() {
        let conn = test_conn().await;
        seed_txn(&conn, "2025-06-15").await;
        seed_txn(&conn, "2025-06-20").await;
        seed_txn(&conn, "2025-05-10").await;
        let months = list_budget_months(&conn).await.unwrap();
        assert_eq!(months.len(), 2);
        assert_eq!(months[0].year, 2025);
        assert_eq!(months[0].month, 6);
        assert_eq!(months[1].year, 2025);
        assert_eq!(months[1].month, 5);
    }

    #[tokio::test]
    async fn test_list_budget_txns() {
        let conn = test_conn().await;
        seed_txn(&conn, "2025-06-15").await;
        seed_txn(&conn, "2025-07-01").await;
        let txns = list_budget_txns(&conn, 2025, 6).await.unwrap();
        assert_eq!(txns.len(), 1);
        assert!(txns[0].date.starts_with("2025-06"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail with `todo!()`**

```bash
cd src-tauri && cargo test budget 2>&1 | grep -E "FAILED|panicked|todo"
```

Expected: tests fail with "not yet implemented" panics.

- [ ] **Step 3: Implement the four inner functions**

Replace the `todo!()` bodies in `budget.rs`:

```rust
pub async fn list_budget_months(conn: &Connection) -> Result<Vec<BudgetMonth>, String> {
    let mut rows = conn
        .query(
            "SELECT DISTINCT CAST(strftime('%Y', date) AS INTEGER) AS year, \
             CAST(strftime('%m', date) AS INTEGER) AS month \
             FROM txn ORDER BY year DESC, month DESC",
            (),
        )
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(BudgetMonth {
            year: row.get(0).map_err(|e| e.to_string())?,
            month: row.get(1).map_err(|e| e.to_string())?,
        });
    }
    Ok(out)
}

pub async fn list_budget_txns(conn: &Connection, year: i32, month: i32) -> Result<Vec<Transaction>, String> {
    let sql = format!(
        "SELECT {TXN_COLS} FROM txn \
         WHERE strftime('%Y', date) = printf('%04d', ?1) \
           AND strftime('%m', date) = printf('%02d', ?2) \
         ORDER BY date ASC, id ASC"
    );
    let mut rows = conn
        .query(&sql, params![year, month])
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(row_to_txn(&row)?);
    }
    Ok(out)
}

pub async fn get_budget_month_target(conn: &Connection, year: i32, month: i32) -> Result<Option<BudgetMonthTarget>, String> {
    let mut rows = conn
        .query(
            "SELECT savings_target, year, month FROM budget_month \
             WHERE (year < ?1 OR (year = ?1 AND month <= ?2)) \
             ORDER BY year DESC, month DESC LIMIT 1",
            params![year, month],
        )
        .await
        .map_err(|e| e.to_string())?;
    match rows.next().await.map_err(|e| e.to_string())? {
        Some(row) => Ok(Some(BudgetMonthTarget {
            savings_target: row.get(0).map_err(|e| e.to_string())?,
            source_year: row.get(1).map_err(|e| e.to_string())?,
            source_month: row.get(2).map_err(|e| e.to_string())?,
        })),
        None => Ok(None),
    }
}

pub async fn set_budget_month_target(conn: &Connection, year: i32, month: i32, savings_target: f64) -> Result<(), String> {
    conn.execute(
        "INSERT INTO budget_month (year, month, savings_target) VALUES (?1, ?2, ?3) \
         ON CONFLICT(year, month) DO UPDATE SET savings_target = excluded.savings_target",
        params![year, month, savings_target],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd src-tauri && cargo test budget 2>&1 | grep -E "test .* (ok|FAILED)"
```

Expected: all 7 `budget::tests::*` tests show `ok`.

- [ ] **Step 5: Register in mod.rs and lib.rs**

In `src-tauri/src/commands/mod.rs`, add:
```rust
pub mod budget;
```

In `src-tauri/src/lib.rs`, add the four commands to `invoke_handler!`:
```rust
commands::budget::list_budget_months_cmd,
commands::budget::list_budget_txns_cmd,
commands::budget::get_budget_month_target_cmd,
commands::budget::set_budget_month_target_cmd,
```

- [ ] **Step 6: Verify it builds**

```bash
cd src-tauri && cargo build 2>&1 | grep -E "^error"
```

Expected: no output.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands/budget.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat: Rust budget commands with tests (list months/txns, get/set target)"
```

---

### Task 3: Update Paycheck — income_account_id + auto-create income transaction

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/commands/paychecks.rs`

- [ ] **Step 1: Add `income_account_id` to the Paycheck model in models.rs**

In `src-tauri/src/models.rs`, add `income_account_id` to the `Paycheck` struct (before `import_source`):

```rust
#[derive(Serialize, Deserialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct Paycheck {
    pub id: i32,
    pub pay_date: String,
    pub employer: String,
    pub pay_period: String,
    pub gross_amount: f64,
    pub net_amount: f64,
    pub federal_tax: f64,
    pub state_tax: f64,
    pub local_tax: f64,
    pub social_security_tax: f64,
    pub medicare_tax: f64,
    pub deductions: Vec<PaycheckDeduction>,
    pub employer_match: Vec<EmployerMatchItem>,
    pub income_account_id: Option<i32>,
    pub import_source: String,
    pub created_at: String,
    pub updated_at: String,
}
```

- [ ] **Step 2: Update paychecks.rs — COLS, row_to_paycheck, NewPaycheck, UpdatePaycheck**

Update `COLS` in `src-tauri/src/commands/paychecks.rs` to include `income_account_id` (after `employer_match`, before `import_source`):

```rust
const COLS: &str = "id, pay_date, employer, pay_period, gross_amount, net_amount, \
    federal_tax, state_tax, local_tax, social_security_tax, medicare_tax, \
    deductions, employer_match, income_account_id, import_source, created_at, updated_at";
```

Update `row_to_paycheck` — `income_account_id` is now index 13, `import_source` is 14, `created_at` is 15, `updated_at` is 16:

```rust
fn row_to_paycheck(row: &libsql::Row) -> Result<Paycheck, String> {
    let deductions_json: String = row.get(11).map_err(|e| e.to_string())?;
    let employer_match_json: String = row.get(12).map_err(|e| e.to_string())?;
    Ok(Paycheck {
        id: row.get(0).map_err(|e| e.to_string())?,
        pay_date: row.get(1).map_err(|e| e.to_string())?,
        employer: row.get(2).map_err(|e| e.to_string())?,
        pay_period: row.get(3).map_err(|e| e.to_string())?,
        gross_amount: row.get(4).map_err(|e| e.to_string())?,
        net_amount: row.get(5).map_err(|e| e.to_string())?,
        federal_tax: row.get(6).map_err(|e| e.to_string())?,
        state_tax: row.get(7).map_err(|e| e.to_string())?,
        local_tax: row.get(8).map_err(|e| e.to_string())?,
        social_security_tax: row.get(9).map_err(|e| e.to_string())?,
        medicare_tax: row.get(10).map_err(|e| e.to_string())?,
        deductions: serde_json::from_str(&deductions_json).map_err(|e| e.to_string())?,
        employer_match: serde_json::from_str(&employer_match_json).map_err(|e| e.to_string())?,
        income_account_id: row.get(13).map_err(|e| e.to_string())?,
        import_source: row.get(14).map_err(|e| e.to_string())?,
        created_at: row.get(15).map_err(|e| e.to_string())?,
        updated_at: row.get(16).map_err(|e| e.to_string())?,
    })
}
```

Add `income_account_id` to `NewPaycheck` and `UpdatePaycheck`:

```rust
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewPaycheck {
    pub pay_date: String,
    pub employer: String,
    pub pay_period: String,
    pub gross_amount: f64,
    pub net_amount: f64,
    pub federal_tax: f64,
    pub state_tax: f64,
    pub local_tax: f64,
    pub social_security_tax: f64,
    pub medicare_tax: f64,
    pub deductions: Vec<PaycheckDeduction>,
    pub employer_match: Vec<EmployerMatchItem>,
    pub income_account_id: Option<i32>,
    pub created_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePaycheck {
    pub id: i32,
    pub pay_date: String,
    pub employer: String,
    pub pay_period: String,
    pub gross_amount: f64,
    pub net_amount: f64,
    pub federal_tax: f64,
    pub state_tax: f64,
    pub local_tax: f64,
    pub social_security_tax: f64,
    pub medicare_tax: f64,
    pub deductions: Vec<PaycheckDeduction>,
    pub employer_match: Vec<EmployerMatchItem>,
    pub income_account_id: Option<i32>,
    pub updated_at: String,
}
```

- [ ] **Step 3: Add auto_create_income_txn helper**

Add this function to `paychecks.rs` (after `auto_create_contributions`):

```rust
async fn auto_create_income_txn(
    conn: &Connection,
    paycheck_id: i32,
    income_account_id: Option<i32>,
    net_amount: f64,
    employer: &str,
    pay_date: &str,
    now: &str,
) -> Result<(), String> {
    let Some(account_id) = income_account_id else {
        return Ok(());
    };
    let description = format!("Paycheck – {}", employer);
    conn.execute(
        "INSERT INTO txn (account_id, transfer_account_id, amount, description, date, \
         type, category, is_contribution, import_source, paycheck_id, \
         generated_balance_id, generated_balance_to_id, created_at, updated_at) \
         VALUES (?1, NULL, ?2, ?3, ?4, 'income', 'fixed', 0, 'paycheck', ?5, \
         NULL, NULL, ?6, ?6)",
        params![account_id, net_amount, description, pay_date, paycheck_id, now],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}
```

- [ ] **Step 4: Update create_paycheck to include income_account_id and call auto_create_income_txn**

Replace the `create_paycheck` function body:

```rust
pub async fn create_paycheck(conn: &Connection, p: &NewPaycheck) -> Result<Paycheck, String> {
    let deductions_json = serde_json::to_string(&p.deductions).map_err(|e| e.to_string())?;
    let employer_match_json = serde_json::to_string(&p.employer_match).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO paycheck (pay_date, employer, pay_period, gross_amount, net_amount, \
         federal_tax, state_tax, local_tax, social_security_tax, medicare_tax, \
         deductions, employer_match, income_account_id, import_source, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 'manual', ?14, ?14)",
        params![
            p.pay_date.clone(), p.employer.clone(), p.pay_period.clone(),
            p.gross_amount, p.net_amount,
            p.federal_tax, p.state_tax, p.local_tax, p.social_security_tax, p.medicare_tax,
            deductions_json, employer_match_json, p.income_account_id, p.created_at.clone()
        ],
    )
    .await
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid() as i32;
    auto_create_contributions(conn, id, &p.pay_date, &p.deductions, &p.employer_match, &p.created_at).await?;
    auto_create_income_txn(conn, id, p.income_account_id, p.net_amount, &p.employer, &p.pay_date, &p.created_at).await?;
    get_paycheck(conn, id).await
}
```

- [ ] **Step 5: Update update_paycheck to include income_account_id and call auto_create_income_txn**

Replace the `update_paycheck` function body:

```rust
pub async fn update_paycheck(conn: &Connection, p: &UpdatePaycheck) -> Result<Paycheck, String> {
    get_paycheck(conn, p.id).await?;

    // Delete ALL txns previously created by this paycheck (contributions + income)
    conn.execute("DELETE FROM txn WHERE paycheck_id = ?1", params![p.id])
        .await
        .map_err(|e| e.to_string())?;

    let deductions_json = serde_json::to_string(&p.deductions).map_err(|e| e.to_string())?;
    let employer_match_json = serde_json::to_string(&p.employer_match).map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE paycheck SET pay_date=?1, employer=?2, pay_period=?3, gross_amount=?4, \
         net_amount=?5, federal_tax=?6, state_tax=?7, local_tax=?8, social_security_tax=?9, \
         medicare_tax=?10, deductions=?11, employer_match=?12, income_account_id=?13, \
         updated_at=?14 WHERE id=?15",
        params![
            p.pay_date.clone(), p.employer.clone(), p.pay_period.clone(),
            p.gross_amount, p.net_amount,
            p.federal_tax, p.state_tax, p.local_tax, p.social_security_tax, p.medicare_tax,
            deductions_json, employer_match_json, p.income_account_id, p.updated_at.clone(), p.id
        ],
    )
    .await
    .map_err(|e| e.to_string())?;

    auto_create_contributions(conn, p.id, &p.pay_date, &p.deductions, &p.employer_match, &p.updated_at).await?;
    auto_create_income_txn(conn, p.id, p.income_account_id, p.net_amount, &p.employer, &p.pay_date, &p.updated_at).await?;
    get_paycheck(conn, p.id).await
}
```

- [ ] **Step 6: Verify it builds**

```bash
cd src-tauri && cargo build 2>&1 | grep -E "^error"
```

Expected: no output.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/commands/paychecks.rs
git commit -m "feat: paycheck income_account_id + auto-create income transaction on save"
```

---

### Task 4: Regenerate TypeScript types

**Files:**
- Modify: `src/lib/types/Paycheck.ts` (auto-generated by ts-rs)

- [ ] **Step 1: Run cargo test to trigger ts-rs type export**

```bash
cd src-tauri && cargo test 2>&1 | grep -E "test .* (ok|FAILED)|error\[" | head -30
```

Expected: all tests pass (including the 7 budget tests from Task 2). ts-rs writes `src/lib/types/Paycheck.ts` with the new `incomeAccountId` field.

- [ ] **Step 2: Verify Paycheck.ts was updated**

```bash
grep "incomeAccountId" src/lib/types/Paycheck.ts
```

Expected: `incomeAccountId: number | null,`

- [ ] **Step 3: Commit**

```bash
git add src/lib/types/Paycheck.ts
git commit -m "chore: regenerate Paycheck type with incomeAccountId"
```

---

### Task 5: TypeScript budget lib + unit tests

**Files:**
- Create: `src/lib/budget/index.ts`
- Create: `src/lib/budget/index.test.ts`

- [ ] **Step 1: Write the failing unit tests**

Create `src/lib/budget/index.test.ts`:

```ts
import { describe, it, expect } from 'vitest'
import { buildBudgetMonth } from './index'
import type { Transaction } from '../types/Transaction'

function txn(overrides: Partial<Transaction> & { id: number }): Transaction {
  return {
    id: overrides.id,
    accountId: 1,
    transferAccountId: null,
    amount: overrides.amount ?? 100,
    description: 'Test',
    date: overrides.date ?? '2025-06-01',
    type: overrides.type ?? 'expense',
    category: overrides.category ?? 'discretionary',
    isContribution: overrides.isContribution ?? false,
    importSource: 'manual',
    generatedBalanceId: null,
    generatedBalanceToId: null,
    paycheckId: null,
    createdAt: '2025-06-01',
    updatedAt: '2025-06-01',
    ...overrides,
  }
}

describe('buildBudgetMonth', () => {
  it('buckets income transactions (type=income, not contribution)', () => {
    const txns = [txn({ id: 1, type: 'income', category: 'fixed', isContribution: false, amount: 3000 })]
    const result = buildBudgetMonth(txns)
    expect(result.income.total).toBe(3000)
    expect(result.income.transactions).toHaveLength(1)
    expect(result.savings.total).toBe(0)
  })

  it('buckets contribution transactions into savings regardless of type', () => {
    const txns = [txn({ id: 1, type: 'income', category: 'savings', isContribution: true, amount: 500 })]
    const result = buildBudgetMonth(txns)
    expect(result.savings.total).toBe(500)
    expect(result.income.total).toBe(0)
  })

  it('does not double-count contributions as income', () => {
    const txns = [
      txn({ id: 1, type: 'income', isContribution: true, amount: 500 }),
      txn({ id: 2, type: 'income', isContribution: false, amount: 3000 }),
    ]
    const result = buildBudgetMonth(txns)
    expect(result.income.total).toBe(3000)
    expect(result.savings.total).toBe(500)
  })

  it('buckets fixed expenses', () => {
    const txns = [txn({ id: 1, type: 'expense', category: 'fixed', amount: 1200 })]
    const result = buildBudgetMonth(txns)
    expect(result.fixed.total).toBe(1200)
    expect(result.fixed.transactions).toHaveLength(1)
  })

  it('buckets discretionary expenses', () => {
    const txns = [txn({ id: 1, type: 'expense', category: 'discretionary', amount: 80 })]
    const result = buildBudgetMonth(txns)
    expect(result.discretionary.total).toBe(80)
  })

  it('computes freeMoney = income - savings - fixed', () => {
    const txns = [
      txn({ id: 1, type: 'income', isContribution: false, amount: 6000 }),
      txn({ id: 2, type: 'income', isContribution: true, amount: 1000 }),
      txn({ id: 3, type: 'expense', category: 'fixed', amount: 1500 }),
    ]
    const result = buildBudgetMonth(txns)
    expect(result.freeMoney).toBe(3500) // 6000 - 1000 - 1500
  })

  it('computes freeMoneyRemaining = freeMoney - discretionary', () => {
    const txns = [
      txn({ id: 1, type: 'income', isContribution: false, amount: 6000 }),
      txn({ id: 2, type: 'income', isContribution: true, amount: 1000 }),
      txn({ id: 3, type: 'expense', category: 'fixed', amount: 1500 }),
      txn({ id: 4, type: 'expense', category: 'discretionary', amount: 800 }),
    ]
    const result = buildBudgetMonth(txns)
    expect(result.freeMoneyRemaining).toBe(2700) // 3500 - 800
  })

  it('returns zero totals and empty arrays for categories with no transactions', () => {
    const result = buildBudgetMonth([])
    expect(result.income.total).toBe(0)
    expect(result.income.transactions).toHaveLength(0)
    expect(result.savings.total).toBe(0)
    expect(result.fixed.total).toBe(0)
    expect(result.discretionary.total).toBe(0)
    expect(result.freeMoney).toBe(0)
    expect(result.freeMoneyRemaining).toBe(0)
  })

  it('excludes transfer transactions from all buckets', () => {
    const txns = [txn({ id: 1, type: 'transfer', amount: 500 })]
    const result = buildBudgetMonth(txns)
    expect(result.income.total).toBe(0)
    expect(result.savings.total).toBe(0)
    expect(result.fixed.total).toBe(0)
    expect(result.discretionary.total).toBe(0)
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
npx vitest run src/lib/budget/index.test.ts 2>&1 | tail -10
```

Expected: error — `src/lib/budget/index.ts` not found.

- [ ] **Step 3: Implement buildBudgetMonth**

Create `src/lib/budget/index.ts`:

```ts
import type { Transaction } from '../types/Transaction'

export type BudgetLineItem = {
  total: number
  transactions: Transaction[]
}

export type BudgetMonthSummary = {
  income: BudgetLineItem
  savings: BudgetLineItem
  fixed: BudgetLineItem
  discretionary: BudgetLineItem
  freeMoney: number
  freeMoneyRemaining: number
}

export type BudgetMonthTarget = {
  savingsTarget: number
  sourceYear: number
  sourceMonth: number
  isInherited: boolean
}

export function buildBudgetMonth(txns: Transaction[]): BudgetMonthSummary {
  const incomeTxns = txns.filter((t) => t.type === 'income' && !t.isContribution)
  const savingsTxns = txns.filter((t) => t.isContribution)
  const fixedTxns = txns.filter((t) => t.type === 'expense' && t.category === 'fixed')
  const discretionaryTxns = txns.filter((t) => t.type === 'expense' && t.category === 'discretionary')

  const sum = (arr: Transaction[]) => arr.reduce((s, t) => s + t.amount, 0)

  const incomeTotal = sum(incomeTxns)
  const savingsTotal = sum(savingsTxns)
  const fixedTotal = sum(fixedTxns)
  const discretionaryTotal = sum(discretionaryTxns)
  const freeMoney = incomeTotal - savingsTotal - fixedTotal

  return {
    income: { total: incomeTotal, transactions: incomeTxns },
    savings: { total: savingsTotal, transactions: savingsTxns },
    fixed: { total: fixedTotal, transactions: fixedTxns },
    discretionary: { total: discretionaryTotal, transactions: discretionaryTxns },
    freeMoney,
    freeMoneyRemaining: freeMoney - discretionaryTotal,
  }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
npx vitest run src/lib/budget/index.test.ts 2>&1 | tail -10
```

Expected: all 8 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/lib/budget/index.ts src/lib/budget/index.test.ts
git commit -m "feat: TS budget lib — buildBudgetMonth with unit tests"
```

---

### Task 6: TS budget API + Pinia store

**Files:**
- Create: `src/lib/api/budget.ts`
- Create: `src/stores/budget.ts`

- [ ] **Step 1: Create the budget API module**

Create `src/lib/api/budget.ts`:

```ts
import { invoke } from '@tauri-apps/api/core'
import type { Transaction } from '../types/Transaction'

export interface BudgetMonth {
  year: number
  month: number
}

export interface BudgetMonthTargetRaw {
  savingsTarget: number
  sourceYear: number
  sourceMonth: number
}

export const listBudgetMonths = () =>
  invoke<BudgetMonth[]>('list_budget_months_cmd')

export const listBudgetTxns = (year: number, month: number) =>
  invoke<Transaction[]>('list_budget_txns_cmd', { year, month })

export const getBudgetMonthTarget = (year: number, month: number) =>
  invoke<BudgetMonthTargetRaw | null>('get_budget_month_target_cmd', { year, month })

export const setBudgetMonthTarget = (year: number, month: number, savingsTarget: number) =>
  invoke<void>('set_budget_month_target_cmd', { year, month, savingsTarget })
```

- [ ] **Step 2: Create the budget Pinia store**

Create `src/stores/budget.ts`:

```ts
import { defineStore } from 'pinia'
import { ref } from 'vue'
import { DateTime } from 'luxon'
import * as api from '../lib/api/budget'
import { buildBudgetMonth } from '../lib/budget/index'
import type { BudgetMonthSummary, BudgetMonthTarget } from '../lib/budget/index'

export type ActiveSection = 'income' | 'savings' | 'fixed' | 'discretionary'

export const useBudgetStore = defineStore('budget', () => {
  const months = ref<api.BudgetMonth[]>([])
  const summary = ref<BudgetMonthSummary | null>(null)
  const target = ref<BudgetMonthTarget | null>(null)
  const activeSection = ref<ActiveSection>('income')

  async function loadMonths() {
    months.value = await api.listBudgetMonths()
    // Ensure the current month is always selectable even with no data
    const now = DateTime.now()
    const hasCurrentMonth = months.value.some(
      (m) => m.year === now.year && m.month === now.month,
    )
    if (!hasCurrentMonth) {
      months.value = [{ year: now.year, month: now.month }, ...months.value]
    }
  }

  async function load(year: number, month: number) {
    const [txns, rawTarget] = await Promise.all([
      api.listBudgetTxns(year, month),
      api.getBudgetMonthTarget(year, month),
    ])
    summary.value = buildBudgetMonth(txns)
    target.value = rawTarget
      ? {
          savingsTarget: rawTarget.savingsTarget,
          sourceYear: rawTarget.sourceYear,
          sourceMonth: rawTarget.sourceMonth,
          isInherited: rawTarget.sourceYear !== year || rawTarget.sourceMonth !== month,
        }
      : null
  }

  async function setTarget(year: number, month: number, savingsTarget: number) {
    await api.setBudgetMonthTarget(year, month, savingsTarget)
    const raw = await api.getBudgetMonthTarget(year, month)
    target.value = raw
      ? {
          savingsTarget: raw.savingsTarget,
          sourceYear: raw.sourceYear,
          sourceMonth: raw.sourceMonth,
          isInherited: false,
        }
      : null
  }

  return { months, summary, target, activeSection, loadMonths, load, setTarget }
})
```

- [ ] **Step 3: Verify TypeScript compiles**

```bash
npx vue-tsc --noEmit 2>&1 | head -20
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/api/budget.ts src/stores/budget.ts
git commit -m "feat: budget API module and Pinia store"
```

---

### Task 7: Budget.vue page

**Files:**
- Create: `src/pages/Budget.vue`

- [ ] **Step 1: Create Budget.vue**

Create `src/pages/Budget.vue`:

```vue
<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { DateTime } from 'luxon'
import { useBudgetStore } from '../stores/budget'
import { useAccountsStore } from '../stores/accounts'
import type { ActiveSection } from '../stores/budget'

const store = useBudgetStore()
const accountsStore = useAccountsStore()

const selectedYear = ref<number>(DateTime.now().year)
const selectedMonth = ref<number>(DateTime.now().month)
const editingTarget = ref(false)
const targetInput = ref<number>(0)

const monthItems = computed(() =>
  store.months.map((m) => ({
    label: DateTime.fromObject({ year: m.year, month: m.month }).toFormat('MMMM yyyy'),
    value: `${m.year}-${m.month}`,
  })),
)

const selectedMonthValue = computed(() => `${selectedYear.value}-${selectedMonth.value}`)

const targetLabel = computed(() => {
  const t = store.target
  if (!t) return null
  const amount = money(t.savingsTarget)
  if (t.isInherited) {
    const from = DateTime.fromObject({ year: t.sourceYear, month: t.sourceMonth }).toFormat('MMM yyyy')
    return { text: `${amount} (from ${from})`, inherited: true }
  }
  return { text: amount, inherited: false }
})

const savingsSubLabel = computed(() => {
  if (editingTarget.value) return null
  return targetLabel.value
})

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })
}

function signedMoney(n: number): string {
  const abs = money(Math.abs(n))
  return n < 0 ? `-${abs}` : abs
}

function accountName(id: number): string {
  return accountsStore.accounts.find((a) => a.id === id)?.name ?? `#${id}`
}

function setActive(section: ActiveSection) {
  store.activeSection = section
}

async function onMonthChange(value: unknown) {
  const [y, m] = String(value).split('-').map(Number)
  selectedYear.value = y
  selectedMonth.value = m
  editingTarget.value = false
  await store.load(y, m)
}

function startEditTarget() {
  targetInput.value = store.target?.savingsTarget ?? 0
  editingTarget.value = true
}

async function saveTarget() {
  await store.setTarget(selectedYear.value, selectedMonth.value, targetInput.value)
  editingTarget.value = false
}

function cancelEditTarget() {
  editingTarget.value = false
}

onMounted(async () => {
  await Promise.all([accountsStore.load(), store.loadMonths()])
  await store.load(selectedYear.value, selectedMonth.value)
})
</script>

<template>
  <div class="p-6 space-y-4">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-semibold">Budget</h1>
      <USelect
        :model-value="selectedMonthValue"
        :items="monthItems"
        class="w-44"
        @update:model-value="onMonthChange"
      />
    </div>

    <template v-if="store.summary">
      <!-- Horizontal formula row -->
      <div class="border border-default rounded-lg overflow-hidden grid grid-cols-5">
        <!-- Income -->
        <button
          class="p-4 text-left border-r border-default transition-colors hover:bg-elevated"
          :class="store.activeSection === 'income' ? 'bg-elevated' : ''"
          @click="setActive('income')"
        >
          <div class="text-xs text-muted uppercase tracking-wide mb-1">Income</div>
          <div class="text-xl font-bold tabular-nums">{{ money(store.summary.income.total) }}</div>
          <div class="text-xs text-muted mt-1">{{ store.summary.income.transactions.length }} transactions</div>
        </button>

        <!-- Savings -->
        <button
          class="p-4 text-left border-r border-default transition-colors hover:bg-elevated"
          :class="store.activeSection === 'savings' ? 'bg-elevated' : ''"
          @click="setActive('savings')"
        >
          <div class="text-xs text-muted uppercase tracking-wide mb-1">− Savings</div>
          <div class="text-xl font-bold tabular-nums">{{ money(store.summary.savings.total) }}</div>
          <div class="mt-1">
            <template v-if="editingTarget">
              <div class="flex items-center gap-1" @click.stop>
                <UInput
                  v-model.number="targetInput"
                  type="number"
                  step="1"
                  size="xs"
                  class="w-24"
                  @keyup.enter="saveTarget"
                  @keyup.escape="cancelEditTarget"
                />
                <UButton size="xs" variant="ghost" icon="i-lucide-check" @click.stop="saveTarget" />
                <UButton size="xs" variant="ghost" icon="i-lucide-x" @click.stop="cancelEditTarget" />
              </div>
            </template>
            <template v-else>
              <div class="flex items-center gap-1">
                <span
                  class="text-xs"
                  :class="savingsSubLabel?.inherited ? 'text-muted' : 'text-muted'"
                >
                  {{ savingsSubLabel ? `target ${savingsSubLabel.text}` : 'no target set' }}
                </span>
                <UButton
                  size="xs"
                  variant="ghost"
                  icon="i-lucide-pencil"
                  class="opacity-50 hover:opacity-100 -my-1"
                  @click.stop="startEditTarget"
                />
              </div>
            </template>
          </div>
        </button>

        <!-- Fixed -->
        <button
          class="p-4 text-left border-r border-default transition-colors hover:bg-elevated"
          :class="store.activeSection === 'fixed' ? 'bg-elevated' : ''"
          @click="setActive('fixed')"
        >
          <div class="text-xs text-muted uppercase tracking-wide mb-1">− Fixed</div>
          <div class="text-xl font-bold tabular-nums">{{ money(store.summary.fixed.total) }}</div>
          <div class="text-xs text-muted mt-1">{{ store.summary.fixed.transactions.length }} transactions</div>
        </button>

        <!-- Free Money (non-interactive) -->
        <div class="p-4 border-r border-default bg-green-500/5">
          <div class="text-xs text-muted uppercase tracking-wide mb-1">= Free Money</div>
          <div
            class="text-xl font-bold tabular-nums"
            :class="store.summary.freeMoney >= 0 ? 'text-green-600' : 'text-red-600'"
          >
            {{ signedMoney(store.summary.freeMoney) }}
          </div>
          <div class="text-xs text-muted mt-1">&nbsp;</div>
        </div>

        <!-- Discretionary -->
        <button
          class="p-4 text-left transition-colors hover:bg-elevated"
          :class="store.activeSection === 'discretionary' ? 'bg-elevated' : ''"
          @click="setActive('discretionary')"
        >
          <div class="text-xs text-muted uppercase tracking-wide mb-1">Discretionary</div>
          <div class="text-xl font-bold tabular-nums">{{ money(store.summary.discretionary.total) }}</div>
          <div
            class="text-xs mt-1"
            :class="store.summary.freeMoneyRemaining >= 0 ? 'text-green-600' : 'text-red-600'"
          >
            {{ signedMoney(store.summary.freeMoneyRemaining) }} remaining
          </div>
        </button>
      </div>

      <!-- Detail panel -->
      <div class="border border-default rounded-lg overflow-hidden">
        <div class="bg-elevated px-4 py-2 border-b border-default text-sm font-medium capitalize text-muted">
          {{ store.activeSection }}
          <span class="font-normal ml-1">
            · {{ store.summary[store.activeSection].transactions.length }} transactions
          </span>
        </div>

        <template v-if="store.summary[store.activeSection].transactions.length > 0">
          <table class="w-full text-sm">
            <thead class="text-left text-muted border-b border-default">
              <tr>
                <th class="px-4 py-2 font-normal w-28">Date</th>
                <th class="py-2 font-normal">Description</th>
                <th class="py-2 font-normal">Account</th>
                <th class="px-4 py-2 font-normal text-right">Amount</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="t in store.summary[store.activeSection].transactions"
                :key="t.id"
                class="border-t border-default/50"
              >
                <td class="px-4 py-2 text-muted w-28">{{ t.date }}</td>
                <td class="py-2">{{ t.description }}</td>
                <td class="py-2 text-muted">{{ accountName(t.accountId) }}</td>
                <td class="px-4 py-2 text-right tabular-nums">{{ money(t.amount) }}</td>
              </tr>
            </tbody>
          </table>
        </template>
        <p v-else class="px-4 py-6 text-sm text-muted">
          No {{ store.activeSection }} transactions this month.
        </p>
      </div>
    </template>

    <p v-else class="text-muted text-sm">No data yet. Add transactions to see your monthly breakdown.</p>
  </div>
</template>
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
npx vue-tsc --noEmit 2>&1 | head -20
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/pages/Budget.vue
git commit -m "feat: Budget.vue — horizontal anti-budget formula with swappable detail panel"
```

---

### Task 8: Paycheck form — income account dropdown + API update

**Files:**
- Modify: `src/lib/api/paychecks.ts`
- Modify: `src/components/PaycheckForm.vue`

- [ ] **Step 1: Add incomeAccountId to NewPaycheck and UpdatePaycheck in api/paychecks.ts**

In `src/lib/api/paychecks.ts`, add `incomeAccountId?: number | null` to both interfaces:

```ts
export interface NewPaycheck {
  payDate: string
  employer: string
  payPeriod: string
  grossAmount: number
  netAmount: number
  federalTax: number
  stateTax: number
  localTax: number
  socialSecurityTax: number
  medicareTax: number
  deductions: PaycheckDeductionInput[]
  employerMatch: EmployerMatchInput[]
  incomeAccountId?: number | null
  createdAt: string
}

export interface UpdatePaycheck {
  id: number
  payDate: string
  employer: string
  payPeriod: string
  grossAmount: number
  netAmount: number
  federalTax: number
  stateTax: number
  localTax: number
  socialSecurityTax: number
  medicareTax: number
  deductions: PaycheckDeductionInput[]
  employerMatch: EmployerMatchInput[]
  incomeAccountId?: number | null
  updatedAt: string
}
```

- [ ] **Step 2: Add incomeAccountId to PaycheckForm.vue**

In `src/components/PaycheckForm.vue`, make the following changes:

**In `<script setup>`:** Add `incomeAccountId: null as number | null` to `form`:

```ts
const form = reactive({
  payDate: today,
  employer: '',
  payPeriod: 'biweekly',
  grossAmount: 0,
  netAmount: 0,
  federalTax: 0,
  stateTax: 0,
  localTax: 0,
  socialSecurityTax: 0,
  medicareTax: 0,
  deductions: [] as DeductionRow[],
  employerMatch: [] as MatchRow[],
  incomeAccountId: null as number | null,
})
```

**In `resetForm()`** add: `form.incomeAccountId = null`

**In the `watch` for `editing`** add (in both the `e` and `c` branches, inside the `if (e)` block after `employerMatch`):
```ts
form.incomeAccountId = e.incomeAccountId ?? null
```
And in the `else if (c)` block:
```ts
form.incomeAccountId = c.incomeAccountId ?? null
```

**Add a computed for all active non-liability accounts** (for the income account dropdown):

```ts
const allActiveAccounts = computed(() =>
  accountsStore.accounts
    .filter((a) => a.isActive && a.type !== 'liability')
    .map((a) => ({ label: `${a.name} (${a.type})`, value: a.id })),
)
```

**In the `save()` function**, pass `incomeAccountId` in both create and update calls:

In `store.create({...})`, add: `incomeAccountId: form.incomeAccountId,`
In `store.update({...})`, add: `incomeAccountId: form.incomeAccountId,`

**In `<template>`**, add the "Deposit to account" field to the Amounts section, after the net amount field:

```html
<!-- In the Amounts section, add a third row after gross/net grid -->
<div class="mt-2">
  <p class="text-xs text-muted mb-1">Deposit to account <span class="text-muted opacity-60">(optional — creates income transaction)</span></p>
  <USelect
    v-model="form.incomeAccountId"
    :items="allActiveAccounts"
    placeholder="None"
  />
</div>
```

- [ ] **Step 3: Verify TypeScript compiles**

```bash
npx vue-tsc --noEmit 2>&1 | head -20
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/api/paychecks.ts src/components/PaycheckForm.vue
git commit -m "feat: paycheck form — optional income account dropdown creates income transaction"
```

---

### Task 9: Router + Nav + fullscreen

**Files:**
- Modify: `src/router.ts`
- Modify: `src/App.vue`
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: Add the Budget route to router.ts**

In `src/router.ts`, add the budget route (between contributions and settings):

```ts
import { createRouter, createWebHashHistory } from 'vue-router'

const routes = [
  { path: '/', name: 'dashboard', component: () => import('./pages/Dashboard.vue') },
  { path: '/accounts', name: 'accounts', component: () => import('./pages/Accounts.vue') },
  { path: '/transactions', name: 'transactions', component: () => import('./pages/Transactions.vue') },
  { path: '/paychecks', name: 'paychecks', component: () => import('./pages/Paychecks.vue') },
  { path: '/contributions', name: 'contributions', component: () => import('./pages/Contributions.vue') },
  { path: '/budget', name: 'budget', component: () => import('./pages/Budget.vue') },
  { path: '/settings', name: 'settings', component: () => import('./pages/Settings.vue') },
]

export const router = createRouter({ history: createWebHashHistory(), routes })
```

- [ ] **Step 2: Enable the Budget nav link in App.vue**

In `src/App.vue`, update the Budget entry in `links` from `disabled: true` to an active link:

```ts
const links = [
  { label: 'Dashboard', to: '/', icon: 'i-lucide-layout-dashboard' },
  { label: 'Accounts', to: '/accounts', icon: 'i-lucide-wallet' },
  { label: 'Transactions', to: '/transactions', icon: 'i-lucide-receipt' },
  { label: 'Paychecks', to: '/paychecks', icon: 'i-lucide-banknote' },
  { label: 'Contributions', to: '/contributions', icon: 'i-lucide-piggy-bank' },
  { label: 'Budget', to: '/budget', icon: 'i-lucide-calculator' },
  { label: 'Forecast', icon: 'i-lucide-trending-up', disabled: true },
  { label: 'Settings', to: '/settings', icon: 'i-lucide-settings' },
]
```

- [ ] **Step 3: Set the window to fullscreen in tauri.conf.json**

In `src-tauri/tauri.conf.json`, update the window config to add `"fullscreen": true`:

```json
"app": {
  "windows": [
    {
      "title": "trackmyfi-app",
      "width": 800,
      "height": 600,
      "fullscreen": true
    }
  ],
  "security": {
    "csp": null
  }
}
```

- [ ] **Step 4: Verify TypeScript compiles**

```bash
npx vue-tsc --noEmit 2>&1 | head -20
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src/router.ts src/App.vue src-tauri/tauri.conf.json
git commit -m "feat: wire Budget route + nav, set app window to fullscreen"
```

---

### Task 10: Full test suite + build verification

**Files:** None (verification only)

- [ ] **Step 1: Run all Vitest unit tests**

```bash
npx vitest run 2>&1 | tail -20
```

Expected: all tests pass including the new budget tests. No failures.

- [ ] **Step 2: Run all Rust tests**

```bash
cd src-tauri && cargo test 2>&1 | grep -E "test .* (ok|FAILED)|^error" | head -30
```

Expected: all tests pass including the 7 new budget tests. No failures.

- [ ] **Step 3: Run vue-tsc type check**

```bash
npx vue-tsc --noEmit 2>&1 | head -20
```

Expected: no errors.

- [ ] **Step 4: Run cargo build**

```bash
cd src-tauri && cargo build 2>&1 | grep -E "^error"
```

Expected: no output (clean build).

- [ ] **Step 5: Run vite build**

```bash
npx vite build 2>&1 | tail -10
```

Expected: build succeeds with no errors.

- [ ] **Step 6: Commit final verification marker**

```bash
git add -p  # Stage any stray files if any
git commit -m "chore: Phase 2d complete — all tests green, builds clean" --allow-empty
```
