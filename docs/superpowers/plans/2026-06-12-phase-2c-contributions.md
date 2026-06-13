# Phase 2c: Contributions View Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a read-only Contributions page that aggregates `is_contribution=1` transactions by account type for a selected year, showing progress toward IRS limits (with catch-up and HSA self/family coverage) plus a grouped transaction drill-down.

**Architecture:** A thin Rust command layer returns raw contribution `Transaction` rows for the selected year + prior year; all aggregation, limit-merging, catch-up, and YoY logic lives in pure TypeScript (`src/lib/contributions/`), fully unit-tested. A new `hsa_coverage` column on `fire_profile` (migration `0004`) drives the HSA limit tier, toggled in Settings. The page is a card grid + grouped table backed by a new Pinia store.

**Tech Stack:** Tauri 2.x · Vue 3 · NuxtUI · libSQL · ts-rs · Pinia · Vitest · Luxon · Rust/tokio tests

**Reference spec:** `docs/superpowers/specs/2026-06-12-phase-2c-contributions-design.md`

---

## File Structure

**Rust (backend):**
- `src-tauri/migrations/0004_hsa_coverage.sql` — new migration (create)
- `src-tauri/src/migrations.rs` — register migration 4 (modify)
- `src-tauri/src/models.rs` — add `hsa_coverage` to `FireProfile` (modify)
- `src-tauri/src/commands/fire_profile.rs` — read/write `hsa_coverage` (modify)
- `src-tauri/src/commands/contributions.rs` — new command file (create)
- `src-tauri/src/commands/mod.rs` — register module (modify)
- `src-tauri/src/lib.rs` — register two commands in invoke_handler (modify)
- `src-tauri/tests/contributions.rs` — Rust round-trip tests (create)
- `src-tauri/tests/roundtrip.rs` — fix `FireProfile` literal for new field (modify)

**TypeScript (frontend):**
- `src/lib/types/FireProfile.ts` — ts-rs regenerated (auto)
- `src/lib/contributions/irsLimits.ts` — year-keyed limits + `resolveYearLimits` (create)
- `src/lib/contributions/irsLimits.test.ts` — Vitest (create)
- `src/lib/contributions/index.ts` — `buildContributionRows` + helpers (create)
- `src/lib/contributions/index.test.ts` — Vitest (create)
- `src/lib/api/contributions.ts` — invoke wrappers (create)
- `src/stores/contributions.ts` — Pinia store (create)
- `src/pages/Contributions.vue` — page (create)
- `src/pages/Settings.vue` — HSA coverage toggle (modify)
- `src/router.ts` — add route (modify)
- `src/App.vue` — enable nav link (modify)

---

## Task 1: Migration + `hsa_coverage` column

**Files:**
- Create: `src-tauri/migrations/0004_hsa_coverage.sql`
- Modify: `src-tauri/src/migrations.rs`
- Modify: `src-tauri/src/models.rs:7-16`
- Modify: `src-tauri/src/commands/fire_profile.rs`
- Modify: `src-tauri/tests/roundtrip.rs:20-29`

- [ ] **Step 1: Write the migration SQL**

Create `src-tauri/migrations/0004_hsa_coverage.sql`:

```sql
ALTER TABLE fire_profile ADD COLUMN hsa_coverage TEXT NOT NULL DEFAULT 'self';
```

- [ ] **Step 2: Register the migration**

In `src-tauri/src/migrations.rs`, add a fourth entry to the `MIGRATIONS` array after the `paychecks` entry (closing `];` stays after it):

```rust
    Migration {
        version: 4,
        name: "hsa_coverage",
        sql: include_str!("../migrations/0004_hsa_coverage.sql"),
    },
```

- [ ] **Step 3: Add the field to the `FireProfile` model**

In `src-tauri/src/models.rs`, add `hsa_coverage` as the last field of the `FireProfile` struct (after `inflation_rate`):

```rust
pub struct FireProfile {
    pub current_age: i32,
    pub target_retirement_age: i32,
    pub annual_expenses_target: f64,
    pub lean_fire_annual_expenses: Option<f64>,
    pub fat_fire_annual_expenses: Option<f64>,
    pub annual_income: f64,
    pub expected_return_rate: f64,
    pub inflation_rate: f64,
    pub hsa_coverage: String,
}
```

- [ ] **Step 4: Read and write the column in `fire_profile.rs`**

In `src-tauri/src/commands/fire_profile.rs`, update `get_profile` — add `hsa_coverage` to the SELECT and map index 8:

```rust
pub async fn get_profile(conn: &Connection) -> Result<FireProfile, String> {
    let mut rows = conn
        .query(
            "SELECT current_age, target_retirement_age, annual_expenses_target, \
             lean_fire_annual_expenses, fat_fire_annual_expenses, annual_income, \
             expected_return_rate, inflation_rate, hsa_coverage FROM fire_profile WHERE id = 1",
            (),
        )
        .await
        .map_err(|e| e.to_string())?;
    let row = rows
        .next()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "fire_profile row missing".to_string())?;
    Ok(FireProfile {
        current_age: row.get(0).map_err(|e| e.to_string())?,
        target_retirement_age: row.get(1).map_err(|e| e.to_string())?,
        annual_expenses_target: row.get(2).map_err(|e| e.to_string())?,
        lean_fire_annual_expenses: row.get(3).map_err(|e| e.to_string())?,
        fat_fire_annual_expenses: row.get(4).map_err(|e| e.to_string())?,
        annual_income: row.get(5).map_err(|e| e.to_string())?,
        expected_return_rate: row.get(6).map_err(|e| e.to_string())?,
        inflation_rate: row.get(7).map_err(|e| e.to_string())?,
        hsa_coverage: row.get(8).map_err(|e| e.to_string())?,
    })
}
```

Update `upsert_profile` — add `hsa_coverage=?9` to the SET clause and bind `p.hsa_coverage.clone()`:

```rust
pub async fn upsert_profile(conn: &Connection, p: &FireProfile) -> Result<(), String> {
    conn.execute(
        "UPDATE fire_profile SET current_age=?1, target_retirement_age=?2, \
         annual_expenses_target=?3, lean_fire_annual_expenses=?4, fat_fire_annual_expenses=?5, \
         annual_income=?6, expected_return_rate=?7, inflation_rate=?8, hsa_coverage=?9 WHERE id = 1",
        libsql::params![
            p.current_age,
            p.target_retirement_age,
            p.annual_expenses_target,
            p.lean_fire_annual_expenses,
            p.fat_fire_annual_expenses,
            p.annual_income,
            p.expected_return_rate,
            p.inflation_rate,
            p.hsa_coverage.clone()
        ],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}
```

- [ ] **Step 5: Fix the `FireProfile` literal in `roundtrip.rs`**

The existing `fire_profile_roundtrip` test builds a `FireProfile` struct literal that will no longer compile. In `src-tauri/tests/roundtrip.rs`, add `hsa_coverage` to the `updated` literal (after `inflation_rate: 0.025,`) and assert the seeded default + updated value.

Add to the `updated` struct literal:

```rust
        hsa_coverage: "family".into(),
```

After `let p = get_profile(&conn).await.unwrap();` (around line 14), add an assertion for the seeded default:

```rust
    assert_eq!(p.hsa_coverage, "self");
```

After `let p2 = get_profile(&conn).await.unwrap();` (around line 31), add:

```rust
    assert_eq!(p2.hsa_coverage, "family");
```

- [ ] **Step 6: Run Rust tests to verify migration + roundtrip pass**

Run: `cd src-tauri && cargo test --test roundtrip --test migrations`
Expected: PASS. ts-rs regenerates `src/lib/types/FireProfile.ts` with `hsaCoverage: string` during the build.

- [ ] **Step 7: Verify the TS type regenerated**

Run: `grep hsaCoverage /Users/tomgobich/code/trackmyfi/src/lib/types/FireProfile.ts`
Expected: output contains `hsaCoverage: string`. If not, run `cd src-tauri && cargo test` once to trigger ts-rs export, then re-check.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/migrations/0004_hsa_coverage.sql src-tauri/src/migrations.rs src-tauri/src/models.rs src-tauri/src/commands/fire_profile.rs src-tauri/tests/roundtrip.rs src/lib/types/FireProfile.ts
git commit -m "feat: add hsa_coverage column to fire_profile"
```

---

## Task 2: Settings HSA coverage toggle

**Files:**
- Modify: `src/pages/Settings.vue`

This task has no unit test (it's a presentational form wired to the existing store, matching the rest of `Settings.vue` which is untested). Verify by typecheck + manual smoke.

- [ ] **Step 1: Add `hsaCoverage` to the form interface and reactive state**

In `src/pages/Settings.vue`, add `hsaCoverage: string` to the `FireProfileForm` interface (after `inflationRate: number`):

```ts
interface FireProfileForm {
  currentAge: number
  targetRetirementAge: number
  annualExpensesTarget: number
  leanFireAnnualExpenses: number | null
  fatFireAnnualExpenses: number | null
  annualIncome: number
  expectedReturnRate: number
  inflationRate: number
  hsaCoverage: string
}
```

Add `hsaCoverage: 'self',` to the `reactive` initializer (after `inflationRate: 0,`).

- [ ] **Step 2: Include `hsaCoverage` in the submitted profile**

In `onSubmit`, add `hsaCoverage: form.hsaCoverage,` to the `profile` object (after `inflationRate: form.inflationRate,`).

- [ ] **Step 3: Add the toggle to the template**

In `src/pages/Settings.vue`, add this `UFormField` after the inflation-rate field and before the Save `UButton`:

```html
      <UFormField label="HSA coverage">
        <USelect
          v-model="form.hsaCoverage"
          :items="[
            { label: 'Self-only', value: 'self' },
            { label: 'Family', value: 'family' },
          ]"
          class="w-44"
        />
      </UFormField>
```

(The `onMounted` `Object.assign(form, store.profile)` already copies `hsaCoverage` from the loaded profile — no change needed there.)

- [ ] **Step 4: Typecheck**

Run: `cd /Users/tomgobich/code/trackmyfi && npx vue-tsc --noEmit`
Expected: no errors related to `Settings.vue`.

- [ ] **Step 5: Commit**

```bash
git add src/pages/Settings.vue
git commit -m "feat: HSA coverage self/family toggle in Settings"
```

---

## Task 3: IRS limits constants + `resolveYearLimits`

**Files:**
- Create: `src/lib/contributions/irsLimits.ts`
- Test: `src/lib/contributions/irsLimits.test.ts`

- [ ] **Step 1: Write the failing test**

Create `src/lib/contributions/irsLimits.test.ts`:

```ts
import { describe, it, expect } from 'vitest'
import { resolveYearLimits } from './irsLimits'

describe('resolveYearLimits', () => {
  it('returns exact limits for a known year, not estimated', () => {
    const { limits, estimated } = resolveYearLimits(2025)
    expect(limits.k401).toBe(23500)
    expect(limits.ira).toBe(7000)
    expect(limits.hsaSelf).toBe(4300)
    expect(limits.hsaFamily).toBe(8550)
    expect(estimated).toBe(false)
  })

  it('falls back to most recent known year for a future year, flagged estimated', () => {
    const { limits, estimated, estimatedFrom } = resolveYearLimits(2099)
    expect(limits.k401).toBe(23500) // 2026 values (latest known)
    expect(estimated).toBe(true)
    expect(estimatedFrom).toBe(2026)
  })

  it('falls back to the oldest known year for a year older than all entries', () => {
    const { limits, estimated, estimatedFrom } = resolveYearLimits(2000)
    expect(limits.k401).toBe(23000) // 2024 values (oldest known)
    expect(estimated).toBe(true)
    expect(estimatedFrom).toBe(2024)
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /Users/tomgobich/code/trackmyfi && npx vitest run src/lib/contributions/irsLimits.test.ts`
Expected: FAIL — cannot find module `./irsLimits`.

- [ ] **Step 3: Write the implementation**

Create `src/lib/contributions/irsLimits.ts`:

```ts
export interface YearLimits {
  k401: number // 401k + roth_401k combined employee elective deferral
  ira: number // traditional_ira + roth_ira combined
  hsaSelf: number
  hsaFamily: number
  k401CatchUpAge: number
  k401CatchUp: number
  iraCatchUpAge: number
  iraCatchUp: number
  hsaCatchUpAge: number
  hsaCatchUp: number
}

// Update this table each year when the IRS announces new limits (typically Oct/Nov).
const IRS_LIMITS: Record<number, YearLimits> = {
  2024: {
    k401: 23000, ira: 7000, hsaSelf: 4150, hsaFamily: 8300,
    k401CatchUpAge: 50, k401CatchUp: 7500,
    iraCatchUpAge: 50, iraCatchUp: 1000,
    hsaCatchUpAge: 55, hsaCatchUp: 1000,
  },
  2025: {
    k401: 23500, ira: 7000, hsaSelf: 4300, hsaFamily: 8550,
    k401CatchUpAge: 50, k401CatchUp: 7500,
    iraCatchUpAge: 50, iraCatchUp: 1000,
    hsaCatchUpAge: 55, hsaCatchUp: 1000,
  },
  2026: {
    k401: 23500, ira: 7000, hsaSelf: 4400, hsaFamily: 8700,
    k401CatchUpAge: 50, k401CatchUp: 7500,
    iraCatchUpAge: 50, iraCatchUp: 1000,
    hsaCatchUpAge: 55, hsaCatchUp: 1000,
  },
}

export interface ResolvedLimits {
  limits: YearLimits
  estimated: boolean
  estimatedFrom?: number
}

/**
 * Resolve IRS limits for a year. If the exact year is known, returns it
 * un-estimated. Otherwise falls back to the nearest known year (clamped to the
 * available range) and flags `estimated` with the source year in `estimatedFrom`.
 */
export function resolveYearLimits(year: number): ResolvedLimits {
  if (IRS_LIMITS[year]) {
    return { limits: IRS_LIMITS[year], estimated: false }
  }
  const knownYears = Object.keys(IRS_LIMITS).map(Number).sort((a, b) => a - b)
  const min = knownYears[0]
  const max = knownYears[knownYears.length - 1]
  const source = year > max ? max : min
  return { limits: IRS_LIMITS[source], estimated: true, estimatedFrom: source }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /Users/tomgobich/code/trackmyfi && npx vitest run src/lib/contributions/irsLimits.test.ts`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add src/lib/contributions/irsLimits.ts src/lib/contributions/irsLimits.test.ts
git commit -m "feat: year-keyed IRS contribution limits with fallback"
```

---

## Task 4: `buildContributionRows` aggregation

**Files:**
- Create: `src/lib/contributions/index.ts`
- Test: `src/lib/contributions/index.test.ts`

This is the core of the slice. The function groups contribution txns by account type, merges shared-limit pairs, applies catch-up, and computes YoY. Build it test-first.

- [ ] **Step 1: Write the failing test**

Create `src/lib/contributions/index.test.ts`:

```ts
import { describe, it, expect } from 'vitest'
import { buildContributionRows } from './index'
import { resolveYearLimits } from './irsLimits'
import type { Transaction } from '../types/Transaction'
import type { Account } from '../types/Account'

const limits = resolveYearLimits(2025).limits

function acct(id: number, type: string): Account {
  return {
    id, name: `${type} account`, type, institution: null,
    isActive: true, includeInFireCalculations: true, createdAt: '2025-01-01',
  }
}

function txn(id: number, accountId: number, amount: number, date: string): Transaction {
  return {
    id, accountId, transferAccountId: null, amount, description: 'Contribution',
    date, type: 'income', category: 'savings', isContribution: true,
    importSource: 'manual', generatedBalanceId: null, generatedBalanceToId: null,
    paycheckId: null, createdAt: date, updatedAt: date,
  }
}

describe('buildContributionRows', () => {
  it('groups contributions by account type into one row per type', () => {
    const accounts = [acct(1, '401k'), acct(2, 'hsa')]
    const txns = [
      txn(10, 1, 1000, '2025-03-01'),
      txn(11, 2, 500, '2025-03-01'),
    ]
    const rows = buildContributionRows(txns, accounts, 2025, 30, 'self', limits)
    const k401 = rows.find((r) => r.label === '401k / Roth 401k')!
    const hsa = rows.find((r) => r.label === 'HSA')!
    expect(k401.total).toBe(1000)
    expect(hsa.total).toBe(500)
  })

  it('merges 401k + roth_401k into one row with a breakdown', () => {
    const accounts = [acct(1, '401k'), acct(2, 'roth_401k')]
    const txns = [
      txn(10, 1, 12000, '2025-06-01'),
      txn(11, 2, 6000, '2025-06-01'),
    ]
    const rows = buildContributionRows(txns, accounts, 2025, 30, 'self', limits)
    const k401 = rows.find((r) => r.label === '401k / Roth 401k')!
    expect(k401.total).toBe(18000)
    expect(k401.limit).toBe(23500)
    expect(k401.breakdown).toEqual([
      { type: '401k', label: '401k', total: 12000 },
      { type: 'roth_401k', label: 'Roth 401k', total: 6000 },
    ])
    expect(k401.pctUsed).toBeCloseTo(18000 / 23500)
  })

  it('omits account types with no contributions in either year', () => {
    const accounts = [acct(1, '401k')]
    const txns = [txn(10, 1, 1000, '2025-03-01')]
    const rows = buildContributionRows(txns, accounts, 2025, 30, 'self', limits)
    expect(rows.find((r) => r.label === 'HSA')).toBeUndefined()
    expect(rows).toHaveLength(1)
  })

  it('orders limited types before unlimited types', () => {
    const accounts = [acct(1, 'brokerage'), acct(2, '401k')]
    const txns = [
      txn(10, 1, 3000, '2025-03-01'),
      txn(11, 2, 1000, '2025-03-01'),
    ]
    const rows = buildContributionRows(txns, accounts, 2025, 30, 'self', limits)
    expect(rows[0].label).toBe('401k / Roth 401k')
    expect(rows[1].label).toBe('Brokerage')
    expect(rows[1].limit).toBeUndefined()
  })

  it('applies 401k catch-up when age >= 50', () => {
    const accounts = [acct(1, '401k')]
    const txns = [txn(10, 1, 1000, '2025-03-01')]
    const rows = buildContributionRows(txns, accounts, 2025, 55, 'self', limits)
    const k401 = rows.find((r) => r.label === '401k / Roth 401k')!
    expect(k401.limitBase).toBe(23500)
    expect(k401.catchUpAmount).toBe(7500)
    expect(k401.limit).toBe(31000)
  })

  it('does not apply catch-up when age < 50', () => {
    const accounts = [acct(1, '401k')]
    const txns = [txn(10, 1, 1000, '2025-03-01')]
    const rows = buildContributionRows(txns, accounts, 2025, 49, 'self', limits)
    const k401 = rows.find((r) => r.label === '401k / Roth 401k')!
    expect(k401.catchUpAmount).toBeUndefined()
    expect(k401.limit).toBe(23500)
  })

  it('applies HSA catch-up at age >= 55, not at 50', () => {
    const accounts = [acct(1, 'hsa')]
    const txns = [txn(10, 1, 100, '2025-03-01')]
    const at50 = buildContributionRows(txns, accounts, 2025, 50, 'self', limits)
      .find((r) => r.label === 'HSA')!
    expect(at50.catchUpAmount).toBeUndefined()
    const at55 = buildContributionRows(txns, accounts, 2025, 55, 'self', limits)
      .find((r) => r.label === 'HSA')!
    expect(at55.catchUpAmount).toBe(1000)
    expect(at55.limit).toBe(5300) // 4300 self + 1000
  })

  it('uses the family HSA limit when coverage is family', () => {
    const accounts = [acct(1, 'hsa')]
    const txns = [txn(10, 1, 100, '2025-03-01')]
    const rows = buildContributionRows(txns, accounts, 2025, 30, 'family', limits)
    const hsa = rows.find((r) => r.label === 'HSA')!
    expect(hsa.limit).toBe(8550)
  })

  it('computes yoyDelta as this year total minus prior year total', () => {
    const accounts = [acct(1, '401k')]
    const txns = [
      txn(10, 1, 5000, '2025-03-01'),
      txn(11, 1, 3000, '2024-03-01'),
    ]
    const rows = buildContributionRows(txns, accounts, 2025, 30, 'self', limits)
    const k401 = rows.find((r) => r.label === '401k / Roth 401k')!
    expect(k401.total).toBe(5000)
    expect(k401.yoyDelta).toBe(2000)
  })

  it('lets pctUsed exceed 1.0 on over-contribution (no clamping)', () => {
    const accounts = [acct(1, 'ira')]
    const txns = [txn(10, 1, 8000, '2025-03-01')]
    const rows = buildContributionRows(txns, accounts, 2025, 30, 'self', limits)
    const ira = rows.find((r) => r.label === 'Traditional / Roth IRA')!
    expect(ira.pctUsed).toBeGreaterThan(1)
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /Users/tomgobich/code/trackmyfi && npx vitest run src/lib/contributions/index.test.ts`
Expected: FAIL — cannot find `buildContributionRows`.

- [ ] **Step 3: Write the implementation**

Create `src/lib/contributions/index.ts`:

```ts
import type { Transaction } from '../types/Transaction'
import type { Account } from '../types/Account'
import type { YearLimits } from './irsLimits'

export interface ContributionBreakdown {
  type: string
  label: string
  total: number
}

export interface ContributionRow {
  label: string
  accountTypes: string[]
  total: number
  breakdown?: ContributionBreakdown[]
  limit?: number
  limitBase?: number
  catchUpAmount?: number
  pctUsed?: number
  yoyDelta?: number
}

type GroupKey = 'k401' | 'ira' | 'hsa' | 'brokerage' | 'crypto' | string

interface GroupDef {
  key: GroupKey
  label: string
  types: string[]
  order: number
  /** Resolve the effective limit + catch-up for this group, or undefined if unlimited. */
  limitFor?: (
    limits: YearLimits,
    age: number,
    hsaCoverage: 'self' | 'family',
  ) => { base: number; catchUp?: number }
}

const TYPE_LABELS: Record<string, string> = {
  '401k': '401k',
  roth_401k: 'Roth 401k',
  traditional_ira: 'Traditional IRA',
  roth_ira: 'Roth IRA',
  hsa: 'HSA',
  brokerage: 'Brokerage',
  crypto: 'Crypto',
}

// Group definitions in display order. Limited types first, then unlimited.
const GROUPS: GroupDef[] = [
  {
    key: 'k401',
    label: '401k / Roth 401k',
    types: ['401k', 'roth_401k'],
    order: 0,
    limitFor: (l, age) => ({
      base: l.k401,
      catchUp: age >= l.k401CatchUpAge ? l.k401CatchUp : undefined,
    }),
  },
  {
    key: 'ira',
    label: 'Traditional / Roth IRA',
    types: ['traditional_ira', 'roth_ira'],
    order: 1,
    limitFor: (l, age) => ({
      base: l.ira,
      catchUp: age >= l.iraCatchUpAge ? l.iraCatchUp : undefined,
    }),
  },
  {
    key: 'hsa',
    label: 'HSA',
    types: ['hsa'],
    order: 2,
    limitFor: (l, age, hsaCoverage) => ({
      base: hsaCoverage === 'family' ? l.hsaFamily : l.hsaSelf,
      catchUp: age >= l.hsaCatchUpAge ? l.hsaCatchUp : undefined,
    }),
  },
  { key: 'brokerage', label: 'Brokerage', types: ['brokerage'], order: 3 },
  { key: 'crypto', label: 'Crypto', types: ['crypto'], order: 4 },
]

function sumByType(
  txns: Transaction[],
  accounts: Account[],
  yearPrefix: string,
): Map<string, number> {
  const typeOf = new Map(accounts.map((a) => [a.id, a.type]))
  const out = new Map<string, number>()
  for (const t of txns) {
    if (!t.date.startsWith(yearPrefix)) continue
    const type = typeOf.get(t.accountId)
    if (!type) continue
    out.set(type, (out.get(type) ?? 0) + t.amount)
  }
  return out
}

export function buildContributionRows(
  txns: Transaction[],
  accounts: Account[],
  year: number,
  age: number,
  hsaCoverage: 'self' | 'family',
  limits: YearLimits,
): ContributionRow[] {
  const thisYear = sumByType(txns, accounts, String(year))
  const priorYear = sumByType(txns, accounts, String(year - 1))

  const rows: (ContributionRow & { _order: number })[] = []

  for (const group of GROUPS) {
    const total = group.types.reduce((s, t) => s + (thisYear.get(t) ?? 0), 0)
    const priorTotal = group.types.reduce((s, t) => s + (priorYear.get(t) ?? 0), 0)

    // Omit groups with no contributions in either year.
    if (total === 0 && priorTotal === 0) continue

    const row: ContributionRow & { _order: number } = {
      label: group.label,
      accountTypes: group.types,
      total,
      yoyDelta: total - priorTotal,
      _order: group.order,
    }

    // Breakdown only for merged (multi-type) groups.
    if (group.types.length > 1) {
      row.breakdown = group.types.map((t) => ({
        type: t,
        label: TYPE_LABELS[t] ?? t,
        total: thisYear.get(t) ?? 0,
      }))
    }

    if (group.limitFor) {
      const { base, catchUp } = group.limitFor(limits, age, hsaCoverage)
      row.limitBase = base
      row.catchUpAmount = catchUp
      row.limit = base + (catchUp ?? 0)
      row.pctUsed = row.limit > 0 ? total / row.limit : undefined
    }

    rows.push(row)
  }

  rows.sort((a, b) => a._order - b._order)
  return rows.map(({ _order, ...rest }) => rest)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /Users/tomgobich/code/trackmyfi && npx vitest run src/lib/contributions/index.test.ts`
Expected: PASS (all tests).

- [ ] **Step 5: Commit**

```bash
git add src/lib/contributions/index.ts src/lib/contributions/index.test.ts
git commit -m "feat: buildContributionRows aggregation with limits and YoY"
```

---

## Task 5: Rust contributions command

**Files:**
- Create: `src-tauri/src/commands/contributions.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs:42-46`
- Test: `src-tauri/tests/contributions.rs`

- [ ] **Step 1: Write the failing test**

Create `src-tauri/tests/contributions.rs`:

```rust
use libsql::Builder;
use trackmyfi_app_lib::commands::accounts::{self, NewAccount};
use trackmyfi_app_lib::commands::contributions;
use trackmyfi_app_lib::commands::transactions::{self, NewTransaction};
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
        created_at: "2025-01-01".into(),
    }).await.unwrap()
}

async fn add_contribution(conn: &libsql::Connection, account_id: i32, amount: f64, date: &str) {
    transactions::create_transaction(conn, &NewTransaction {
        account_id,
        transfer_account_id: None,
        amount,
        description: "Contribution".into(),
        date: date.into(),
        r#type: "income".into(),
        category: "savings".into(),
        is_contribution: true,
        import_source: "manual".into(),
        update_balance: false,
        created_at: format!("{date}T00:00:00Z"),
    }).await.unwrap();
}

async fn add_non_contribution(conn: &libsql::Connection, account_id: i32, amount: f64, date: &str) {
    transactions::create_transaction(conn, &NewTransaction {
        account_id,
        transfer_account_id: None,
        amount,
        description: "Paycheck".into(),
        date: date.into(),
        r#type: "income".into(),
        category: "uncategorized".into(),
        is_contribution: false,
        import_source: "manual".into(),
        update_balance: false,
        created_at: format!("{date}T00:00:00Z"),
    }).await.unwrap();
}

#[tokio::test]
async fn list_contribution_txns_returns_selected_and_prior_year() {
    let conn = setup().await;
    let acct = make_account(&conn, "401k", "401k").await;
    add_contribution(&conn, acct, 1000.0, "2026-03-01").await;
    add_contribution(&conn, acct, 800.0, "2025-03-01").await;
    add_contribution(&conn, acct, 500.0, "2024-03-01").await; // outside window

    let rows = contributions::list_contribution_txns(&conn, 2026).await.unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|t| t.date.starts_with("2026") || t.date.starts_with("2025")));
}

#[tokio::test]
async fn list_contribution_txns_excludes_non_contributions() {
    let conn = setup().await;
    let acct = make_account(&conn, "401k", "401k").await;
    add_contribution(&conn, acct, 1000.0, "2026-03-01").await;
    add_non_contribution(&conn, acct, 9999.0, "2026-03-02").await;

    let rows = contributions::list_contribution_txns(&conn, 2026).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].amount, 1000.0);
}

#[tokio::test]
async fn list_contribution_txns_empty_when_none() {
    let conn = setup().await;
    let rows = contributions::list_contribution_txns(&conn, 2026).await.unwrap();
    assert!(rows.is_empty());
}

#[tokio::test]
async fn list_contribution_years_returns_distinct_desc() {
    let conn = setup().await;
    let acct = make_account(&conn, "401k", "401k").await;
    add_contribution(&conn, acct, 1000.0, "2026-03-01").await;
    add_contribution(&conn, acct, 800.0, "2025-03-01").await;
    add_contribution(&conn, acct, 700.0, "2025-06-01").await; // same year, deduped
    add_non_contribution(&conn, acct, 9999.0, "2020-01-01").await; // excluded

    let years = contributions::list_contribution_years(&conn).await.unwrap();
    assert_eq!(years, vec!["2026".to_string(), "2025".to_string()]);
}
```

> Note: This test depends on `transactions::create_transaction` and `NewTransaction` being public with the fields shown. Confirm field names against `src-tauri/src/commands/transactions.rs` before running — they were read from the codebase (`account_id`, `transfer_account_id`, `amount`, `description`, `date`, `type`, `category`, `is_contribution`, `import_source`, `update_balance`, `created_at`).

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test --test contributions`
Expected: FAIL — `contributions` module / functions don't exist (compile error).

- [ ] **Step 3: Write the command implementation**

Create `src-tauri/src/commands/contributions.rs`:

```rust
use crate::db::Db;
use crate::models::Transaction;
use libsql::{params, Connection};
use tauri::State;

const COLS: &str = "id, account_id, transfer_account_id, amount, description, date, type, \
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

pub async fn list_contribution_txns(conn: &Connection, year: i32) -> Result<Vec<Transaction>, String> {
    let sql = format!(
        "SELECT {COLS} FROM txn WHERE is_contribution = 1 \
         AND strftime('%Y', date) IN (?1, ?2) ORDER BY date DESC, id DESC"
    );
    let mut rows = conn
        .query(&sql, params![year.to_string(), (year - 1).to_string()])
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(row_to_txn(&row)?);
    }
    Ok(out)
}

pub async fn list_contribution_years(conn: &Connection) -> Result<Vec<String>, String> {
    let mut rows = conn
        .query(
            "SELECT DISTINCT strftime('%Y', date) AS year FROM txn \
             WHERE is_contribution = 1 ORDER BY year DESC",
            (),
        )
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(row.get::<String>(0).map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub async fn list_contribution_txns_cmd(db: State<'_, Db>, year: i32) -> Result<Vec<Transaction>, String> {
    let conn = db.conn().await?;
    list_contribution_txns(&conn, year).await
}

#[tauri::command]
pub async fn list_contribution_years_cmd(db: State<'_, Db>) -> Result<Vec<String>, String> {
    let conn = db.conn().await?;
    list_contribution_years(&conn).await
}
```

- [ ] **Step 4: Register the module**

In `src-tauri/src/commands/mod.rs`, add (keep alphabetical order — after `accounts;`):

```rust
pub mod contributions;
```

- [ ] **Step 5: Register the two commands**

In `src-tauri/src/lib.rs`, add to the `generate_handler!` macro list (after the paychecks commands, before the closing `])`):

```rust
            commands::contributions::list_contribution_txns_cmd,
            commands::contributions::list_contribution_years_cmd,
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cd src-tauri && cargo test --test contributions`
Expected: PASS (4 tests).

- [ ] **Step 7: Run full Rust suite for regressions**

Run: `cd src-tauri && cargo test`
Expected: PASS (all tests, including the prior roundtrip/migrations).

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/commands/contributions.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs src-tauri/tests/contributions.rs
git commit -m "feat: Rust list_contribution_txns + list_contribution_years commands"
```

---

## Task 6: API bindings + Pinia store

**Files:**
- Create: `src/lib/api/contributions.ts`
- Create: `src/stores/contributions.ts`

No unit test (thin invoke wrappers + store, matching untested `api/*` and `stores/*` siblings). Verified by typecheck and the page that consumes them.

- [ ] **Step 1: Write the API bindings**

Create `src/lib/api/contributions.ts`:

```ts
import { invoke } from '@tauri-apps/api/core'
import type { Transaction } from '../types/Transaction'

export const listContributionTxns = (year: number) =>
  invoke<Transaction[]>('list_contribution_txns_cmd', { year })

export const listContributionYears = () =>
  invoke<string[]>('list_contribution_years_cmd')
```

- [ ] **Step 2: Write the store**

Create `src/stores/contributions.ts`:

```ts
import { defineStore } from 'pinia'
import { ref } from 'vue'
import { DateTime } from 'luxon'
import type { Transaction } from '../lib/types/Transaction'
import * as api from '../lib/api/contributions'

export const useContributionsStore = defineStore('contributions', () => {
  const txns = ref<Transaction[]>([])
  const years = ref<number[]>([])
  const selectedYear = ref<number>(DateTime.now().year)

  async function loadYears() {
    const raw = await api.listContributionYears()
    years.value = raw.map(Number)
    // Ensure the current year is always selectable even with no data yet.
    const current = DateTime.now().year
    if (!years.value.includes(current)) {
      years.value = [current, ...years.value].sort((a, b) => b - a)
    }
  }

  async function load(year: number) {
    selectedYear.value = year
    txns.value = await api.listContributionTxns(year)
  }

  return { txns, years, selectedYear, loadYears, load }
})
```

- [ ] **Step 3: Typecheck**

Run: `cd /Users/tomgobich/code/trackmyfi && npx vue-tsc --noEmit`
Expected: no errors in the new files.

- [ ] **Step 4: Commit**

```bash
git add src/lib/api/contributions.ts src/stores/contributions.ts
git commit -m "feat: contributions API bindings and Pinia store"
```

---

## Task 7: Contributions page

**Files:**
- Create: `src/pages/Contributions.vue`

Presentational Vue component (untested, matching `Transactions.vue` / `Paychecks.vue`). Verified by typecheck + manual smoke.

- [ ] **Step 1: Write the page**

Create `src/pages/Contributions.vue`:

```vue
<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { DateTime } from 'luxon'
import { useContributionsStore } from '../stores/contributions'
import { useAccountsStore } from '../stores/accounts'
import { useFireProfileStore } from '../stores/fireProfile'
import { buildContributionRows, type ContributionRow } from '../lib/contributions/index'
import { resolveYearLimits } from '../lib/contributions/irsLimits'

const store = useContributionsStore()
const accountsStore = useAccountsStore()
const fp = useFireProfileStore()

const selectedYear = ref<number>(DateTime.now().year)

const resolved = computed(() => resolveYearLimits(selectedYear.value))

const rows = computed<ContributionRow[]>(() => {
  if (!fp.profile) return []
  return buildContributionRows(
    store.txns,
    accountsStore.accounts,
    selectedYear.value,
    fp.profile.currentAge,
    (fp.profile.hsaCoverage as 'self' | 'family') ?? 'self',
    resolved.value.limits,
  )
})

const ytdTotal = computed(() => rows.value.reduce((s, r) => s + r.total, 0))

function accountName(id: number): string {
  return accountsStore.accounts.find((a) => a.id === id)?.name ?? `#${id}`
}

function accountType(id: number): string {
  return accountsStore.accounts.find((a) => a.id === id)?.type ?? ''
}

// Transactions belonging to a given row's account types, for the selected year.
function rowTxns(row: ContributionRow) {
  return store.txns
    .filter(
      (t) =>
        t.date.startsWith(String(selectedYear.value)) &&
        row.accountTypes.includes(accountType(t.accountId)),
    )
}

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })
}

function pct(n: number | undefined): string {
  return n === undefined ? '—' : `${(n * 100).toFixed(0)}%`
}

function barColor(pctUsed: number | undefined): string {
  if (pctUsed === undefined) return 'bg-gray-400'
  if (pctUsed > 1) return 'bg-amber-500'
  if (pctUsed >= 1) return 'bg-green-500'
  if (pctUsed >= 0.8) return 'bg-amber-500'
  return 'bg-green-500'
}

function barWidth(pctUsed: number | undefined): string {
  if (pctUsed === undefined) return '0%'
  return `${Math.min(pctUsed * 100, 100)}%`
}

function limitLabel(row: ContributionRow): string {
  if (row.limit === undefined) return 'No IRS limit'
  const base = money(row.limit)
  if (row.catchUpAmount) return `${base} (incl. ${money(row.catchUpAmount)} catch-up)`
  return base
}

function importLabel(source: string): string {
  return source === 'paycheck' ? 'via Paycheck' : 'Manual'
}

async function onYearChange(year: number) {
  selectedYear.value = year
  await store.load(year)
}

onMounted(async () => {
  await Promise.all([accountsStore.load(), fp.load(), store.loadYears()])
  selectedYear.value = DateTime.now().year
  await store.load(selectedYear.value)
})
</script>

<template>
  <div class="p-6 space-y-6">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-semibold">Contributions</h1>
      <USelect
        :model-value="selectedYear"
        :items="store.years.map((y) => ({ label: String(y), value: y }))"
        class="w-28"
        @update:model-value="onYearChange"
      />
    </div>

    <UAlert
      v-if="resolved.estimated"
      color="warning"
      variant="soft"
      icon="i-lucide-info"
      :title="`IRS limits estimated from ${resolved.estimatedFrom}`"
      :description="`Update irsLimits.ts when ${selectedYear} limits are announced.`"
    />

    <div class="flex gap-6 text-sm">
      <span>YTD Total: <strong>{{ money(ytdTotal) }}</strong></span>
      <span class="text-muted">{{ rows.length }} account types</span>
    </div>

    <!-- Card grid -->
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
      <div
        v-for="row in rows"
        :key="row.label"
        class="border border-default rounded-lg p-4 space-y-2"
      >
        <div class="text-sm font-medium">{{ row.label }}</div>
        <div class="text-xl font-bold tabular-nums">{{ money(row.total) }}</div>
        <template v-if="row.limit !== undefined">
          <div class="bg-elevated rounded-full h-1.5 overflow-hidden">
            <div
              class="h-full rounded-full"
              :class="barColor(row.pctUsed)"
              :style="{ width: barWidth(row.pctUsed) }"
            />
          </div>
          <div class="text-xs text-muted">{{ pct(row.pctUsed) }} of {{ limitLabel(row) }}</div>
        </template>
        <div v-else class="text-xs text-muted">No IRS limit</div>
        <div
          v-if="row.yoyDelta !== undefined"
          class="text-xs"
          :class="row.yoyDelta > 0 ? 'text-green-600' : row.yoyDelta < 0 ? 'text-red-600' : 'text-muted'"
        >
          {{ row.yoyDelta > 0 ? '+' : '' }}{{ money(row.yoyDelta) }} vs {{ selectedYear - 1 }}
        </div>
        <div
          v-if="row.breakdown"
          class="text-xs text-muted pt-2 border-t border-default/50"
        >
          {{ row.breakdown.map((b) => `${b.label}: ${money(b.total)}`).join(' · ') }}
        </div>
      </div>
    </div>

    <p v-if="!rows.length" class="text-muted text-sm">
      No contributions recorded for {{ selectedYear }}.
    </p>

    <!-- Grouped transaction table -->
    <div v-for="row in rows" :key="`group-${row.label}`" class="border border-default rounded-lg overflow-hidden">
      <div class="bg-elevated px-4 py-2 flex justify-between items-center">
        <span class="font-medium text-sm">{{ row.label }}</span>
        <div class="flex gap-4 text-xs text-muted">
          <span>YTD: <strong class="text-default">{{ money(row.total) }}</strong></span>
          <span v-if="row.limit !== undefined">Limit: <strong class="text-default">{{ money(row.limit) }}</strong></span>
          <span v-if="row.pctUsed !== undefined">{{ pct(row.pctUsed) }}</span>
        </div>
      </div>
      <table class="w-full text-sm">
        <tbody>
          <tr
            v-for="t in rowTxns(row)"
            :key="t.id"
            class="border-t border-default/50"
          >
            <td class="px-4 py-2 text-muted w-28">{{ t.date }}</td>
            <td class="py-2">{{ t.description }}</td>
            <td class="py-2 text-muted">{{ accountName(t.accountId) }}</td>
            <td class="py-2 text-muted text-xs">{{ importLabel(t.importSource) }}</td>
            <td class="px-4 py-2 text-right tabular-nums">{{ money(t.amount) }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
```

- [ ] **Step 2: Typecheck**

Run: `cd /Users/tomgobich/code/trackmyfi && npx vue-tsc --noEmit`
Expected: no errors in `Contributions.vue`.

- [ ] **Step 3: Commit**

```bash
git add src/pages/Contributions.vue
git commit -m "feat: Contributions page — card grid + grouped table"
```

---

## Task 8: Router + nav wiring

**Files:**
- Modify: `src/router.ts:7-8`
- Modify: `src/App.vue:7`

- [ ] **Step 1: Add the route**

In `src/router.ts`, add after the `paychecks` route (line 7):

```ts
  { path: '/contributions', name: 'contributions', component: () => import('./pages/Contributions.vue') },
```

- [ ] **Step 2: Enable the nav link**

In `src/App.vue`, replace the disabled Contributions link:

```ts
  { label: 'Contributions', icon: 'i-lucide-piggy-bank', disabled: true },
```

with:

```ts
  { label: 'Contributions', to: '/contributions', icon: 'i-lucide-piggy-bank' },
```

- [ ] **Step 3: Typecheck + full test suite**

Run: `cd /Users/tomgobich/code/trackmyfi && npx vue-tsc --noEmit && npm test`
Expected: no type errors; all Vitest suites pass.

- [ ] **Step 4: Commit**

```bash
git add src/router.ts src/App.vue
git commit -m "feat: wire Contributions route and nav link"
```

---

## Task 9: Manual GUI smoke test

Cannot run headless (same caveat as Phase 1/2a/2b). Run `npm run tauri dev` and verify:

- [ ] Contributions nav link is enabled and routes to the page
- [ ] Year picker defaults to the current year and lists years with contribution data
- [ ] Cards show one per account type with contributions; 401k/Roth merged into one card with a Trad/Roth breakdown line
- [ ] Progress bar color: green < 80%, amber 80–99%, green at 100%, amber > 100%
- [ ] A card for someone age ≥ 50 shows "(incl. $X catch-up)"; the limit increases accordingly
- [ ] Brokerage/crypto cards show "No IRS limit" and no progress bar
- [ ] Grouped table below shows the contribution txns per type with date, description, account, source, amount
- [ ] Settings → HSA coverage toggle persists; switching to Family raises the HSA card's limit after reload
- [ ] Changing the year reloads data and recomputes YoY against the new prior year

---

## Self-Review Notes

- **Spec coverage:** year picker (Task 6/7), prior+selected year query (Task 5), card grid + progress bar colors incl. >100% (Task 7), grouped table (Task 7), IRS limits + fallback banner (Task 3/7), catch-up labeled (Task 4/7), HSA self/family toggle (Task 1/2), brokerage/crypto shown without limit (Task 4/7), `list_contribution_years` for the picker (Task 5/6). All spec sections map to a task.
- **Type consistency:** `ContributionRow`, `YearLimits`, `resolveYearLimits` signatures are consistent across Tasks 3–7. `buildContributionRows` arg order `(txns, accounts, year, age, hsaCoverage, limits)` matches all call sites (test + page).
- **Known dependency to confirm at execution time:** Task 5's Rust test assumes `transactions::create_transaction` / `NewTransaction` are `pub` with the listed fields. Confirmed against `src-tauri/src/commands/transactions.rs` during planning, but re-verify field names if the test fails to compile.
```
