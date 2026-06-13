# TrackMyFI — Phase 2c: Contributions View (Design)

**Date:** 2026-06-12
**Phase:** 2 (Transactions, Paychecks & Budget) → **Slice 2c: Contributions**
**Builds on:** Phase 2b (Paychecks, auto-created contribution `txn` rows) — built & merged 2026-06-12.
**Stack:** Tauri 2.x · Vue 3 · NuxtUI · libSQL embedded replica · ts-rs · Pinia · Vitest · Luxon

---

## 1. Scope & approach

The Contributions view is read-only — no new tables, no CRUD. It queries `txn WHERE is_contribution = 1` for a selected year and the prior year, aggregates in pure TypeScript, and displays a card grid (progress toward IRS limits) plus a grouped transaction table (drill-down by account type).

**What's in scope:**
- Year picker (defaults to current year), querying selected year + prior year only
- Card grid per account type group: YTD total, IRS limit, % used (progress bar), YoY delta, Trad/Roth breakdown
- Grouped transaction table below cards: one header block + rows per group, date DESC
- IRS limits: year-keyed TS constants with automatic fallback + "estimated from YYYY" banner
- Catch-up contributions: auto-applied from `fire_profile.current_age`, clearly labeled ("incl. $X,XXX catch-up")
- HSA self/family coverage toggle in Settings, stored on `fire_profile`
- Account types without IRS limits (brokerage, crypto) shown as cards and table groups, without limit/% columns

**What's out of scope:**
- Any write operations (contributions are created via Transactions or Paychecks)
- CSV import
- Employer match shown separately (employer match txns are `is_contribution=1` and roll into the account type total naturally)

---

## 2. Key decisions

### Grouping by account type, not individual account
IRS limits are per account type (e.g. all 401k contributions share one cap), and users typically have one active account per type. Grouping by type maps directly to IRS tracking.

### Shared-limit pairs merged into one row
- `401k` + `roth_401k` → "401k / Roth 401k" (shared $23,500 employee limit)
- `traditional_ira` + `roth_ira` → "Traditional / Roth IRA" (shared $7,000 limit)

Each merged row shows a Trad/Roth breakdown beneath the combined total so the split is visible for tax planning.

### Query scoped to selected year + prior year only
The Rust command accepts `year: i32` and filters `strftime('%Y', date) IN (?, ?)` with `year` and `year - 1`. Only two years of data loaded at any time; the store reloads when the year picker changes.

### TS-first aggregation (consistent with FIRE math pattern)
All grouping, limit merging, YoY delta, and IRS limit logic lives in pure TS helpers in `src/lib/contributions/`. The Rust command is a thin filter query returning `Vec<Transaction>` (the existing type). Account type is resolved in TS via the already-loaded accounts store.

### Year-keyed IRS constants + release cycle
IRS limits are hardcoded in `src/lib/contributions/irsLimits.ts` as a `Record<number, YearLimits>`. `resolveYearLimits(year)` walks backwards to find the most recent known year ≤ `year`. If the current year isn't in the table yet, `estimated: true` is returned and a subtle page banner appears: "IRS limits estimated from [year] — update `irsLimits.ts` when [selected year] limits are announced." Updated once annually (typically October/November) with a small release.

### Catch-up contributions auto-applied from profile age
`fire_profile.current_age` determines eligibility: age ≥ 50 for 401k (+$7,500) and IRA (+$1,000) catch-up; age ≥ 55 for HSA (+$1,000). The effective limit is displayed as e.g. "$31,000 (incl. $7,500 catch-up)" so the user can verify their age is correct.

### HSA coverage stored on `fire_profile`
A new `hsa_coverage TEXT NOT NULL DEFAULT 'self'` column on `fire_profile` (migration `0004`) controls whether the HSA limit uses the self-only or family tier. Toggled in Settings via a Self/Family select. Fits the single-row settings pattern already established.

### Layout: card grid + grouped transaction table
Top section: one card per account type group with progress bar, YTD total, limit, YoY delta, and Trad/Roth footnote. Bottom section: grouped transaction table using the card's account type as a group header row (showing the same summary stats), with individual txn rows (date, description, account name, import source, amount) sorted date DESC within each group. Account types with no contributions in either year are omitted entirely.

---

## 3. Data model

### New migration: `0004_hsa_coverage.sql`

```sql
ALTER TABLE fire_profile ADD COLUMN hsa_coverage TEXT NOT NULL DEFAULT 'self';
```

No other schema changes. All contribution data already exists in `txn`.

### `fire_profile` updates
- Rust struct: add `hsa_coverage: String`
- `upsert_fire_profile` command: include `hsa_coverage` in INSERT/UPDATE
- ts-rs regenerates `src/lib/types/FireProfile.ts` with `hsaCoverage: string`

---

## 4. Rust command

New file `src-tauri/src/commands/contributions.rs`. Follows the established pattern: testable inner `async fn(conn, year)` + thin `#[tauri::command]` `_cmd` wrapper.

```
list_contribution_txns(year: i32) → Vec<Transaction>
```

```sql
SELECT <txn cols>
FROM txn
WHERE is_contribution = 1
  AND strftime('%Y', date) IN (?1, ?2)
ORDER BY date DESC
```

Parameters: `year.to_string()`, `(year - 1).to_string()`.

Returns the existing `Transaction` type — no new ts-rs type needed.

---

## 5. IRS limits constants

**`src/lib/contributions/irsLimits.ts`**

```ts
interface YearLimits {
  k401: number          // 401k + roth_401k combined employee elective deferral
  ira: number           // traditional_ira + roth_ira combined
  hsaSelf: number
  hsaFamily: number
  k401CatchUpAge: number  // 50
  k401CatchUp: number     // additional amount (e.g. $7,500)
  iraCatchUpAge: number   // 50
  iraCatchUp: number      // additional amount (e.g. $1,000)
  hsaCatchUpAge: number   // 55
  hsaCatchUp: number      // additional amount (e.g. $1,000)
}

const IRS_LIMITS: Record<number, YearLimits> = {
  2024: { k401: 23000, ira: 7000, hsaSelf: 4150, hsaFamily: 8300,
          k401CatchUpAge: 50, k401CatchUp: 7500,
          iraCatchUpAge: 50, iraCatchUp: 1000,
          hsaCatchUpAge: 55, hsaCatchUp: 1000 },
  2025: { k401: 23500, ira: 7000, hsaSelf: 4300, hsaFamily: 8550,
          k401CatchUpAge: 50, k401CatchUp: 7500,
          iraCatchUpAge: 50, iraCatchUp: 1000,
          hsaCatchUpAge: 55, hsaCatchUp: 1000 },
  2026: { k401: 23500, ira: 7000, hsaSelf: 4400, hsaFamily: 8700,
          k401CatchUpAge: 50, k401CatchUp: 7500,
          iraCatchUpAge: 50, iraCatchUp: 1000,
          hsaCatchUpAge: 55, hsaCatchUp: 1000 },
}

export function resolveYearLimits(year: number): {
  limits: YearLimits
  estimated: boolean
  estimatedFrom?: number
}
```

---

## 6. TS aggregation layer

**`src/lib/contributions/index.ts`** — no Tauri deps, fully unit-testable.

### Output type

```ts
export interface ContributionRow {
  label: string           // "401k / Roth 401k", "HSA", "Brokerage", etc.
  accountTypes: string[]  // account types in this group
  total: number           // combined YTD for selected year
  breakdown?: {           // only for merged-limit pairs
    type: string
    label: string
    total: number
  }[]
  limit?: number          // effective limit (base + catch-up if eligible)
  limitBase?: number      // base limit without catch-up
  catchUpAmount?: number  // catch-up portion (defined when age qualifies)
  pctUsed?: number        // 0–1, only when limit is defined
  yoyDelta?: number       // thisYear.total - priorYear.total
}
```

### Main function

```ts
export function buildContributionRows(
  txns: Transaction[],
  accounts: Account[],
  year: number,
  age: number,
  hsaCoverage: 'self' | 'family',
  limits: YearLimits,
): ContributionRow[]
```

**Logic:**
1. Split `txns` into `thisYear` (date starts with `year`) and `priorYear` (date starts with `year - 1`)
2. Group each set by `account.type` (looked up via accounts array)
3. Sum amounts per type per year-set
4. Merge shared-limit pairs: combine `401k` + `roth_401k` totals into one row with `breakdown`; same for `traditional_ira` + `roth_ira`
5. Apply limits: resolve base limit and catch-up eligibility from `age`; set `limit`, `limitBase`, `catchUpAmount`, `pctUsed`
6. Compute `yoyDelta` for each row
7. Order: types with IRS limits first (`401k/roth_401k`, `ira`, `hsa`), then unlimited types (`brokerage`, `crypto`). Types with zero txns in both years are omitted.

---

## 7. Frontend

### New files

| Path | Purpose |
|---|---|
| `src/lib/contributions/irsLimits.ts` | Year-keyed IRS limits constants + `resolveYearLimits()` |
| `src/lib/contributions/index.ts` | `buildContributionRows()` and supporting helpers |
| `src/lib/contributions/index.test.ts` | Vitest unit tests |
| `src/lib/api/contributions.ts` | `invoke()` wrapper for `list_contribution_txns_cmd` |
| `src/stores/contributions.ts` | Pinia store — `selectedYear`, `txns`, `load(year)` |
| `src/pages/Contributions.vue` | Card grid + grouped transaction table + year picker |

### Modified files

| Path | Change |
|---|---|
| `src-tauri/migrations/0004_hsa_coverage.sql` | New migration |
| `src-tauri/src/commands/fire_profile.rs` | Add `hsa_coverage` to struct + upsert |
| `src-tauri/src/commands/contributions.rs` | New command file |
| `src-tauri/src/commands/mod.rs` | Register contributions module |
| `src-tauri/src/lib.rs` | Register `list_contribution_txns_cmd` in invoke_handler |
| `src/lib/types/FireProfile.ts` | ts-rs regenerated — adds `hsaCoverage` |
| `src/pages/Settings.vue` | HSA coverage Self/Family toggle |
| `src/router.ts` | Add `/contributions` route |
| `src/App.vue` | Enable Contributions nav link (remove `disabled: true`, add `to: '/contributions'`) |

### `Contributions.vue` structure

- `onMounted`: load accounts store, fire profile store (for `current_age` and `hsaCoverage`), contributions store for current year
- **Year picker**: `USelect` populated from the set of years present in all contribution txn dates (union of both years loaded), defaulting to `DateTime.now().year`. On change, calls `store.load(year)`
- **Estimated limits banner**: shown when `resolveYearLimits` returns `estimated: true` — "IRS limits estimated from [estimatedFrom] — update `irsLimits.ts` when [selectedYear] limits are announced"
- **Card grid**: one card per `ContributionRow`
  - Progress bar: green (< 80%), amber (80–99%), green (100%)
  - Shows: label, YTD total, limit string (e.g. "$31,000 (incl. $7,500 catch-up)"), % used, YoY delta (colored green/red/muted)
  - Trad/Roth breakdown shown as a footnote line (only for merged pairs)
  - Brokerage/crypto cards: omit progress bar, show "No IRS limit"
- **Grouped transaction table**: one section per `ContributionRow`, sorted by the row order above
  - Group header: label, YTD, limit, % used, YoY delta (mirrors the card summary)
  - Transaction rows: date, description, account name, import source badge ("via Paycheck" / "Manual"), amount

---

## 8. Testing

### Pure TS (Vitest) — `src/lib/contributions/index.test.ts`

- `buildContributionRows`: txns from two account types → two rows; merged 401k + roth_401k → one row with breakdown; type with zero txns omitted; types with IRS limits ordered before unlimited types
- Catch-up: age ≥ 50 → `catchUpAmount` set for 401k; age < 50 → no catch-up; age ≥ 55 → HSA catch-up applied
- `pctUsed`: correct ratio; above 100% clamped or uncapped (include txns recorded before limit was officially announced — just show > 100%)
- `yoyDelta`: positive when this year > prior year; negative; zero when no prior year data
- HSA coverage: `hsaFamily` used when `hsaCoverage === 'family'`

### `resolveYearLimits` (Vitest) — inline or separate test file

- Known year → returns exact limits, `estimated: false`
- Future unknown year → returns most recent year's limits, `estimated: true`, `estimatedFrom` set correctly
- Year older than all known → returns oldest known year

### Rust round-trip (within `contributions.rs` test module)

- `list_contribution_txns` with `year = 2026` returns only rows with dates in 2026 and 2025
- Rows with `is_contribution = 0` excluded
- Returns empty vec when no matching rows

### Migration test
- `0004` migration applies cleanly; `fire_profile.hsa_coverage` defaults to `'self'`

---

## 9. Build order

1. Migration `0004_hsa_coverage.sql` + update `fire_profile.rs` + regenerate `FireProfile.ts` + Settings HSA toggle
2. IRS limits constants (`irsLimits.ts`) + `resolveYearLimits` + tests
3. `buildContributionRows` helper + Vitest tests
4. Rust `list_contribution_txns_cmd` + Rust tests
5. API bindings + contributions Pinia store
6. `Contributions.vue` page
7. Router + nav wiring

---

## 10. Assumptions / non-goals

- `pctUsed` can exceed 1.0 — no clamping; over-contribution is a real scenario worth surfacing
- Employer match contributions are not broken out separately; they roll into the account type total (they are `is_contribution=1` txns on investment accounts)
- No editing of contribution transactions from this page — link to Transactions page is out of scope for this slice
- Year picker is bounded to years that have at least one contribution txn; no arbitrary year entry
- No support for age-based limit changes mid-year (birthday during the year) — the profile age is treated as the age for the full year
