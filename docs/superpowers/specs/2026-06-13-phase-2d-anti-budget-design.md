# TrackMyFI — Phase 2d: Anti-Budget Design

**Date:** 2026-06-13  
**Status:** Approved, ready for implementation

---

## Overview

Phase 2d adds the Anti-Budget monthly view — the pay-yourself-first math for any given month:

```
Income
− Savings
− Fixed
= Free Money
  (vs. Discretionary spent)
```

Everything except the optional savings target is derived from existing transaction data. No new transaction entry is required beyond what paychecks and the transaction ledger already provide.

---

## Data Changes

### Migration 0005 — two changes in one file

**New table: `budget_month`**

```sql
CREATE TABLE budget_month (
  id      INTEGER PRIMARY KEY AUTOINCREMENT,
  year    INTEGER NOT NULL,
  month   INTEGER NOT NULL,  -- 1–12
  savings_target REAL NOT NULL,
  UNIQUE(year, month)
);
```

Stores optional per-month savings targets. Only created when the user explicitly sets a target via the inline editor on the Budget page.

**New column: `paycheck.income_account_id`**

```sql
ALTER TABLE paycheck ADD COLUMN income_account_id INTEGER REFERENCES account(id);
```

Optional. When set, saving or updating a paycheck auto-creates a linked income transaction for the net amount (same pattern as deductions auto-creating contribution transactions).

---

## Paycheck → Income Transaction Link

When a paycheck is saved/updated with `income_account_id` set:

- Delete any existing income transaction linked to this paycheck (`import_source = 'paycheck'`, `type = 'income'`, `paycheck_id = id`)
- Create a new income transaction:
  - `type = 'income'`
  - `category = 'fixed'` (paycheck income is recurring)
  - `amount = net_amount`
  - `date = pay_date`
  - `description = 'Paycheck – {employer}'`
  - `account_id = income_account_id`
  - `import_source = 'paycheck'`
  - `paycheck_id = paycheck.id`

When a paycheck is deleted and had `income_account_id` set, the linked income transaction is also deleted.

On update, the old linked income txn is deleted before the new one is created — same lifecycle as contribution transactions.

**Paycheck form change:** new optional "Income Account" dropdown (account selector, same ComboboxInput pattern used elsewhere). Positioned after the net amount field. Label: "Deposit to account". No validation required — it's optional.

---

## Rust Commands

All follow the existing pattern: testable `async fn(conn, …)` inner function + thin `#[tauri::command]` `_cmd` wrapper. Row mapping by column index, not `de::from_row`.

### `list_budget_months_cmd`

Returns `Vec<{year: i32, month: i32}>` — all year/month pairs that have at least one transaction. Used to populate the month picker. Query:

### `list_budget_txns_cmd(year: i32, month: i32)`

Returns `Vec<Transaction>` — all transactions for the given year/month (all types and categories). Used by the budget store to load data for a selected month. Query:

```sql
SELECT * FROM txn
WHERE strftime('%Y', date) = printf('%04d', ?1)
  AND strftime('%m', date) = printf('%02d', ?2)
ORDER BY date ASC
```

```sql
SELECT DISTINCT CAST(strftime('%Y', date) AS INTEGER) AS year,
                CAST(strftime('%m', date) AS INTEGER) AS month
FROM txn
ORDER BY year DESC, month DESC
```

### `get_budget_month_target_cmd(year: i32, month: i32)`

Returns `Option<BudgetMonthTarget>` where:

```rust
struct BudgetMonthTarget {
    savings_target: f64,
    source_year: i32,
    source_month: i32,
}
```

Uses a most-recent-at-or-before fallback query — if no record exists for the requested month, returns the most recent prior record:

```sql
SELECT savings_target, year, month
FROM budget_month
WHERE (year < ?1 OR (year = ?1 AND month <= ?2))
ORDER BY year DESC, month DESC
LIMIT 1
```

`source_year` and `source_month` let the UI distinguish "set for this month" from "inherited from a prior month."

### `set_budget_month_target_cmd(year: i32, month: i32, savings_target: f64)`

Upserts a `budget_month` row for the given year/month:

```sql
INSERT INTO budget_month (year, month, savings_target)
VALUES (?1, ?2, ?3)
ON CONFLICT(year, month) DO UPDATE SET savings_target = excluded.savings_target
```

---

## TypeScript Lib: `src/lib/budget/`

Pure functions, fully unit-tested (Vitest).

### Types

```ts
export type BudgetLineItem = {
  total: number
  transactions: Transaction[]
}

export type BudgetMonthSummary = {
  income: BudgetLineItem
  savings: BudgetLineItem
  fixed: BudgetLineItem
  discretionary: BudgetLineItem
  freeMoney: number        // income.total - savings.total - fixed.total
  freeMoneyRemaining: number  // freeMoney - discretionary.total
}

export type BudgetMonthTarget = {
  savingsTarget: number
  sourceYear: number
  sourceMonth: number
  isInherited: boolean     // true when sourceYear/sourceMonth != selected year/month
}
```

### `buildBudgetMonth(txns: Transaction[]): BudgetMonthSummary`

Partitions the provided transaction list by type/category (txns are pre-filtered to the selected month by the Rust layer):

- **income**: `type === 'income'`
- **savings**: `isContribution === true`
- **fixed**: `type === 'expense' && category === 'fixed'`
- **discretionary**: `type === 'expense' && category === 'discretionary'`

`freeMoney = income.total - savings.total - fixed.total`  
`freeMoneyRemaining = freeMoney - discretionary.total`

No savings target logic here — that lives in the store/page.

---

## Store: `src/stores/budget.ts`

```ts
const months = ref<{year: number, month: number}[]>([])
const selectedYear = ref<number>()
const selectedMonth = ref<number>()
const target = ref<BudgetMonthTarget | null>(null)
const summary = ref<BudgetMonthSummary | null>(null)
const activeSection = ref<'income' | 'savings' | 'fixed' | 'discretionary' | null>(null)
```

`load(year, month)` — calls `list_budget_txns_cmd(year, month)`, calls `get_budget_month_target_cmd(year, month)`, builds `BudgetMonthSummary` via `buildBudgetMonth(txns)`. Sets `activeSection` to `'income'` if not already set.

`setTarget(savingsTarget)` — calls `set_budget_month_target_cmd`, then re-fetches the target.

`loadMonths()` — calls `list_budget_months_cmd`, sets `months`.

---

## Page: `src/pages/Budget.vue`

### Layout

```
┌─────────────────────────────────────────────────────────────────┐
│  Budget                                          June 2026 ▾    │
├────────────┬──────────────┬────────────┬─────────────┬──────────┤
│  INCOME    │  − SAVINGS   │  − FIXED   │ = FREE MONEY│  DISCRET.│
│  $6,200    │  $1,958      │  $1,840    │  $2,402     │  $1,102  │
│  2 txns ▾  │  target $2k ▾│  5 txns ▾  │             │  $1,300 ▾│
├────────────┴──────────────┴────────────┴─────────────┴──────────┤
│  Detail panel (swaps content based on active column)            │
│  INCOME — 2 transactions                                        │
│  Jun 01  Paycheck – Acme Corp  Checking  $3,100                 │
│  Jun 15  Paycheck – Acme Corp  Checking  $3,100                 │
└─────────────────────────────────────────────────────────────────┘
```

### Formula row columns

| Column | Clickable | Sub-label |
|---|---|---|
| Income | ✓ | `{n} transactions` |
| − Savings | ✓ | `target ${amount}` or `${amount} (from {Mon YYYY})` (muted if inherited) + pencil icon |
| − Fixed | ✓ | `{n} transactions` |
| = Free Money | ✗ | — (non-interactive, green highlight) |
| Discretionary | ✓ | `${freeMoneyRemaining} remaining` (green if positive, red if negative) |

Clicking a column sets `activeSection` on the store. The Free Money column has no click handler. All five columns are always visible — no collapsing of the row itself.

### Savings target inline edit

The pencil icon in the Savings column opens a small inline input (popover or inline swap) to enter a new savings target. Submitting calls `store.setTarget(value)` and re-fetches. The inherited label ("from Jan 2026") disappears once a target is saved for the selected month.

### Detail panel

A table below the formula row: Date | Description | Account | Amount columns — same style as Contributions and Transactions pages. Shown for whichever `activeSection` is currently selected. Defaults to `income` on load.

Empty state per section: "No income transactions this month.", "No contributions this month.", etc.

### Month picker

Dropdown populated from `store.months`, formatted as `"June 2026"`. Defaults to current year/month on mount. If no months have data yet, shows "No data yet" and an empty state on the page.

---

## Fullscreen Default

`src-tauri/tauri.conf.json` window config: set `"fullscreen": true` (Tauri 2.x uses `fullscreen` under `windows[0]`). This makes the app launch maximized/fullscreen on first open.

---

## Navigation

Add "Budget" to the nav (between Contributions and Forecast). Route: `/budget`. The page is registered in `vue-router` like the other pages.

---

## Testing

**Unit tests (`src/lib/budget/index.test.ts`):**
- `buildBudgetMonth` correctly partitions transactions by type/category
- `freeMoney` and `freeMoneyRemaining` compute correctly
- Categories with no transactions return `total: 0, transactions: []`
- `isContribution=true` transactions are bucketed into savings regardless of type/category

**Rust tests (inner fn tests):**
- `get_budget_month_target` returns `None` when no rows exist
- `get_budget_month_target` returns the exact month's row when one exists
- `get_budget_month_target` returns the most recent prior row when the exact month has no entry
- `set_budget_month_target` creates a new row and upserts correctly

---

## Out of Scope for 2d

- Income target per month (only savings target is configurable)
- Exporting or printing the monthly summary
- Alerts when discretionary exceeds free money
- `transfer`-type transactions (excluded from all line items)
