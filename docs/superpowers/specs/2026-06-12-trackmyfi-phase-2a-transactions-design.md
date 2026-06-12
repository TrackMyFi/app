# TrackMyFI — Phase 2a: Transactions (Design)

**Date:** 2026-06-12
**Phase:** 2 (Transactions, Paychecks & Budget) → **Slice 2a: Transactions**
**Builds on:** Phase 1 (FireProfile, Accounts, AccountBalance snapshots, Dashboard) — built & merged 2026-06-12.
**Stack:** Tauri 2.x · Vue 3 · NuxtUI · libSQL embedded replica · ts-rs · Pinia · Vitest · Luxon

---

## 1. Scope & approach

Phase 2 is four distinct subsystems with a natural dependency chain. We **decompose Phase 2 into vertical slices** (same as Phase 1), build them in order, and give each its own spec → plan → branch. We design the whole phase at a high level here and design **slice 2a (Transactions)** in full.

| # | Slice | Builds on | Net-new tables |
|---|---|---|---|
| **2a** | **Transactions** (this spec) | Accounts/Balances | `transaction`, `import_mapping` |
| 2b | Paychecks | Transactions (auto-creates contribution txns) | `paycheck` (+ `deductions` JSON) |
| 2c | Contributions | Transactions (`isContribution`) + Paychecks | none — derived view + IRS-limits constant |
| 2d | Anti-Budget | Transactions + Paychecks | `budget_month` |

Slices 2b–2d stay at the one-line level above and get their own brainstorm later, absorbing what we learn from 2a. Nav gains a **Transactions** item this slice.

---

## 2. Core model decisions

### Transactions are an informational ledger (not balance-derived)
Account balances remain **snapshot-only** (the Phase 1 decision stands). Transactions are a parallel ledger for income/spending analysis, contributions, savings-rate precision, and the anti-budget. A transaction does **not** implicitly change any balance.

### Opt-in balance materialization (linked snapshots)
The transaction form has an opt-in **"Update balance" switch** with an info panel + live preview. When **on**, saving the transaction also writes a new `AccountBalance` snapshot:

> new snapshot = *(most recent snapshot on/before the transaction's `date`)* **+ signed delta**, dated at the transaction `date`.

The snapshot stays the single source of truth for balances; the transaction just optionally materializes its effect, recording the change in **both** the ledger and the net-worth history.

- **Default:** switch **on** for cash/liability account types, **off** for investment types (their balances come from statements).
- **Linked, not decoupled:** the transaction stores `generatedBalanceId` (and `generatedBalanceToId` for transfers). Editing amount/date/account **re-applies** the delta to the linked snapshot(s); deleting the transaction **removes** them. This honors "recorded at both locations" and prevents silent drift.
- The server (`create/update_transaction`) is authoritative for the write; the form's preview mirrors the exact same math client-side.

### Transfers are a single row
A transfer is one event: one `transaction` row with `accountId` (source) + `transferAccountId` (destination), one positive `amount`. When the balance switch is on it writes **two** snapshots (source −, destination +). Transfers are **excluded** from income/expense/net totals so they never double-count or distort savings-rate math.

### Signed-delta convention
`amount` is stored as a **positive magnitude**; direction derives from `type`:
- `income` → `+amount`
- `expense` → `−amount`
- `transfer` → `−amount` on source, `+amount` on destination

### Categories: fixed enum
Coarse anti-budget buckets only — **not** YNAB-style granular categories. `category ∈ { savings, fixed, discretionary, uncategorized }`. `type ∈ { income, expense, transfer }`. No category-management UI.

---

## 3. Data model & schema

New migration `src-tauri/migrations/0002_transactions.sql` + a `Migration` entry in `src-tauri/src/migrations.rs` (hand-rolled ordered-SQL runner from Phase 1).

### `transaction`
| Column | Type | Notes |
|---|---|---|
| `id` | INTEGER PK | i32 (ts-rs → `number`) |
| `accountId` | INTEGER → account | primary/source account |
| `transferAccountId` | INTEGER? → account | destination; only when `type = transfer` |
| `amount` | REAL | positive magnitude; sign from `type` |
| `description` | TEXT | |
| `date` | TEXT | ISO `yyyy-MM-dd` |
| `type` | TEXT | `income` \| `expense` \| `transfer` |
| `category` | TEXT | `savings` \| `fixed` \| `discretionary` \| `uncategorized` |
| `isContribution` | INTEGER | bool (read `i64 != 0`); powers 2c Contributions |
| `importSource` | TEXT | `manual` \| `csv` |
| `generatedBalanceId` | INTEGER? → account_balance | snapshot written by the switch (source side) |
| `generatedBalanceToId` | INTEGER? → account_balance | destination-side snapshot (transfers only) |
| `createdAt` | TEXT | |
| `updatedAt` | TEXT | |

### `import_mapping` (saved named CSV mappings)
| Column | Type | Notes |
|---|---|---|
| `id` | INTEGER PK | i32 |
| `name` | TEXT UNIQUE | user-facing mapping name |
| `config` | TEXT | JSON: column→field map, date format, amount-sign convention, has-header flag, default account/category |
| `createdAt` | TEXT | |

Two nullable `generated*Id` columns (vs. a join table) are the simpler choice for a single-user app and cover the at-most-two-snapshots-per-transaction case exactly.

---

## 4. Rust commands

`src-tauri/src/commands/transactions.rs` + `import_mappings.rs`, following the Phase 1 pattern: testable inner `async fn(conn, …)` + thin `#[tauri::command]` `_cmd` wrapper, **manual** row mapping by column index, booleans as INTEGER, `#[serde(rename_all = "camelCase")]`, `ts-rs` types emitted to `src/lib/types/`. Request structs (`NewTransaction`, `UpdateTransaction`, `TransactionFilter`, `NewImportMapping`) keep casing serde-controlled.

- `list_transactions(filter)` — filter by account / type / category / date-range + description search; returns rows **and** filtered running totals (income / expense / net, transfers excluded). Pagination via limit/offset.
- `create_transaction(NewTransaction)` — inserts the row; if `updateBalance`, computes base (latest snapshot on/before `date`) + signed delta, inserts snapshot(s), back-links `generatedBalanceId` / `generatedBalanceToId`. Transfers insert two.
- `update_transaction(UpdateTransaction)` — re-applies delta to linked snapshot(s) on amount/date/account change; creates/removes snapshots when the switch is toggled.
- `delete_transaction(id)` — removes the row and any linked generated snapshot(s).
- `bulk_create_transactions(rows)` — one batch for an entire CSV import; **never** writes balance snapshots.
- `list_import_mappings` / `create_import_mapping` / `delete_import_mapping`.

The balance base-lookup + delta + linked-snapshot inner function is the trickiest logic and gets the most Rust round-trip coverage.

---

## 5. Frontend

Mirrors the Phase 1 `src/lib/api/*` + `src/stores/*` layout.

- `src/lib/api/transactions.ts`, `src/lib/api/importMappings.ts` — `invoke()` wrappers.
- `src/stores/transactions.ts` (Pinia) — filtered list, totals, CRUD actions.
- `src/lib/csv/` — **pure TS, framework-free, Tauri-independent, fully unit-tested:**
  - `parseCsv` (papaparse)
  - `applyMapping(rows, config)` — date formats, amount-sign conventions, column→field map
  - `detectDuplicates(parsed, existing)` — same account + date + amount + description
- Pages/components:
  - `src/pages/Transactions.vue` — the list workbench
  - `src/components/TransactionForm.vue` — manual entry/edit, balance-update switch + live preview
  - `src/components/ImportWizard.vue` — multi-step CSV import
  - reuses existing `DateInput`, `StatCard`

**New dependency:** `papaparse` (+ `@types/papaparse`).

---

## 6. Key UX flows

### Transactions list workbench (`Transactions.vue`)
Paginated, sortable table. Filters: account, type, category, date range; text search on description. Inline quick-edit of `category` (fast triage of `uncategorized` imported rows). Row actions edit/delete via `TransactionForm`. Running totals for the filtered set (income / expense / net).

### CSV import wizard (`ImportWizard.vue`)
1. **Select file + account.** If a saved mapping matches or is chosen by name, skip to step 3.
2. **Map columns.** Table of detected headers + sample rows; map each field (date, amount, description; optional type/category); set date format + amount-sign convention. Option to **save mapping as a name** for reuse.
3. **Preview & dedup.** Parsed rows rendered as they'll be saved. Rows matching an existing transaction flagged **duplicate**, unchecked by default. Defaults: `type` from amount sign, `category = uncategorized`, `importSource = csv`.
4. **Confirm.** `bulk_create_transactions` inserts checked rows. **No snapshots written.**

### Balance-update switch (`TransactionForm.vue`)
Toggle + info panel; defaults on for cash/liability, off for investment. When on, live preview: *"Latest balance for {account} on/before {date}: $X → new snapshot: $Y"* (source + destination lines for a transfer). Computed client-side from the store's latest-snapshot data; the authoritative write is server-side.

---

## 7. Testing

- **Pure TS (Vitest)** — the bulk of coverage:
  - `csv/`: `parseCsv`, `applyMapping` (date formats, sign conventions, missing/extra columns), `detectDuplicates`.
  - signed-delta + balance-preview helper: income/expense/transfer, back-dated transactions, no-prior-snapshot case.
  - filtered running totals (transfers excluded).
- **Rust round-trip:** `create_transaction` with `updateBalance` on/off writes & back-links correct snapshot(s); `update` re-applies delta; `delete` unwinds; transfer writes two; `bulk_create` writes none; mapping CRUD; `0002` migration test.
- **Manual GUI smoke:** `npm run tauri dev` (cannot run headless), same as Phase 1.

---

## 8. Build order (sub-slices within 2a)

1. Schema + Rust commands + ts-rs types (transactions CRUD, no balance switch).
2. Manual entry form + list workbench (filters, search, totals, inline category edit).
3. Balance-update switch + linked-snapshot logic + preview.
4. CSV import (`csv/` lib → wizard → bulk insert → dedup) + saved mappings.

---

## 9. Assumptions / non-goals (this slice)

- CSV import does **not** trigger the balance switch (informational bulk insert only).
- Import defaults: `type` from amount sign, `category = uncategorized` (editable after).
- No per-bank preset parsers (generic column-mapping covers it).
- No custom/user-defined categories (anti-budget uses the fixed enum).
- Paychecks, Contributions, Anti-Budget are **out of scope** here — slices 2b–2d.
