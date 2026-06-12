# TrackMyFI — Phase 2b: Paychecks (Design)

**Date:** 2026-06-12
**Phase:** 2 (Transactions, Paychecks & Budget) → **Slice 2b: Paychecks**
**Builds on:** Phase 2a (Transactions, `txn` table, auto-generated balance snapshots) — built & merged 2026-06-12.
**Stack:** Tauri 2.x · Vue 3 · NuxtUI · libSQL embedded replica · ts-rs · Pinia · Vitest · Luxon

---

## 1. Scope & approach

Paychecks are a standalone entity — distinct from the transaction ledger — that capture full paycheck breakdowns (gross, net, taxes, deductions, employer match). They feed the anti-budget (2d) as the income source and auto-generate contribution transactions in `txn` so the Contributions view (2c) stays current without duplicate data entry.

**What's in scope for 2b:**
- Manual paycheck entry and editing (no CSV import — deferred indefinitely; not worth the complexity vs. usage)
- Deductions with optional auto-contribution creation (account picker per qualifying deduction)
- Employer match section on the paycheck form (auto-creates contribution transactions)
- Paychecks list workbench with date filter and summary stats
- Rust commands: list, get, create, update, delete

**What's out of scope:**
- CSV import (dropped — see decision below)
- Contributions view — slice 2c

---

## 2. Key decisions

### No CSV import for paychecks
Paycheck CSV (ADP/Gusto/Paychex) requires mapping deduction columns as repeating groups — meaningfully harder than transaction CSV and unlikely to be used frequently enough to justify the complexity. Manual entry is sufficient; CSV can be revisited as a future addition if the need arises.

### Account picker per contribution deduction
When a deduction has a `contributionAccountType` (e.g. `401k`), the form shows an account dropdown filtered to accounts of that type. If the user has one matching account it auto-selects; if multiple, they choose explicitly. The `accountId` lives on the deduction itself. A contribution transaction is only auto-created when **both** `contributionAccountType` and `accountId` are set — deductions without an account linked are stored but skipped at contribution-creation time.

### Employer match on the paycheck form
Employer match entries live in a dedicated section below deductions (not in the deductions array). Each match item has a label, amount, and account picker. On save they auto-create contribution transactions (`isContribution=true`) alongside deduction contributions. This captures everything in one form entry.

### Taxes as dedicated columns (fully split)
Five dedicated `REAL` columns — one per paystub line — instead of a catch-all `fica_tax`. Enables SQL-native annualization (`SUM(federal_tax)` etc. grouped by year). Matches actual paystub layout.

### Contributions are already solved by the `txn` table
Annualizing contributions (e.g. total 401k contributions YTD) is handled by querying `txn` where `is_contribution=1` joined with `account.type`. The deductions JSON is a display artifact for the form; the live analytical data lives in `txn`. This is exactly what Phase 2c (Contributions view) will query.

### `payPeriod` is a fixed enum
`weekly | biweekly | semimonthly | monthly | irregular`. The `irregular` value covers Stripe payouts, freelance payments, bonuses, and any other non-regular income. In the anti-budget (2d), regular-frequency paychecks can be projected forward; `irregular` ones count as actual income only when they arrive.

### Auto-contribution lifecycle mirrors transaction linked-snapshot pattern
On `create_paycheck`: auto-create contribution txns. On `update_paycheck`: delete all `txn` rows where `paycheck_id = id`, then recreate from updated state. On `delete_paycheck`: cascade handles cleanup automatically via FK. A new `importSource` value `'paycheck'` marks auto-created txns for provenance.

---

## 3. Data model & schema

New migration `src-tauri/migrations/0003_paychecks.sql`.

### `paycheck` table

| Column | Type | Notes |
|---|---|---|
| `id` | INTEGER PK | i32 (ts-rs → `number`) |
| `pay_date` | TEXT | ISO `yyyy-MM-dd` |
| `employer` | TEXT | |
| `pay_period` | TEXT | `weekly\|biweekly\|semimonthly\|monthly\|irregular` |
| `gross_amount` | REAL | |
| `net_amount` | REAL | |
| `federal_tax` | REAL | Default 0 |
| `state_tax` | REAL | Default 0 |
| `local_tax` | REAL | Default 0 |
| `social_security_tax` | REAL | Default 0 |
| `medicare_tax` | REAL | Default 0 |
| `deductions` | TEXT | JSON: `PaycheckDeduction[]` |
| `employer_match` | TEXT | JSON: `EmployerMatchItem[]` |
| `import_source` | TEXT | `manual` (CSV dropped) |
| `created_at` | TEXT | |
| `updated_at` | TEXT | |

### `txn` table change

```sql
ALTER TABLE txn ADD COLUMN paycheck_id INTEGER REFERENCES paycheck(id) ON DELETE CASCADE;
```

Contribution transactions auto-created by a paycheck carry `paycheck_id` so edit/delete can find and clean them up.

### JSON sub-types

**`PaycheckDeduction`**
```
{ label: string, amount: number, preTax: boolean, contributionAccountType?: string, accountId?: number }
```
`contributionAccountType` values match `account.type` (e.g. `401k`, `roth_401k`, `hsa`, `roth_ira`, `traditional_ira`). A contribution transaction is only created when both fields are set.

**`EmployerMatchItem`**
```
{ label: string, amount: number, accountId?: number }
```
A contribution transaction is created when `accountId` is set.

Both types are Rust structs with `Serialize + Deserialize + TS`, exported to `src/lib/types/`. Read from the DB as JSON strings and deserialized before returning to the frontend — the TS side always sees typed arrays, never raw JSON strings.

---

## 4. Rust commands

New file `src-tauri/src/commands/paychecks.rs`. Follows established pattern: testable inner `async fn(conn, …)` + thin `#[tauri::command]` `_cmd` wrapper, manual row mapping by column index, booleans as INTEGER, `#[serde(rename_all = "camelCase")]`, ts-rs types emitted to `src/lib/types/`.

| Command | Description |
|---|---|
| `list_paychecks(filter?)` | Returns `Vec<Paycheck>`, filter by date range + optional employer search. Sorted `pay_date DESC`. |
| `get_paycheck(id)` | Single paycheck by id. |
| `create_paycheck(NewPaycheck)` | Inserts paycheck row; serializes deductions/match to JSON; auto-creates contribution txns; returns created `Paycheck`. |
| `update_paycheck(UpdatePaycheck)` | Updates row; deletes all `txn` where `paycheck_id = id`; recreates contributions from updated state; returns updated `Paycheck`. |
| `delete_paycheck(id)` | Deletes paycheck; linked contribution txns cascade automatically. |

### Auto-contribution creation (shared by create and update)

For each `deduction` where `contributionAccountType.is_some() && accountId.is_some()`:
- Insert txn: `type='income'`, `category='savings'`, `is_contribution=1`, `amount=deduction.amount`, `date=pay_date`, `description=deduction.label`, `account_id=deduction.accountId`, `paycheck_id=paycheck_id`, `import_source='paycheck'`.

For each `employer_match` item where `accountId.is_some()`:
- Same as above using match item's label/amount/accountId.

### `NewPaycheck` / `UpdatePaycheck` request structs

`NewPaycheck` carries all fields plus `deductions: Vec<PaycheckDeduction>` and `employer_match: Vec<EmployerMatchItem>` plus `created_at`. `UpdatePaycheck` is the same minus `import_source` plus `updated_at`. Casing is serde-controlled via `#[serde(rename_all = "camelCase")]`.

---

## 5. Frontend

Follows the `src/lib/api/*` + `src/stores/*` layout from Phase 1/2a.

### New files

| Path | Purpose |
|---|---|
| `src/lib/types/Paycheck.ts` | ts-rs generated |
| `src/lib/types/PaycheckDeduction.ts` | ts-rs generated |
| `src/lib/types/EmployerMatchItem.ts` | ts-rs generated |
| `src/lib/api/paychecks.ts` | `invoke()` wrappers for all 5 commands |
| `src/lib/paychecks/index.ts` | Pure TS helpers: `contributionItems()`, `paycheckTotals()` |
| `src/lib/paychecks/index.test.ts` | Vitest unit tests |
| `src/stores/paychecks.ts` | Pinia store — list, CRUD actions |
| `src/pages/Paychecks.vue` | List workbench |
| `src/components/PaycheckForm.vue` | Create/edit form |

Router and nav both gain the `Paychecks` entry.

### `Paychecks.vue` (list workbench)

Paginated table sorted by `pay_date DESC`. Filters: date range, employer text search. Summary stat cards: total gross, total net, paycheck count for filtered set. Row actions: edit (opens `PaycheckForm`), delete (confirm dialog). Follows the same workbench pattern as `Transactions.vue`.

### `PaycheckForm.vue` (create/edit)

Single scrollable form — no wizard, no steps. Five sections:

1. **Paycheck info** — `payDate` (DateInput), `employer` (text), `payPeriod` (select: weekly/biweekly/semimonthly/monthly/irregular)
2. **Amounts** — `grossAmount`, `netAmount`
3. **Taxes withheld** — `federalTax`, `stateTax`, `localTax`, `socialSecurityTax`, `medicareTax`
4. **Deductions** — dynamic list; "+ Add" button appends a row. Each row: `label` (text), `amount` (number), `preTax` (toggle). When `contributionAccountType` is set, the row expands inline to show a `contributionAccountType` select + an `accountId` dropdown filtered to accounts of that type (accounts store already loaded). Non-contribution rows stay compact.
5. **Employer Match** — dynamic list; "+ Add" button. Each row: `label`, `amount`, `accountId` dropdown (all investment account types).

Below the sections, a **contribution preview panel** shows which transactions will be created on save (one line per qualifying deduction + match item: label → account name, amount). Updates reactively as the form changes. Empty when no contributions are linked.

---

## 6. Pure TS helpers (`src/lib/paychecks/`)

Framework-free, Tauri-independent, fully unit-tested:

**`contributionItems(deductions, employerMatch)`**
Returns the subset that will auto-create transactions: deductions with both `contributionAccountType` and `accountId` set; match items with `accountId` set. Powers the contribution preview panel.

**`paycheckTotals(paychecks)`**
Returns `{ totalGross, totalNet, count }` for an array of paychecks. Used by the list workbench summary stats.

---

## 7. Testing

### Pure TS (Vitest)
- `contributionItems`: deduction with both fields set → included; missing `accountId` → excluded; missing `contributionAccountType` → excluded; match item with `accountId` → included; empty arrays → empty result.
- `paycheckTotals`: correct sums, empty array → zeros.

### Rust round-trip
- `create_paycheck` auto-creates correct contribution txns (count, `paycheck_id`, `is_contribution`, amount, date, account, `import_source='paycheck'`).
- Deduction with `contributionAccountType` but no `accountId` → no txn created.
- `update_paycheck` deletes old contributions and recreates from updated state.
- `delete_paycheck` cascades — linked txns removed.
- `list_paychecks` date range filter returns correct subset.
- `0003` migration test (same pattern as `0001`/`0002`).

### Manual GUI smoke test
`npm run tauri dev` — cannot run headless; same caveat as Phase 1/2a.

---

## 8. Build order (sub-slices within 2b)

1. Schema (`0003_paychecks.sql` + `ALTER TABLE txn`) + Rust commands + ts-rs types.
2. Pure TS helpers (`contributionItems`, `paycheckTotals`) + Vitest tests.
3. Paychecks store + API bindings.
4. `PaycheckForm.vue` — static fields first, then dynamic deduction/match rows, then contribution preview.
5. `Paychecks.vue` list workbench.
6. Router + nav wiring.

---

## 9. Assumptions / non-goals (this slice)

- CSV import for paychecks is dropped (not a future-planned item — revisit only if clearly needed).
- No validation that `grossAmount - taxes - deductions ≈ netAmount`; the form is informational.
- The Contributions view (2c) and Anti-Budget (2d) are out of scope here.
- `paycheck.import_source` is always `'manual'` in this slice (no CSV import). Distinct from `txn.import_source='paycheck'`, which marks auto-created contribution transactions.
