# CSV Import — Generate Balance Snapshots

**Date:** 2026-06-16  
**Status:** Approved

---

## Overview

When importing transactions via CSV, the import wizard currently sets `updateBalance: false` for all rows, meaning no `AccountBalance` snapshots are created. This makes CSV import useful for spending/category/anti-budget history but leaves the net worth chart and account balance history empty unless snapshots are added manually.

This feature adds an opt-in "Generate balance snapshots from these transactions" toggle to the import wizard. When enabled, each imported transaction cascades the account balance forward from a known starting point, producing a snapshot per transaction. This works for both:

- **Historical backfill** — where no prior snapshots exist and a seed balance must be entered
- **Ongoing monthly import** — where a prior snapshot already exists and is picked up automatically

Only non-investment accounts (checking, savings, credit cards) get this option. Investment account balances fluctuate due to market movement independent of transactions, so snapshot generation is not meaningful there.

---

## UI Changes

### Step 2 — Sidebar (column mapping screen)

A new "Generate Balance Snapshots" section is added at the bottom of the Step 2 right-side sidebar, below "Save Mapping". It is only rendered when the selected account is non-investment (same `isInvestment()` check used in `TransactionForm`).

Contents:
- A toggle: "Generate balance snapshots from these transactions"
- When the toggle is on, one of two conditional sub-states:
  - **Prior snapshot exists** (any `AccountBalance` for the selected account with `recordedAt` ≤ earliest CSV transaction date): display a confirmation line — "Will cascade from your [date] snapshot of $X". No additional input.
  - **No prior snapshot found**: display a seed balance input — "No balance found before [earliest date] — enter a starting balance". Required before the user can proceed to import.

The "earliest CSV transaction date" is the minimum date across **all parsed rows** (not just selected ones), since this check occurs in Step 2 before the user can include/exclude rows. If a seed balance is entered and the user later deselects the early rows in Step 3, the seed snapshot is still inserted — it's a harmless extra data point.

The check runs against `accountsStore.allBalances`, which is already loaded on mount. No new API calls.

### Step 3 — Preview table (review screen)

When "Generate balance snapshots" is on, a **"Balance"** column appears on the right side of the preview table. It shows the projected running balance after each transaction, computed reactively on the frontend.

Computation rules:
- Starting value: seed balance entered (or the prior snapshot's balance if one exists)
- Rows sorted by date, oldest first, for the running total
- Only **checked (included)** rows affect the running total — toggling a row's checkbox recalculates reactively
- Transfer rows show the balance delta from the source account's perspective
- Column is hidden entirely when the toggle is off

The column is display-only; no new inputs.

---

## Logic Changes

### `ImportWizard.vue`

New reactive state:
- `generateSnapshots: ref<boolean>(false)` — toggle state
- `seedBalance: ref<number>(0)` — user-entered seed balance when no prior snapshot exists

New computed values:
- `priorSnapshot` — most recent `AccountBalance` for the selected account with `recordedAt` ≤ earliest parsed transaction date. Derived from `accountsStore.allBalances`.
- `needsSeed` — `true` when `generateSnapshots` is on and `priorSnapshot` is null
- `baseBalance` — `priorSnapshot?.balance ?? seedBalance.value` (the starting point for the running total)
- `runningBalances` — array of per-row projected balances, computed from `baseBalance` cascading through included rows sorted by date

Updated `confirmImport()`:

```
if (!generateSnapshots) {
  // existing path: bulkCreateTransactions with updateBalance: false
} else {
  // 1. If needsSeed, insert seed AccountBalance snapshot via addBalance API
  // 2. Sort selected rows by date ascending
  // 3. Call createTransaction individually for each row with updateBalance: true
}
```

No changes to `bulkCreateTransactions` — the existing path is unchanged.

### Rust (no changes)

The existing `create_transaction` + `materialize_snapshots` pipeline handles everything:
- Finds `base_balance` (most recent snapshot on or before transaction date)
- Inserts a new `AccountBalance` snapshot at the transaction date
- Links snapshot ID back to the transaction via `generated_balance_id`
- Handles transfers: both source and destination accounts get snapshots (`generated_balance_to_id`)

Processing sequentially oldest-first ensures each transaction sees the snapshot from the previous one. For same-date transactions, Rust's `base_balance` query uses `ORDER BY recorded_at DESC, id DESC` so later-inserted snapshots take precedence — same-date chaining works correctly.

---

## Edge Cases

| Scenario | Behavior |
|---|---|
| No rows selected | Toggle/seed still visible; import is a no-op either way |
| All rows unchecked except one | Running balance reflects only the one included row |
| Transfer row | Both accounts get snapshots via existing `materialize_snapshots`; preview shows source account delta |
| Transactions out of date order in CSV | Sorted oldest-first before processing; order in CSV is irrelevant |
| Same-date transactions | Processed sequentially; each sees the previous transaction's snapshot via `id DESC` tiebreaker |
| Prior snapshot exists, seed input shown then hidden | `seedBalance` ref is reset when `priorSnapshot` becomes non-null |
| Account type changes after toggle is set | `generateSnapshots` resets to false if new account is investment type |

---

## Files Affected

- `src/components/ImportWizard.vue` — all changes (toggle, seed input, balance column, import logic)
- No Rust changes
- No new API functions
- No new Rust commands
- No migrations
