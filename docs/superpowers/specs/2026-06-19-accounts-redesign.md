# Accounts Page Redesign

**Date:** 2026-06-19
**Status:** Approved

## Problem

The current `/accounts` page loads every balance snapshot for every account on mount (`list_all_balances`), then renders them all inline in expandable cards. This is slow to load and hard to scroll through as history accumulates.

## Goals

- Fast initial load regardless of snapshot history size
- Accounts listed by FIRE / non-FIRE group with summary stats
- Snapshot history accessible per-account on a dedicated detail page
- Balance history chart on the detail page that switches context based on accordion state

---

## Page 1: Accounts List (`/accounts`)

### Layout

```
[Net Worth]  [FIRE Accounts]  [Non-FIRE Accounts]     + Add Account

FIRE Accounts
┌─────────────────────────────────────────────────────┐
│ Account               │ Type      │ Balance    │ ⋯  │
│ Fidelity 401K · Fid.  │ 401(k)    │ $62,569.91 │ ⋯  │
│ Wealthfront           │ HY Savings│ $15,049.87 │ ⋯  │
└─────────────────────────────────────────────────────┘

Non-FIRE Accounts
┌─────────────────────────────────────────────────────┐
│ Account               │ Type      │ Balance    │ ⋯  │
│ PNC Bank · PNC        │ Checking  │  $8,886.30 │ ⋯  │
└─────────────────────────────────────────────────────┘

▶ Archived (2)
```

### Stat Cards

Three cards above the tables:
- **Net Worth** — sum of latest balances across all active accounts (assets minus liabilities)
- **FIRE Accounts** — sum of latest balances for `includeInFireCalculations = true` accounts
- **Non-FIRE Accounts** — sum of latest balances for `includeInFireCalculations = false` accounts

### Account Tables

Each section renders a table. Columns: Account (name + institution as subtitle), Type, Balance (right-aligned), ⋯.

- Clicking anywhere on a row (except the ⋯ button) navigates to `/accounts/:id`
- The ⋯ button opens a dropdown menu with **Edit** (opens AccountForm modal) and **Archive** (confirms + archives)
- Row hover state highlights the full row

### Archived Section

Collapsible at the bottom, collapsed by default. Archived rows show the same columns and ⋯ menu but with **Restore** and **Delete** actions instead of Archive.

### Data Loading

Replace the current `list_all_balances` call with a new `list_latest_balances_cmd` that returns only the single most recent snapshot per account. This is sufficient for all display needs on the list page (stat cards + per-row balance). The store's `allBalances` ref is replaced with `latestBalances`.

---

## Page 2: Account Detail (`/accounts/:id`)

### Layout

```
← Accounts

Fidelity 401K (Gardner)                    [Edit]  [Archive]
401(k) · Fidelity · FIRE ✓

┌─────────────────────────────────────────────────────┐
│ Balance History — Monthly           (or: Jun 2025)  │
│                                                     │
│  [line chart]                                       │
└─────────────────────────────────────────────────────┘

Balance Snapshots                         [+ Add Snapshot]

┌─────────────────────────────────────────────────────┐
│ ▶ June 2025          3 snapshots · $62,569.91       │
│ ▶ May 2025           5 snapshots · $60,102.44       │
│ ▶ April 2025         2 snapshots · $57,813.00       │
└─────────────────────────────────────────────────────┘
```

### Header

- Back link `← Accounts` navigates to `/accounts`
- Account name (h1), type + institution + FIRE status line
- **Edit** opens AccountForm modal; **Archive** confirms + archives and redirects to `/accounts`
- **Delete** is available via a danger zone below the accordion (requires explicit confirmation)

### Chart Area

One chart, two modes, same space:

| Accordion state | Chart shown |
|---|---|
| All months collapsed | Monthly history — one point per month (latest snapshot balance of that month), from `list_balance_month_summaries_cmd` |
| One month expanded | Intra-month — one point per snapshot in that month, from the lazy-loaded snapshot data for the open month |

Only one month can be open at a time. Opening a month collapses any other open month and switches the chart.

Chart uses `@unovis/vue` VisLine/VisXYContainer consistent with the existing NetWorthChart pattern.

### Balance Snapshots Accordion

**On mount:** call `list_balance_month_summaries_cmd(account_id)` → array of `{ month: "2025-06", count: 3, latest_balance: 62569.91 }` sorted descending. These populate the accordion headers immediately.

**On expand:** call `list_balances_for_month_cmd(account_id, "2025-06")` → `AccountBalance[]` sorted descending by date. Results are cached in a `Map<string, AccountBalance[]>` keyed by month string — subsequent opens of the same month use the cache.

**Snapshot row:** date (formatted), balance (currency), optional 🧾 icon if `linkedTransactionId` is set (opens TransactionDetail modal on click), edit pencil (inline edit: date + balance fields in-row), delete trash (confirms then removes).

### Add Snapshot

`+ Add Snapshot` button opens a modal containing the existing `BalanceForm` component. After save: refresh month summaries + invalidate the cache entry for the affected month.

---

## Backend Changes

### New Rust models / types

```rust
pub struct BalanceMonthSummary {
    pub month: String,        // "YYYY-MM"
    pub count: i64,
    pub latest_balance: f64,  // balance of the most recent snapshot in this month
}
```

### New Rust commands

#### `list_latest_balances_cmd()`
Returns one `AccountBalance` per account — the most recent snapshot by `recorded_at`. Used by the list page to display per-row balance and compute stat card totals.

```sql
SELECT b.id, b.account_id, b.balance, b.recorded_at, t.id
FROM account_balance b
INNER JOIN (
  SELECT account_id, MAX(recorded_at) AS max_date
  FROM account_balance
  GROUP BY account_id
) latest ON b.account_id = latest.account_id AND b.recorded_at = latest.max_date
LEFT JOIN txn t ON t.generated_balance_id = b.id OR t.generated_balance_to_id = b.id
```

#### `list_balance_month_summaries_cmd(account_id: i32)`
Returns one row per month for the given account, sorted descending. The correlated subquery runs once per distinct month (not per row), so performance is proportional to the number of months, not total snapshots.

```sql
SELECT
  strftime('%Y-%m', recorded_at) AS month,
  COUNT(*) AS count,
  (SELECT balance FROM account_balance b2
   WHERE b2.account_id = ?1
     AND strftime('%Y-%m', b2.recorded_at) = strftime('%Y-%m', account_balance.recorded_at)
   ORDER BY b2.recorded_at DESC LIMIT 1) AS latest_balance
FROM account_balance
WHERE account_id = ?1
GROUP BY month
ORDER BY month DESC
```

#### `list_balances_for_month_cmd(account_id: i32, month: String)`
Returns all `AccountBalance` rows for a given account and month, sorted descending.

```sql
SELECT b.id, b.account_id, b.balance, b.recorded_at, t.id
FROM account_balance b
LEFT JOIN txn t ON t.generated_balance_id = b.id OR t.generated_balance_to_id = b.id
WHERE b.account_id = ?1
  AND strftime('%Y-%m', b.recorded_at) = ?2
ORDER BY b.recorded_at DESC
```

### New TS API functions (`src/lib/api/accounts.ts`)

```ts
listLatestBalances(): Promise<AccountBalance[]>
listBalanceMonthSummaries(accountId: number): Promise<BalanceMonthSummary[]>
listBalancesForMonth(accountId: number, month: string): Promise<AccountBalance[]>
```

### New TS type (`src/lib/types/BalanceMonthSummary.ts`)

```ts
export type BalanceMonthSummary = { month: string; count: number; latestBalance: number }
```

---

## Frontend Changes

### Router

Add route: `{ path: '/accounts/:id', name: 'account-detail', component: () => import('./pages/AccountDetail.vue') }`

### Store (`src/stores/accounts.ts`)

`allBalances` must be preserved — `Dashboard.vue` passes it to `activeFireInputs` to build the net-worth-over-time series, which needs every historical snapshot.

Changes:
- Add `latestBalances` ref (`AccountBalance[]`) — one entry per account
- Add `loadList()` action: fetches accounts + latest balances only (fast path for the list page)
- `load()` remains unchanged: fetches accounts + all balances (used by Dashboard)
- `Accounts.vue` calls `store.loadList()` on mount instead of `store.load()`
- `Dashboard.vue` continues to call `store.load()`

### New page: `src/pages/AccountDetail.vue`

Handles: mount load (summaries), accordion expand (lazy fetch + cache), chart data derivation, inline snapshot editing, delete account with redirect.

### Modified: `src/pages/Accounts.vue`

- Remove BalanceForm, balance history table, editingBalance state, all per-snapshot logic
- Render stat cards using `latestBalances`
- Render FIRE / non-FIRE tables with ⋯ dropdown menus
- Render archived collapsible section
- Row click → `router.push({ name: 'account-detail', params: { id: account.id } })`

---

## Out of Scope

- Pagination or infinite scroll on the detail page (month-level lazy load is sufficient)
- Charts on the list page (summary stats are enough there)
- Any changes to how snapshots are created at the Rust layer
