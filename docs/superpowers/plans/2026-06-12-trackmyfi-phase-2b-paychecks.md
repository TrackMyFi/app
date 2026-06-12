# Phase 2b: Paychecks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Paychecks feature — manual entry of paycheck records with structured tax/deduction breakdowns and automatic contribution transaction creation — on top of the existing Phase 2a transactions foundation.

**Architecture:** A new `paycheck` SQLite table stores gross/net/tax columns and JSON arrays for deductions and employer match items. On create/update, Rust commands iterate those arrays and insert contribution transactions (`import_source='paycheck'`) into the existing `txn` table with a `paycheck_id` back-link so edits and deletes stay consistent. Pure TS helpers (`contributionItems`, `paycheckTotals`) power the form's contribution preview panel and list stats.

**Tech Stack:** Rust/Tauri 2.x · libsql 0.9 · serde_json (already in Cargo.toml) · ts-rs 10 · Vue 3 · NuxtUI · Pinia · Vitest · Luxon

**Spec:** `docs/superpowers/specs/2026-06-12-trackmyfi-phase-2b-paychecks-design.md`

---

## File Map

**New files:**
- `src-tauri/migrations/0003_paychecks.sql` — CREATE TABLE paycheck + ALTER TABLE txn
- `src-tauri/src/commands/paychecks.rs` — Rust commands (list/get/create/update/delete)
- `src-tauri/tests/paychecks.rs` — Rust round-trip integration tests
- `src/lib/paychecks/index.ts` — pure TS helpers: `contributionItems`, `paycheckTotals`
- `src/lib/paychecks/index.test.ts` — Vitest tests
- `src/lib/api/paychecks.ts` — `invoke()` wrappers
- `src/stores/paychecks.ts` — Pinia store
- `src/pages/Paychecks.vue` — list workbench page
- `src/components/PaycheckForm.vue` — create/edit form

**Modified files:**
- `src-tauri/src/models.rs` — add `PaycheckDeduction`, `EmployerMatchItem`, `Paycheck` structs
- `src-tauri/src/migrations.rs` — register migration 0003
- `src-tauri/src/commands/mod.rs` — add `pub mod paychecks;`
- `src-tauri/src/lib.rs` — register 5 new commands in `invoke_handler`
- `src-tauri/tests/migrations.rs` — add `paycheck` to expected tables
- `src/router.ts` — add `/paychecks` route
- `src/App.vue` — enable Paychecks nav link

---

## Task 1: Schema, models, and migration

**Files:**
- Create: `src-tauri/migrations/0003_paychecks.sql`
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/migrations.rs`
- Modify: `src-tauri/tests/migrations.rs`

- [ ] **Step 1: Create the migration SQL**

Create `src-tauri/migrations/0003_paychecks.sql`:

```sql
CREATE TABLE paycheck (
  id                  INTEGER PRIMARY KEY AUTOINCREMENT,
  pay_date            TEXT NOT NULL,
  employer            TEXT NOT NULL,
  pay_period          TEXT NOT NULL,
  gross_amount        REAL NOT NULL,
  net_amount          REAL NOT NULL,
  federal_tax         REAL NOT NULL DEFAULT 0,
  state_tax           REAL NOT NULL DEFAULT 0,
  local_tax           REAL NOT NULL DEFAULT 0,
  social_security_tax REAL NOT NULL DEFAULT 0,
  medicare_tax        REAL NOT NULL DEFAULT 0,
  deductions          TEXT NOT NULL DEFAULT '[]',
  employer_match      TEXT NOT NULL DEFAULT '[]',
  import_source       TEXT NOT NULL DEFAULT 'manual',
  created_at          TEXT NOT NULL,
  updated_at          TEXT NOT NULL
);

CREATE INDEX idx_paycheck_date ON paycheck(pay_date);

ALTER TABLE txn ADD COLUMN paycheck_id INTEGER REFERENCES paycheck(id) ON DELETE CASCADE;
```

- [ ] **Step 2: Add the three new model structs to `src-tauri/src/models.rs`**

Append after the existing `ImportMapping` struct:

```rust
#[derive(Serialize, Deserialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct PaycheckDeduction {
    pub label: String,
    pub amount: f64,
    pub pre_tax: bool,
    pub contribution_account_type: Option<String>,
    pub account_id: Option<i32>,
}

#[derive(Serialize, Deserialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct EmployerMatchItem {
    pub label: String,
    pub amount: f64,
    pub account_id: Option<i32>,
}

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
    pub import_source: String,
    pub created_at: String,
    pub updated_at: String,
}
```

- [ ] **Step 3: Register migration 0003 in `src-tauri/src/migrations.rs`**

Add after the existing `transactions` migration entry:

```rust
    Migration {
        version: 3,
        name: "paychecks",
        sql: include_str!("../migrations/0003_paychecks.sql"),
    },
```

- [ ] **Step 4: Update the migration test to assert the `paycheck` table exists**

In `src-tauri/tests/migrations.rs`, update the table list in the assertion:

```rust
    for t in [
        "fire_profile",
        "account",
        "account_balance",
        "txn",
        "import_mapping",
        "paycheck",
        "schema_migrations",
    ] {
```

- [ ] **Step 5: Run the migration test**

```bash
cd src-tauri && cargo test --test migrations
```

Expected: `migrations_create_all_tables ... ok`

The `cargo test` step also compiles all structs, causing ts-rs to export:
- `src/lib/types/PaycheckDeduction.ts`
- `src/lib/types/EmployerMatchItem.ts`
- `src/lib/types/Paycheck.ts`

Verify those three files now exist in `src/lib/types/`.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/migrations/0003_paychecks.sql \
        src-tauri/src/models.rs \
        src-tauri/src/migrations.rs \
        src-tauri/tests/migrations.rs \
        src/lib/types/Paycheck.ts \
        src/lib/types/PaycheckDeduction.ts \
        src/lib/types/EmployerMatchItem.ts
git commit -m "feat: paycheck schema, models, and migration"
```

---

## Task 2: Pure TS helpers (TDD)

**Files:**
- Create: `src/lib/paychecks/index.test.ts`
- Create: `src/lib/paychecks/index.ts`

- [ ] **Step 1: Write the failing tests**

Create `src/lib/paychecks/index.test.ts`:

```typescript
import { describe, it, expect } from 'vitest'
import { contributionItems, paycheckTotals } from './index'
import type { Paycheck } from '../types/Paycheck'

describe('contributionItems', () => {
  it('includes a deduction that has both contributionAccountType and accountId', () => {
    const deductions = [
      { label: '401k', amount: 750, preTax: true, contributionAccountType: '401k', accountId: 1 },
    ]
    const items = contributionItems(deductions, [])
    expect(items).toHaveLength(1)
    expect(items[0]).toEqual({ label: '401k', amount: 750, accountId: 1 })
  })

  it('excludes a deduction missing accountId', () => {
    const deductions = [
      { label: '401k', amount: 750, preTax: true, contributionAccountType: '401k', accountId: null },
    ]
    expect(contributionItems(deductions, [])).toHaveLength(0)
  })

  it('excludes a deduction missing contributionAccountType', () => {
    const deductions = [
      { label: 'Health', amount: 180, preTax: true, contributionAccountType: null, accountId: 1 },
    ]
    expect(contributionItems(deductions, [])).toHaveLength(0)
  })

  it('includes an employer match item that has accountId', () => {
    const match = [{ label: '401k Match', amount: 375, accountId: 2 }]
    const items = contributionItems([], match)
    expect(items).toHaveLength(1)
    expect(items[0]).toEqual({ label: '401k Match', amount: 375, accountId: 2 })
  })

  it('excludes an employer match item without accountId', () => {
    const match = [{ label: '401k Match', amount: 375, accountId: null }]
    expect(contributionItems([], match)).toHaveLength(0)
  })

  it('combines qualifying deductions and match items', () => {
    const deductions = [
      { label: '401k', amount: 750, preTax: true, contributionAccountType: '401k', accountId: 1 },
      { label: 'Health', amount: 180, preTax: true, contributionAccountType: null, accountId: null },
    ]
    const match = [
      { label: '401k Match', amount: 375, accountId: 1 },
      { label: 'Unlinked Match', amount: 100, accountId: null },
    ]
    const items = contributionItems(deductions, match)
    expect(items).toHaveLength(2)
  })

  it('returns empty for empty inputs', () => {
    expect(contributionItems([], [])).toHaveLength(0)
  })
})

describe('paycheckTotals', () => {
  it('sums grossAmount and netAmount across paychecks', () => {
    const paychecks = [
      { grossAmount: 5000, netAmount: 3500 },
      { grossAmount: 4800, netAmount: 3200 },
    ] as Paycheck[]
    expect(paycheckTotals(paychecks)).toEqual({ totalGross: 9800, totalNet: 6700, count: 2 })
  })

  it('returns zeros for an empty array', () => {
    expect(paycheckTotals([])).toEqual({ totalGross: 0, totalNet: 0, count: 0 })
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
export PATH="$HOME/Library/Application Support/fnm/node-versions/v24.12.0/installation/bin:$PATH"
npm test -- src/lib/paychecks/index.test.ts
```

Expected: FAIL — `Cannot find module './index'`

- [ ] **Step 3: Implement the helpers**

Create `src/lib/paychecks/index.ts`:

```typescript
import type { Paycheck } from '../types/Paycheck'

export interface ContributionPreviewItem {
  label: string
  amount: number
  accountId: number
}

interface DeductionLike {
  label: string
  amount: number
  contributionAccountType?: string | null
  accountId?: number | null
}

interface MatchLike {
  label: string
  amount: number
  accountId?: number | null
}

export function contributionItems(
  deductions: DeductionLike[],
  employerMatch: MatchLike[],
): ContributionPreviewItem[] {
  const items: ContributionPreviewItem[] = []
  for (const d of deductions) {
    if (d.contributionAccountType != null && d.accountId != null) {
      items.push({ label: d.label, amount: d.amount, accountId: d.accountId })
    }
  }
  for (const m of employerMatch) {
    if (m.accountId != null) {
      items.push({ label: m.label, amount: m.amount, accountId: m.accountId })
    }
  }
  return items
}

export function paycheckTotals(paychecks: Paycheck[]): {
  totalGross: number
  totalNet: number
  count: number
} {
  let totalGross = 0
  let totalNet = 0
  for (const p of paychecks) {
    totalGross += p.grossAmount
    totalNet += p.netAmount
  }
  return { totalGross, totalNet, count: paychecks.length }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
export PATH="$HOME/Library/Application Support/fnm/node-versions/v24.12.0/installation/bin:$PATH"
npm test -- src/lib/paychecks/index.test.ts
```

Expected: all tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/lib/paychecks/index.ts src/lib/paychecks/index.test.ts
git commit -m "feat: pure TS paycheck helpers (contributionItems, paycheckTotals)"
```

---

## Task 3: Rust commands — create, list, get

**Files:**
- Create: `src-tauri/src/commands/paychecks.rs`
- Create: `src-tauri/tests/paychecks.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing Rust tests**

Create `src-tauri/tests/paychecks.rs`:

```rust
use libsql::Builder;
use trackmyfi_app_lib::commands::accounts::{self, NewAccount, NewBalance};
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
            account_id: None, // no account linked
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
async fn get_paycheck_missing_id_errors() {
    let conn = setup().await;
    assert!(paychecks::get_paycheck(&conn, 9999).await.is_err());
}
```

- [ ] **Step 2: Run to verify tests fail**

```bash
cd src-tauri && cargo test --test paychecks 2>&1 | head -20
```

Expected: compile error — `commands::paychecks` not found

- [ ] **Step 3: Create `src-tauri/src/commands/paychecks.rs`**

```rust
use crate::db::Db;
use crate::models::{Paycheck, PaycheckDeduction, EmployerMatchItem};
use libsql::{params, Connection};
use serde::Deserialize;
use tauri::State;

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
    pub created_at: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PaycheckFilter {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub employer: Option<String>,
}

const COLS: &str = "id, pay_date, employer, pay_period, gross_amount, net_amount, \
    federal_tax, state_tax, local_tax, social_security_tax, medicare_tax, \
    deductions, employer_match, import_source, created_at, updated_at";

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
        import_source: row.get(13).map_err(|e| e.to_string())?,
        created_at: row.get(14).map_err(|e| e.to_string())?,
        updated_at: row.get(15).map_err(|e| e.to_string())?,
    })
}

async fn auto_create_contributions(
    conn: &Connection,
    paycheck_id: i32,
    pay_date: &str,
    deductions: &[PaycheckDeduction],
    employer_match: &[EmployerMatchItem],
    now: &str,
) -> Result<(), String> {
    for ded in deductions {
        if ded.contribution_account_type.is_some() {
            if let Some(account_id) = ded.account_id {
                conn.execute(
                    "INSERT INTO txn (account_id, transfer_account_id, amount, description, date, \
                     type, category, is_contribution, import_source, paycheck_id, \
                     generated_balance_id, generated_balance_to_id, created_at, updated_at) \
                     VALUES (?1, NULL, ?2, ?3, ?4, 'income', 'savings', 1, 'paycheck', ?5, \
                     NULL, NULL, ?6, ?6)",
                    params![account_id, ded.amount, ded.label.clone(), pay_date, paycheck_id, now],
                )
                .await
                .map_err(|e| e.to_string())?;
            }
        }
    }
    for em in employer_match {
        if let Some(account_id) = em.account_id {
            conn.execute(
                "INSERT INTO txn (account_id, transfer_account_id, amount, description, date, \
                 type, category, is_contribution, import_source, paycheck_id, \
                 generated_balance_id, generated_balance_to_id, created_at, updated_at) \
                 VALUES (?1, NULL, ?2, ?3, ?4, 'income', 'savings', 1, 'paycheck', ?5, \
                 NULL, NULL, ?6, ?6)",
                params![account_id, em.amount, em.label.clone(), pay_date, paycheck_id, now],
            )
            .await
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub async fn get_paycheck(conn: &Connection, id: i32) -> Result<Paycheck, String> {
    let sql = format!("SELECT {COLS} FROM paycheck WHERE id = ?1");
    let mut rows = conn.query(&sql, params![id]).await.map_err(|e| e.to_string())?;
    match rows.next().await.map_err(|e| e.to_string())? {
        Some(row) => row_to_paycheck(&row),
        None => Err(format!("paycheck {id} not found")),
    }
}

pub async fn list_paychecks(conn: &Connection, f: &PaycheckFilter) -> Result<Vec<Paycheck>, String> {
    let mut where_clauses: Vec<String> = Vec::new();
    let mut bind_params: Vec<libsql::Value> = Vec::new();

    if let Some(s) = &f.start_date {
        where_clauses.push("pay_date >= ?".into());
        bind_params.push(libsql::Value::Text(s.clone()));
    }
    if let Some(e) = &f.end_date {
        where_clauses.push("pay_date <= ?".into());
        bind_params.push(libsql::Value::Text(e.clone()));
    }
    if let Some(emp) = &f.employer {
        where_clauses.push("employer LIKE ?".into());
        bind_params.push(libsql::Value::Text(format!("%{}%", emp)));
    }

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    let sql = format!("SELECT {COLS} FROM paycheck {where_sql} ORDER BY pay_date DESC, id DESC");
    let mut rows = conn
        .query(&sql, libsql::params_from_iter(bind_params))
        .await
        .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(row_to_paycheck(&row)?);
    }
    Ok(out)
}

pub async fn create_paycheck(conn: &Connection, p: &NewPaycheck) -> Result<Paycheck, String> {
    let deductions_json = serde_json::to_string(&p.deductions).map_err(|e| e.to_string())?;
    let employer_match_json = serde_json::to_string(&p.employer_match).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO paycheck (pay_date, employer, pay_period, gross_amount, net_amount, \
         federal_tax, state_tax, local_tax, social_security_tax, medicare_tax, \
         deductions, employer_match, import_source, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'manual', ?13, ?13)",
        params![
            p.pay_date.clone(), p.employer.clone(), p.pay_period.clone(),
            p.gross_amount, p.net_amount,
            p.federal_tax, p.state_tax, p.local_tax, p.social_security_tax, p.medicare_tax,
            deductions_json, employer_match_json, p.created_at.clone()
        ],
    )
    .await
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid() as i32;
    auto_create_contributions(conn, id, &p.pay_date, &p.deductions, &p.employer_match, &p.created_at).await?;
    get_paycheck(conn, id).await
}

// ---- thin command wrappers ----

#[tauri::command]
pub async fn list_paychecks_cmd(
    db: State<'_, Db>,
    filter: PaycheckFilter,
) -> Result<Vec<Paycheck>, String> {
    let conn = db.conn().await?;
    list_paychecks(&conn, &filter).await
}

#[tauri::command]
pub async fn get_paycheck_cmd(db: State<'_, Db>, id: i32) -> Result<Paycheck, String> {
    let conn = db.conn().await?;
    get_paycheck(&conn, id).await
}

#[tauri::command]
pub async fn create_paycheck_cmd(
    db: State<'_, Db>,
    paycheck: NewPaycheck,
) -> Result<Paycheck, String> {
    let conn = db.conn().await?;
    create_paycheck(&conn, &paycheck).await
}
```

- [ ] **Step 4: Add `pub mod paychecks;` to `src-tauri/src/commands/mod.rs`**

```rust
pub mod accounts;
pub mod fire_profile;
pub mod import_mappings;
pub mod paychecks;
pub mod transactions;
```

- [ ] **Step 5: Run the failing tests to verify progress**

```bash
cd src-tauri && cargo test --test paychecks 2>&1 | tail -20
```

Expected: tests for update/delete will be absent (not written yet), and create/get/list tests should pass. If any test fails, fix before continuing.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands/paychecks.rs \
        src-tauri/src/commands/mod.rs \
        src-tauri/tests/paychecks.rs
git commit -m "feat: Rust paycheck create/list/get commands with auto-contribution logic"
```

---

## Task 4: Rust commands — update and delete

**Files:**
- Modify: `src-tauri/src/commands/paychecks.rs` (add `update_paycheck`, `delete_paycheck` and their `_cmd` wrappers)
- Modify: `src-tauri/src/lib.rs` (register all 5 commands)
- Modify: `src-tauri/tests/paychecks.rs` (add update/delete tests)

- [ ] **Step 1: Add update and delete tests to `src-tauri/tests/paychecks.rs`**

Append to the existing test file:

```rust
#[tokio::test]
async fn update_paycheck_recreates_contributions() {
    let conn = setup().await;
    let acct = make_account(&conn, "Fidelity 401k", "401k").await;
    let acct2 = make_account(&conn, "HSA", "hsa").await;

    // Create with one contribution deduction
    let mut p = base_paycheck("Acme", "2026-06-15");
    p.deductions = vec![
        PaycheckDeduction {
            label: "401k".into(), amount: 750.0, pre_tax: true,
            contribution_account_type: Some("401k".into()), account_id: Some(acct),
        },
    ];
    let created = paychecks::create_paycheck(&conn, &p).await.unwrap();
    assert_eq!(contribution_count_for(&conn, created.id).await, 1);

    // Update: change amount and add a second contribution deduction
    paychecks::update_paycheck(&conn, &trackmyfi_app_lib::commands::paychecks::UpdatePaycheck {
        id: created.id,
        pay_date: "2026-06-15".into(),
        employer: "Acme".into(),
        pay_period: "biweekly".into(),
        gross_amount: 5000.0,
        net_amount: 3100.0,
        federal_tax: 800.0, state_tax: 250.0, local_tax: 0.0,
        social_security_tax: 310.0, medicare_tax: 72.5,
        deductions: vec![
            PaycheckDeduction {
                label: "401k".into(), amount: 800.0, pre_tax: true,
                contribution_account_type: Some("401k".into()), account_id: Some(acct),
            },
            PaycheckDeduction {
                label: "HSA".into(), amount: 150.0, pre_tax: true,
                contribution_account_type: Some("hsa".into()), account_id: Some(acct2),
            },
        ],
        employer_match: vec![],
        updated_at: "2026-06-16T00:00:00Z".into(),
    }).await.unwrap();

    // Old contribution removed, two new ones created
    assert_eq!(contribution_count_for(&conn, created.id).await, 2);

    // Verify new amount
    let mut rows = conn.query(
        "SELECT amount FROM txn WHERE paycheck_id = ?1 AND account_id = ?2",
        libsql::params![created.id, acct],
    ).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<f64>(0).unwrap(), 800.0);
}

#[tokio::test]
async fn delete_paycheck_removes_contributions() {
    let conn = setup().await;
    let acct = make_account(&conn, "Fidelity 401k", "401k").await;

    let mut p = base_paycheck("Acme", "2026-06-15");
    p.deductions = vec![
        PaycheckDeduction {
            label: "401k".into(), amount: 750.0, pre_tax: true,
            contribution_account_type: Some("401k".into()), account_id: Some(acct),
        },
    ];
    let created = paychecks::create_paycheck(&conn, &p).await.unwrap();
    assert_eq!(txn_count(&conn).await, 1);

    paychecks::delete_paycheck(&conn, created.id).await.unwrap();
    assert_eq!(txn_count(&conn).await, 0);

    // Paycheck itself gone
    assert!(paychecks::get_paycheck(&conn, created.id).await.is_err());
}
```

- [ ] **Step 2: Run to verify new tests fail**

```bash
cd src-tauri && cargo test --test paychecks update_paycheck 2>&1 | tail -5
```

Expected: compile error — `update_paycheck` and `delete_paycheck` not found

- [ ] **Step 3: Add `UpdatePaycheck` struct and `update_paycheck` / `delete_paycheck` functions to `src-tauri/src/commands/paychecks.rs`**

Add after the existing `NewPaycheck` struct definition:

```rust
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
    pub updated_at: String,
}
```

Add the two functions after `create_paycheck`:

```rust
pub async fn update_paycheck(conn: &Connection, p: &UpdatePaycheck) -> Result<Paycheck, String> {
    // Delete all contribution txns previously created by this paycheck
    conn.execute("DELETE FROM txn WHERE paycheck_id = ?1", params![p.id])
        .await
        .map_err(|e| e.to_string())?;

    let deductions_json = serde_json::to_string(&p.deductions).map_err(|e| e.to_string())?;
    let employer_match_json = serde_json::to_string(&p.employer_match).map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE paycheck SET pay_date=?1, employer=?2, pay_period=?3, gross_amount=?4, \
         net_amount=?5, federal_tax=?6, state_tax=?7, local_tax=?8, social_security_tax=?9, \
         medicare_tax=?10, deductions=?11, employer_match=?12, updated_at=?13 WHERE id=?14",
        params![
            p.pay_date.clone(), p.employer.clone(), p.pay_period.clone(),
            p.gross_amount, p.net_amount,
            p.federal_tax, p.state_tax, p.local_tax, p.social_security_tax, p.medicare_tax,
            deductions_json, employer_match_json, p.updated_at.clone(), p.id
        ],
    )
    .await
    .map_err(|e| e.to_string())?;

    auto_create_contributions(conn, p.id, &p.pay_date, &p.deductions, &p.employer_match, &p.updated_at).await?;
    get_paycheck(conn, p.id).await
}

pub async fn delete_paycheck(conn: &Connection, id: i32) -> Result<(), String> {
    // Explicitly delete contribution transactions before deleting the paycheck
    // (FK cascade requires PRAGMA foreign_keys = ON which is not set globally)
    conn.execute("DELETE FROM txn WHERE paycheck_id = ?1", params![id])
        .await
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM paycheck WHERE id = ?1", params![id])
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
```

Add the two `_cmd` wrappers at the bottom of the file (after the existing `create_paycheck_cmd`):

```rust
#[tauri::command]
pub async fn update_paycheck_cmd(
    db: State<'_, Db>,
    paycheck: UpdatePaycheck,
) -> Result<Paycheck, String> {
    let conn = db.conn().await?;
    update_paycheck(&conn, &paycheck).await
}

#[tauri::command]
pub async fn delete_paycheck_cmd(db: State<'_, Db>, id: i32) -> Result<(), String> {
    let conn = db.conn().await?;
    delete_paycheck(&conn, id).await
}
```

- [ ] **Step 4: Register all 5 paycheck commands in `src-tauri/src/lib.rs`**

Add inside `invoke_handler!`, after the `import_mappings` lines:

```rust
            commands::paychecks::list_paychecks_cmd,
            commands::paychecks::get_paycheck_cmd,
            commands::paychecks::create_paycheck_cmd,
            commands::paychecks::update_paycheck_cmd,
            commands::paychecks::delete_paycheck_cmd,
```

- [ ] **Step 5: Run all paycheck tests**

```bash
cd src-tauri && cargo test --test paychecks
```

Expected: all tests pass

- [ ] **Step 6: Run all tests to confirm nothing regressed**

```bash
cd src-tauri && cargo test
```

Expected: all tests pass

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands/paychecks.rs \
        src-tauri/src/lib.rs \
        src-tauri/tests/paychecks.rs
git commit -m "feat: Rust paycheck update/delete commands; register all 5 in invoke_handler"
```

---

## Task 5: API bindings and Pinia store

**Files:**
- Create: `src/lib/api/paychecks.ts`
- Create: `src/stores/paychecks.ts`

- [ ] **Step 1: Create `src/lib/api/paychecks.ts`**

```typescript
import { invoke } from '@tauri-apps/api/core'
import type { Paycheck } from '../types/Paycheck'

export interface PaycheckFilter {
  startDate?: string | null
  endDate?: string | null
  employer?: string | null
}

export interface PaycheckDeductionInput {
  label: string
  amount: number
  preTax: boolean
  contributionAccountType?: string | null
  accountId?: number | null
}

export interface EmployerMatchInput {
  label: string
  amount: number
  accountId?: number | null
}

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
  updatedAt: string
}

export const listPaychecks = (filter: PaycheckFilter = {}) =>
  invoke<Paycheck[]>('list_paychecks_cmd', { filter })
export const getPaycheck = (id: number) =>
  invoke<Paycheck>('get_paycheck_cmd', { id })
export const createPaycheck = (paycheck: NewPaycheck) =>
  invoke<Paycheck>('create_paycheck_cmd', { paycheck })
export const updatePaycheck = (paycheck: UpdatePaycheck) =>
  invoke<Paycheck>('update_paycheck_cmd', { paycheck })
export const deletePaycheck = (id: number) =>
  invoke<void>('delete_paycheck_cmd', { id })
```

- [ ] **Step 2: Create `src/stores/paychecks.ts`**

```typescript
import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Paycheck } from '../lib/types/Paycheck'
import * as api from '../lib/api/paychecks'

export const usePaychecksStore = defineStore('paychecks', () => {
  const paychecks = ref<Paycheck[]>([])
  const filter = ref<api.PaycheckFilter>({})

  async function load() {
    paychecks.value = await api.listPaychecks(filter.value)
  }
  async function setFilter(patch: Partial<api.PaycheckFilter>) {
    filter.value = { ...filter.value, ...patch }
    await load()
  }
  async function create(p: api.NewPaycheck) {
    await api.createPaycheck(p)
    await load()
  }
  async function update(p: api.UpdatePaycheck) {
    await api.updatePaycheck(p)
    await load()
  }
  async function remove(id: number) {
    await api.deletePaycheck(id)
    await load()
  }

  return { paychecks, filter, load, setFilter, create, update, remove }
})

export type { Paycheck }
```

- [ ] **Step 3: Verify TypeScript compiles cleanly**

```bash
export PATH="$HOME/Library/Application Support/fnm/node-versions/v24.12.0/installation/bin:$PATH"
npx vue-tsc --noEmit
```

Expected: no errors

- [ ] **Step 4: Commit**

```bash
git add src/lib/api/paychecks.ts src/stores/paychecks.ts
git commit -m "feat: paycheck API bindings and Pinia store"
```

---

## Task 6: PaycheckForm component

**Files:**
- Create: `src/components/PaycheckForm.vue`

- [ ] **Step 1: Create `src/components/PaycheckForm.vue`**

```vue
<script setup lang="ts">
import { computed, reactive, watch } from 'vue'
import { DateTime } from 'luxon'
import { usePaychecksStore } from '../stores/paychecks'
import { useAccountsStore } from '../stores/accounts'
import { contributionItems } from '../lib/paychecks/index'
import { INVESTMENT_TYPES } from '../lib/accountTypes'
import DateInput from './DateInput.vue'
import type { Paycheck } from '../lib/types/Paycheck'

const PAY_PERIODS = ['weekly', 'biweekly', 'semimonthly', 'monthly', 'irregular'] as const

// Account types eligible as contribution targets
const CONTRIBUTION_ACCOUNT_TYPES = [...INVESTMENT_TYPES].map((t) => ({ label: t, value: t }))

const props = defineProps<{ editing: Paycheck | null }>()
const emit = defineEmits<{ saved: [] }>()

const store = usePaychecksStore()
const accountsStore = useAccountsStore()

const today = DateTime.now().toISODate()!

interface DeductionRow {
  label: string
  amount: number
  preTax: boolean
  contributionAccountType: string | null
  accountId: number | null
}

interface MatchRow {
  label: string
  amount: number
  accountId: number | null
}

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
})

function resetForm() {
  form.payDate = today
  form.employer = ''
  form.payPeriod = 'biweekly'
  form.grossAmount = 0
  form.netAmount = 0
  form.federalTax = 0
  form.stateTax = 0
  form.localTax = 0
  form.socialSecurityTax = 0
  form.medicareTax = 0
  form.deductions = []
  form.employerMatch = []
}

watch(
  () => props.editing,
  (e) => {
    if (e) {
      form.payDate = e.payDate
      form.employer = e.employer
      form.payPeriod = e.payPeriod
      form.grossAmount = e.grossAmount
      form.netAmount = e.netAmount
      form.federalTax = e.federalTax
      form.stateTax = e.stateTax
      form.localTax = e.localTax
      form.socialSecurityTax = e.socialSecurityTax
      form.medicareTax = e.medicareTax
      form.deductions = e.deductions.map((d) => ({
        label: d.label,
        amount: d.amount,
        preTax: d.preTax,
        contributionAccountType: d.contributionAccountType ?? null,
        accountId: d.accountId ?? null,
      }))
      form.employerMatch = e.employerMatch.map((m) => ({
        label: m.label,
        amount: m.amount,
        accountId: m.accountId ?? null,
      }))
    } else {
      resetForm()
    }
  },
  { immediate: true },
)

// Account dropdown filtered by contribution type
function accountsForType(type: string | null) {
  if (!type) return []
  return accountsStore.accounts
    .filter((a) => a.type === type && a.isActive)
    .map((a) => ({ label: a.name, value: a.id }))
}

const investmentAccountItems = computed(() =>
  accountsStore.accounts
    .filter((a) => INVESTMENT_TYPES.has(a.type) && a.isActive)
    .map((a) => ({ label: a.name, value: a.id })),
)

function addDeduction() {
  form.deductions.push({ label: '', amount: 0, preTax: true, contributionAccountType: null, accountId: null })
}
function removeDeduction(i: number) { form.deductions.splice(i, 1) }

function addMatch() {
  form.employerMatch.push({ label: '', amount: 0, accountId: null })
}
function removeMatch(i: number) { form.employerMatch.splice(i, 1) }

// Clear accountId when contribution type changes (avoid stale selection)
function onContributionTypeChange(ded: DeductionRow) {
  ded.accountId = null
}

const preview = computed(() => {
  const items = contributionItems(form.deductions, form.employerMatch)
  return items.map((item) => ({
    ...item,
    accountName: accountsStore.accounts.find((a) => a.id === item.accountId)?.name ?? `#${item.accountId}`,
  }))
})

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}

async function save() {
  const now = DateTime.now().toISO()!
  if (props.editing) {
    await store.update({
      id: props.editing.id,
      payDate: form.payDate,
      employer: form.employer,
      payPeriod: form.payPeriod,
      grossAmount: form.grossAmount,
      netAmount: form.netAmount,
      federalTax: form.federalTax,
      stateTax: form.stateTax,
      localTax: form.localTax,
      socialSecurityTax: form.socialSecurityTax,
      medicareTax: form.medicareTax,
      deductions: form.deductions,
      employerMatch: form.employerMatch,
      updatedAt: now,
    })
  } else {
    await store.create({
      payDate: form.payDate,
      employer: form.employer,
      payPeriod: form.payPeriod,
      grossAmount: form.grossAmount,
      netAmount: form.netAmount,
      federalTax: form.federalTax,
      stateTax: form.stateTax,
      localTax: form.localTax,
      socialSecurityTax: form.socialSecurityTax,
      medicareTax: form.medicareTax,
      deductions: form.deductions,
      employerMatch: form.employerMatch,
      createdAt: now,
    })
  }
  emit('saved')
}
</script>

<template>
  <form class="space-y-5" @submit.prevent="save">

    <!-- Paycheck info -->
    <div class="space-y-3">
      <p class="text-xs font-semibold uppercase tracking-wide text-muted">Paycheck info</p>
      <div class="grid grid-cols-3 gap-3">
        <div>
          <p class="text-xs text-muted mb-1">Pay date</p>
          <DateInput v-model="form.payDate" />
        </div>
        <div>
          <p class="text-xs text-muted mb-1">Employer</p>
          <UInput v-model="form.employer" placeholder="Employer" />
        </div>
        <div>
          <p class="text-xs text-muted mb-1">Pay period</p>
          <USelect
            v-model="form.payPeriod"
            :items="PAY_PERIODS.map((p) => ({ label: p, value: p }))"
          />
        </div>
      </div>
    </div>

    <!-- Amounts -->
    <div class="space-y-3">
      <p class="text-xs font-semibold uppercase tracking-wide text-muted">Amounts</p>
      <div class="grid grid-cols-2 gap-3">
        <div>
          <p class="text-xs text-muted mb-1">Gross</p>
          <UInput v-model.number="form.grossAmount" type="number" step="0.01" placeholder="0.00" />
        </div>
        <div>
          <p class="text-xs text-muted mb-1">Net (take-home)</p>
          <UInput v-model.number="form.netAmount" type="number" step="0.01" placeholder="0.00" />
        </div>
      </div>
    </div>

    <!-- Taxes -->
    <div class="space-y-3">
      <p class="text-xs font-semibold uppercase tracking-wide text-muted">Taxes withheld</p>
      <div class="grid grid-cols-3 gap-3">
        <div>
          <p class="text-xs text-muted mb-1">Federal</p>
          <UInput v-model.number="form.federalTax" type="number" step="0.01" placeholder="0.00" />
        </div>
        <div>
          <p class="text-xs text-muted mb-1">State</p>
          <UInput v-model.number="form.stateTax" type="number" step="0.01" placeholder="0.00" />
        </div>
        <div>
          <p class="text-xs text-muted mb-1">Local</p>
          <UInput v-model.number="form.localTax" type="number" step="0.01" placeholder="0.00" />
        </div>
        <div>
          <p class="text-xs text-muted mb-1">Social Security</p>
          <UInput v-model.number="form.socialSecurityTax" type="number" step="0.01" placeholder="0.00" />
        </div>
        <div>
          <p class="text-xs text-muted mb-1">Medicare</p>
          <UInput v-model.number="form.medicareTax" type="number" step="0.01" placeholder="0.00" />
        </div>
      </div>
    </div>

    <!-- Deductions -->
    <div class="space-y-2">
      <div class="flex items-center justify-between">
        <p class="text-xs font-semibold uppercase tracking-wide text-muted">Deductions</p>
        <UButton size="xs" variant="soft" icon="i-lucide-plus" @click="addDeduction">Add</UButton>
      </div>
      <div v-for="(ded, i) in form.deductions" :key="i" class="rounded border border-default p-3 space-y-2">
        <div class="grid grid-cols-[1fr_auto_auto_auto] gap-2 items-center">
          <UInput v-model="ded.label" placeholder="Label" />
          <UInput v-model.number="ded.amount" type="number" step="0.01" placeholder="0.00" class="w-28" />
          <UCheckbox v-model="ded.preTax" label="Pre-tax" />
          <UButton size="xs" variant="ghost" color="error" icon="i-lucide-x" @click="removeDeduction(i)" />
        </div>
        <div class="grid grid-cols-2 gap-2">
          <div>
            <p class="text-xs text-muted mb-1">Contribution type (optional)</p>
            <USelect
              v-model="ded.contributionAccountType"
              :items="CONTRIBUTION_ACCOUNT_TYPES"
              placeholder="None"
              clearable
              @change="onContributionTypeChange(ded)"
            />
          </div>
          <div v-if="ded.contributionAccountType">
            <p class="text-xs text-muted mb-1">Account</p>
            <USelect
              v-model="ded.accountId"
              :items="accountsForType(ded.contributionAccountType)"
              placeholder="Select account"
            />
          </div>
        </div>
      </div>
      <p v-if="form.deductions.length === 0" class="text-xs text-muted">No deductions added.</p>
    </div>

    <!-- Employer Match -->
    <div class="space-y-2">
      <div class="flex items-center justify-between">
        <p class="text-xs font-semibold uppercase tracking-wide text-muted">Employer Match</p>
        <UButton size="xs" variant="soft" icon="i-lucide-plus" @click="addMatch">Add</UButton>
      </div>
      <div v-for="(em, i) in form.employerMatch" :key="i" class="grid grid-cols-[1fr_auto_1fr_auto] gap-2 items-center">
        <UInput v-model="em.label" placeholder="Label" />
        <UInput v-model.number="em.amount" type="number" step="0.01" placeholder="0.00" class="w-28" />
        <USelect v-model="em.accountId" :items="investmentAccountItems" placeholder="Account" />
        <UButton size="xs" variant="ghost" color="error" icon="i-lucide-x" @click="removeMatch(i)" />
      </div>
      <p v-if="form.employerMatch.length === 0" class="text-xs text-muted">No employer match added.</p>
    </div>

    <!-- Contribution preview -->
    <div v-if="preview.length > 0" class="rounded border border-default p-3 space-y-1">
      <p class="text-xs font-semibold uppercase tracking-wide text-muted">Contributions that will be created</p>
      <div v-for="item in preview" :key="item.accountId + item.label" class="flex justify-between text-sm">
        <span class="text-muted">{{ item.label }} → {{ item.accountName }}</span>
        <span class="tabular-nums text-green-600">{{ money(item.amount) }}</span>
      </div>
    </div>

    <div class="flex justify-end gap-2 pt-2">
      <UButton type="submit">{{ props.editing ? 'Save' : 'Add paycheck' }}</UButton>
    </div>

  </form>
</template>
```

- [ ] **Step 2: Verify TypeScript compiles cleanly**

```bash
export PATH="$HOME/Library/Application Support/fnm/node-versions/v24.12.0/installation/bin:$PATH"
npx vue-tsc --noEmit
```

Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add src/components/PaycheckForm.vue
git commit -m "feat: PaycheckForm component"
```

---

## Task 7: Paychecks list workbench page

**Files:**
- Create: `src/pages/Paychecks.vue`

- [ ] **Step 1: Create `src/pages/Paychecks.vue`**

```vue
<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { usePaychecksStore } from '../stores/paychecks'
import { useAccountsStore } from '../stores/accounts'
import { paycheckTotals } from '../lib/paychecks/index'
import PaycheckForm from '../components/PaycheckForm.vue'
import type { Paycheck } from '../lib/types/Paycheck'
import { confirm } from '@tauri-apps/plugin-dialog'

const store = usePaychecksStore()
const accountsStore = useAccountsStore()

const isModalOpen = ref(false)
const editing = ref<Paycheck | null>(null)

function openAdd() { editing.value = null; isModalOpen.value = true }
function openEdit(p: Paycheck) { editing.value = p; isModalOpen.value = true }
function onSaved() { isModalOpen.value = false }

async function removeRow(p: Paycheck) {
  const ok = await confirm(`Delete paycheck from "${p.employer}" on ${p.payDate}?`, {
    title: 'Delete paycheck',
  })
  if (ok) await store.remove(p.id)
}

const startDate = ref('')
const endDate = ref('')
const employerSearch = ref('')

async function applyFilters() {
  await store.setFilter({
    startDate: startDate.value || null,
    endDate: endDate.value || null,
    employer: employerSearch.value || null,
  })
}

const totals = computed(() => paycheckTotals(store.paychecks))

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}

onMounted(async () => {
  await accountsStore.load()
  await store.load()
})
</script>

<template>
  <div class="p-6 space-y-4">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-semibold">Paychecks</h1>
      <UButton icon="i-lucide-plus" @click="openAdd">Add paycheck</UButton>
    </div>

    <div class="flex flex-wrap gap-2 items-end">
      <div>
        <p class="text-xs text-muted mb-1">From</p>
        <UInput v-model="startDate" type="date" class="w-36" />
      </div>
      <div>
        <p class="text-xs text-muted mb-1">To</p>
        <UInput v-model="endDate" type="date" class="w-36" />
      </div>
      <UInput v-model="employerSearch" placeholder="Search employer" class="w-44" />
      <UButton @click="applyFilters">Apply</UButton>
    </div>

    <div class="flex gap-6 text-sm">
      <span>Gross: <strong>{{ money(totals.totalGross) }}</strong></span>
      <span>Net: <strong>{{ money(totals.totalNet) }}</strong></span>
      <span class="text-muted">{{ totals.count }} paychecks</span>
    </div>

    <table class="w-full text-sm">
      <thead class="text-left text-muted border-b border-default">
        <tr>
          <th class="py-2">Date</th>
          <th>Employer</th>
          <th>Period</th>
          <th class="text-right">Gross</th>
          <th class="text-right">Net</th>
          <th class="text-right">Federal</th>
          <th class="text-right">SS + Medicare</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="p in store.paychecks" :key="p.id" class="border-b border-default/50">
          <td class="py-2">{{ p.payDate }}</td>
          <td>{{ p.employer }}</td>
          <td class="capitalize">{{ p.payPeriod }}</td>
          <td class="text-right tabular-nums">{{ money(p.grossAmount) }}</td>
          <td class="text-right tabular-nums">{{ money(p.netAmount) }}</td>
          <td class="text-right tabular-nums">{{ money(p.federalTax) }}</td>
          <td class="text-right tabular-nums">{{ money(p.socialSecurityTax + p.medicareTax) }}</td>
          <td class="text-right">
            <UButton size="xs" variant="ghost" icon="i-lucide-pencil" @click="openEdit(p)" />
            <UButton size="xs" variant="ghost" color="error" icon="i-lucide-trash-2" @click="removeRow(p)" />
          </td>
        </tr>
        <tr v-if="!store.paychecks.length">
          <td colspan="8" class="py-6 text-center text-muted">No paychecks yet.</td>
        </tr>
      </tbody>
    </table>

    <UModal v-model:open="isModalOpen" :title="editing ? 'Edit paycheck' : 'Add paycheck'">
      <template #body>
        <PaycheckForm :editing="editing" @saved="onSaved" />
      </template>
    </UModal>
  </div>
</template>
```

- [ ] **Step 2: Verify TypeScript compiles cleanly**

```bash
export PATH="$HOME/Library/Application Support/fnm/node-versions/v24.12.0/installation/bin:$PATH"
npx vue-tsc --noEmit
```

Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add src/pages/Paychecks.vue
git commit -m "feat: Paychecks list workbench page"
```

---

## Task 8: Router and nav wiring

**Files:**
- Modify: `src/router.ts`
- Modify: `src/App.vue`

- [ ] **Step 1: Add the Paychecks route to `src/router.ts`**

```typescript
import { createRouter, createWebHashHistory } from 'vue-router'

const routes = [
  { path: '/', name: 'dashboard', component: () => import('./pages/Dashboard.vue') },
  { path: '/accounts', name: 'accounts', component: () => import('./pages/Accounts.vue') },
  { path: '/transactions', name: 'transactions', component: () => import('./pages/Transactions.vue') },
  { path: '/paychecks', name: 'paychecks', component: () => import('./pages/Paychecks.vue') },
  { path: '/settings', name: 'settings', component: () => import('./pages/Settings.vue') },
]

export const router = createRouter({ history: createWebHashHistory(), routes })
```

- [ ] **Step 2: Enable the Paychecks nav link in `src/App.vue`**

Replace the disabled Paychecks entry:

```typescript
// Before:
{ label: 'Paychecks', icon: 'i-lucide-banknote', disabled: true },

// After:
{ label: 'Paychecks', to: '/paychecks', icon: 'i-lucide-banknote' },
```

- [ ] **Step 3: Run full test suite + type check**

```bash
export PATH="$HOME/Library/Application Support/fnm/node-versions/v24.12.0/installation/bin:$PATH"
npm test && npx vue-tsc --noEmit
```

Expected: all Vitest tests pass, no TypeScript errors

```bash
cd src-tauri && cargo test
```

Expected: all Rust tests pass

- [ ] **Step 4: Commit**

```bash
git add src/router.ts src/App.vue
git commit -m "feat: wire Paychecks route and nav link"
```

---

## Done

Phase 2b is complete when all tasks pass. The Paychecks page is accessible from the nav and allows manual paycheck entry with deductions, employer match, and automatic contribution transaction creation. Phase 2c (Contributions view) can now be designed — it queries `txn` where `is_contribution = 1`.
