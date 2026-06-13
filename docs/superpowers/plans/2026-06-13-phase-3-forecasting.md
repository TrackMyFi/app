# Phase 3 — Forecasting Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add FIRE forecasting (Coast FIRE, Lean/Fat variants, required-contribution back-solve, live what-if planner) on a new Forecast page, and reconcile the Dashboard to use a real contribution-derived monthly figure.

**Architecture:** All new math is pure, unit-tested TypeScript in `src/lib/fire/`. No new DB tables, no migrations, no new Rust commands — the derived contribution baseline reuses the existing `list_contribution_txns_cmd`. The Forecast page composes the math with a unovis chart, three variant cards, and a right-side what-if drawer holding ephemeral override sliders.

**Tech Stack:** Vue 3 + NuxtUI 4 (`USlideover`, `USlider`, `UCard`, `UButton`), unovis (`@unovis/vue`), Luxon, Pinia, Vitest. Node via fnm — prefix commands with the fnm node path if `node`/`npm` is not found: `export PATH="$HOME/Library/Application Support/fnm/node-versions/v24.12.0/installation/bin:$PATH"`.

---

## File Structure

| File | Responsibility |
|---|---|
| `src/lib/fire/contributionRate.ts` (create) | Trailing-12-mo average of real contributions + fallback resolver |
| `src/lib/fire/coast.ts` (create) | Coast FIRE number, coast status, coast-crossing date |
| `src/lib/fire/requiredContribution.ts` (create) | Back-solved monthly contribution to hit FIRE number by retirement |
| `src/lib/fire/projection.ts` (modify) | Add `projectionSeries` for the growth-curve chart |
| `src/lib/fire/forecast.ts` (create) | `buildForecast` aggregator → Lean/Regular/Fat × 6-metric objects |
| `src/lib/fire/index.ts` (modify) | Re-export new modules |
| `src/components/ForecastChart.vue` (create) | Growth curve + FIRE-number + Coast reference lines |
| `src/pages/Forecast.vue` (create) | Page: chart + variant cards + what-if drawer |
| `src/router.ts` (modify) | Add `/forecast` route |
| `src/App.vue` (modify) | Enable the (currently disabled) Forecast nav link |
| `src/pages/Dashboard.vue` (modify) | Use derived contribution; drop "Approximate" hint |

---

## Task 1: Derived monthly contribution (`contributionRate.ts`)

**Files:**
- Create: `src/lib/fire/contributionRate.ts`
- Test: `src/lib/fire/contributionRate.test.ts`

The baseline monthly contribution for forecasts. Pure functions over `Transaction[]`. Caller supplies the fallback estimate (so this module stays free of account/balance deps).

- [ ] **Step 1: Write the failing test**

Create `src/lib/fire/contributionRate.test.ts`:

```ts
import { describe, it, expect } from 'vitest'
import {
  trailingMonthlyContribution,
  hasFullYearOfContributions,
  derivedMonthlyContribution,
} from './contributionRate'
import type { Transaction } from '../types/Transaction'

// Minimal Transaction factory — only fields the functions read matter.
function txn(p: Partial<Transaction>): Transaction {
  return {
    id: 1, accountId: 1, transferAccountId: null, amount: 0, description: '',
    date: '2026-01-01', type: 'expense', category: '', isContribution: false,
    importSource: 'manual', generatedBalanceId: null, generatedBalanceToId: null,
    paycheckId: null, createdAt: '', updatedAt: '', ...p,
  }
}

const asOf = '2026-06-30'

describe('trailingMonthlyContribution', () => {
  it('sums contributions in the trailing 12 months and divides by 12', () => {
    const txns = [
      txn({ amount: 600, date: '2026-06-01', isContribution: true }),
      txn({ amount: 600, date: '2026-01-15', isContribution: true }),
      txn({ amount: 999, date: '2026-03-01', isContribution: false }), // not a contribution
    ]
    expect(trailingMonthlyContribution(txns, asOf)).toBe(100) // 1200 / 12
  })

  it('excludes contributions older than 12 months and any in the future', () => {
    const txns = [
      txn({ amount: 1200, date: '2025-06-01', isContribution: true }), // exactly 12mo+1day before → out
      txn({ amount: 1200, date: '2026-12-01', isContribution: true }), // future → out
    ]
    expect(trailingMonthlyContribution(txns, asOf)).toBe(0)
  })
})

describe('hasFullYearOfContributions', () => {
  it('is true when a contribution exists at or before the 12-month cutoff', () => {
    const txns = [txn({ amount: 100, date: '2025-06-30', isContribution: true })]
    expect(hasFullYearOfContributions(txns, asOf)).toBe(true)
  })

  it('is false when all contributions are newer than the cutoff', () => {
    const txns = [txn({ amount: 100, date: '2026-02-01', isContribution: true })]
    expect(hasFullYearOfContributions(txns, asOf)).toBe(false)
  })

  it('is false with no contributions', () => {
    expect(hasFullYearOfContributions([], asOf)).toBe(false)
  })
})

describe('derivedMonthlyContribution', () => {
  it('uses actual trailing average when a full year of history exists', () => {
    const txns = [
      txn({ amount: 1200, date: '2025-06-30', isContribution: true }),
      txn({ amount: 1200, date: '2026-05-01', isContribution: true }),
    ]
    const r = derivedMonthlyContribution(txns, asOf, 500)
    expect(r.estimated).toBe(false)
    expect(r.monthly).toBe(200) // 2400 / 12
  })

  it('falls back to the supplied estimate when <12 months of history', () => {
    const txns = [txn({ amount: 1200, date: '2026-05-01', isContribution: true })]
    const r = derivedMonthlyContribution(txns, asOf, 500)
    expect(r.estimated).toBe(true)
    expect(r.monthly).toBe(500)
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test -- contributionRate`
Expected: FAIL — cannot find module `./contributionRate`.

- [ ] **Step 3: Write minimal implementation**

Create `src/lib/fire/contributionRate.ts`:

```ts
import { DateTime } from 'luxon'
import type { Transaction } from '../types/Transaction'

function cutoffIso(asOfIso: string): string {
  return DateTime.fromISO(asOfIso).minus({ months: 12 }).toISODate()!
}

/**
 * Average monthly contribution over the trailing 12 months ending at `asOfIso`.
 * Sums `isContribution` txns with `cutoff < date <= asOf`, divided by 12.
 */
export function trailingMonthlyContribution(txns: Transaction[], asOfIso: string): number {
  const cutoff = cutoffIso(asOfIso)
  let total = 0
  for (const t of txns) {
    if (!t.isContribution) continue
    if (t.date > cutoff && t.date <= asOfIso) total += t.amount
  }
  return total / 12
}

/**
 * True when at least one contribution exists at or before the 12-month cutoff,
 * i.e. the trailing-12-month window captures a full year of contribution history.
 */
export function hasFullYearOfContributions(txns: Transaction[], asOfIso: string): boolean {
  const cutoff = cutoffIso(asOfIso)
  return txns.some(t => t.isContribution && t.date <= cutoff)
}

export interface DerivedContribution {
  monthly: number
  estimated: boolean
}

/**
 * Derived monthly contribution baseline. Uses the trailing-12-month actual
 * average when a full year of contribution history exists; otherwise returns the
 * caller-supplied `estimateMonthly` (the Phase 1 savings-rate approximation) and
 * flags `estimated: true`.
 */
export function derivedMonthlyContribution(
  txns: Transaction[], asOfIso: string, estimateMonthly: number,
): DerivedContribution {
  if (hasFullYearOfContributions(txns, asOfIso)) {
    return { monthly: trailingMonthlyContribution(txns, asOfIso), estimated: false }
  }
  return { monthly: estimateMonthly, estimated: true }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm run test -- contributionRate`
Expected: PASS (all cases).

- [ ] **Step 5: Commit**

```bash
git add src/lib/fire/contributionRate.ts src/lib/fire/contributionRate.test.ts
git commit -m "feat: derived monthly contribution from trailing-12mo txns"
```

---

## Task 2: Coast FIRE math (`coast.ts`)

**Files:**
- Create: `src/lib/fire/coast.ts`
- Test: `src/lib/fire/coast.test.ts`

- [ ] **Step 1: Write the failing test**

Create `src/lib/fire/coast.test.ts`:

```ts
import { describe, it, expect } from 'vitest'
import { DateTime } from 'luxon'
import { realAnnualReturn, coastFireNumber, coastStatus } from './coast'

const from = DateTime.fromISO('2026-01-01')

describe('realAnnualReturn', () => {
  it('with zero inflation equals the nominal return', () => {
    expect(realAnnualReturn(0.07, 0)).toBeCloseTo(0.07, 6)
  })
  it('is reduced by inflation', () => {
    expect(realAnnualReturn(0.07, 0.03)).toBeCloseTo((1.07 / 1.03) - 1, 6)
  })
})

describe('coastFireNumber', () => {
  it('discounts the FIRE number back over years to retirement at the real return', () => {
    // 7% real, 10 years: 1_000_000 / 1.07^10
    const expected = 1_000_000 / Math.pow(1.07, 10)
    expect(coastFireNumber(1_000_000, 40, 50, 0.07, 0)).toBeCloseTo(expected, 2)
  })
  it('returns the FIRE number when already at/past retirement age', () => {
    expect(coastFireNumber(1_000_000, 50, 50, 0.07, 0)).toBe(1_000_000)
    expect(coastFireNumber(1_000_000, 55, 50, 0.07, 0)).toBe(1_000_000)
  })
})

describe('coastStatus', () => {
  it('reports coasting when investable already meets the coast number', () => {
    const coastNum = coastFireNumber(1_000_000, 40, 50, 0.07, 0)
    const r = coastStatus(coastNum + 1, 0, 1_000_000, 40, 50, 0.07, 0, from)
    expect(r.coasting).toBe(true)
    expect(r.crossingDate).toBeNull()
    expect(r.coastNumber).toBeCloseTo(coastNum, 2)
  })

  it('projects a future crossing date when not yet coasting', () => {
    const r = coastStatus(100_000, 2_000, 1_000_000, 40, 50, 0.07, 0, from)
    expect(r.coasting).toBe(false)
    expect(r.crossingDate).not.toBeNull()
    expect(r.crossingDate!.toMillis()).toBeGreaterThan(from.toMillis())
  })

  it('returns a null crossing date when the coast number is unreachable', () => {
    const r = coastStatus(0, 0, 1_000_000, 40, 50, 0.07, 0, from)
    expect(r.coasting).toBe(false)
    expect(r.crossingDate).toBeNull()
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test -- coast`
Expected: FAIL — cannot find module `./coast`.

- [ ] **Step 3: Write minimal implementation**

Create `src/lib/fire/coast.ts`:

```ts
import { DateTime } from 'luxon'
import { realMonthlyReturn, monthsToFire } from './projection'

/** Annual real return implied by the monthly real return basis used in projections. */
export function realAnnualReturn(expectedReturnRate: number, inflationRate: number): number {
  return Math.pow(1 + realMonthlyReturn(expectedReturnRate, inflationRate), 12) - 1
}

/**
 * Investable net worth needed today so it compounds to `fireNumber` by retirement
 * with zero further contributions. When already at/past retirement age, the coast
 * number is the full FIRE number (no time left to grow).
 */
export function coastFireNumber(
  fireNumber: number, currentAge: number, targetRetirementAge: number,
  expectedReturnRate: number, inflationRate: number,
): number {
  const years = targetRetirementAge - currentAge
  if (years <= 0) return fireNumber
  const r = realAnnualReturn(expectedReturnRate, inflationRate)
  return fireNumber / Math.pow(1 + r, years)
}

export interface CoastStatus {
  coasting: boolean
  coastNumber: number
  /** Date the current trajectory crosses the coast number; null if coasting or unreachable. */
  crossingDate: DateTime | null
}

export function coastStatus(
  investable: number, monthlyContribution: number, fireNumber: number,
  currentAge: number, targetRetirementAge: number,
  expectedReturnRate: number, inflationRate: number,
  from: DateTime = DateTime.now(),
): CoastStatus {
  const coastNumber = coastFireNumber(fireNumber, currentAge, targetRetirementAge, expectedReturnRate, inflationRate)
  if (investable >= coastNumber) return { coasting: true, coastNumber, crossingDate: null }
  const months = monthsToFire(
    investable, monthlyContribution,
    realMonthlyReturn(expectedReturnRate, inflationRate), coastNumber,
  )
  return { coasting: false, coastNumber, crossingDate: months === null ? null : from.plus({ months }) }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm run test -- coast`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/fire/coast.ts src/lib/fire/coast.test.ts
git commit -m "feat: Coast FIRE number and coast-crossing status"
```

---

## Task 3: Required contribution back-solve (`requiredContribution.ts`)

**Files:**
- Create: `src/lib/fire/requiredContribution.ts`
- Test: `src/lib/fire/requiredContribution.test.ts`

- [ ] **Step 1: Write the failing test**

Create `src/lib/fire/requiredContribution.test.ts`:

```ts
import { describe, it, expect } from 'vitest'
import { requiredMonthlyContribution } from './requiredContribution'

describe('requiredMonthlyContribution', () => {
  it('back-solves the annuity payment that reaches the target (verifies via forward FV)', () => {
    const pv = 100_000, target = 1_000_000, n = 240 // 20 years
    const pmt = requiredMonthlyContribution(pv, target, 0.07, 0, n)!
    // Forward-simulate FV with this pmt; should land on target.
    const r = Math.pow(1.07, 1 / 12) - 1
    let fv = pv
    for (let m = 0; m < n; m++) fv = fv * (1 + r) + pmt
    expect(fv).toBeCloseTo(target, 0)
  })

  it('returns 0 when the present value alone already reaches the target', () => {
    expect(requiredMonthlyContribution(2_000_000, 1_000_000, 0.07, 0, 240)).toBe(0)
  })

  it('handles ~zero real return with the linear formula', () => {
    // real return 0 → pmt = (target - pv) / n
    expect(requiredMonthlyContribution(0, 1200, 0.03, 0.03, 12)).toBeCloseTo(100, 6)
  })

  it('returns null when there is no time left (months <= 0)', () => {
    expect(requiredMonthlyContribution(0, 1_000_000, 0.07, 0, 0)).toBeNull()
    expect(requiredMonthlyContribution(0, 1_000_000, 0.07, 0, -12)).toBeNull()
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test -- requiredContribution`
Expected: FAIL — cannot find module.

- [ ] **Step 3: Write minimal implementation**

Create `src/lib/fire/requiredContribution.ts`:

```ts
import { realMonthlyReturn } from './projection'

/**
 * Monthly contribution required so `presentValue` grows to `target` in exactly
 * `monthsToRetirement` months, using the monthly real return.
 *
 * Closed-form annuity payment: FV = PV·(1+r)^n + PMT·((1+r)^n − 1)/r
 *   → PMT = (target − PV·(1+r)^n) · r / ((1+r)^n − 1)
 *
 * Returns 0 when the present value alone already reaches the target.
 * Returns null when `monthsToRetirement <= 0` (no time left).
 */
export function requiredMonthlyContribution(
  presentValue: number, target: number,
  expectedReturnRate: number, inflationRate: number,
  monthsToRetirement: number,
): number | null {
  if (monthsToRetirement <= 0) return null
  const r = realMonthlyReturn(expectedReturnRate, inflationRate)
  const n = monthsToRetirement
  if (Math.abs(r) < 1e-9) {
    const pmt = (target - presentValue) / n
    return pmt <= 0 ? 0 : pmt
  }
  const growth = Math.pow(1 + r, n)
  const fvPresent = presentValue * growth
  if (fvPresent >= target) return 0
  const pmt = ((target - fvPresent) * r) / (growth - 1)
  return pmt <= 0 ? 0 : pmt
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm run test -- requiredContribution`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/fire/requiredContribution.ts src/lib/fire/requiredContribution.test.ts
git commit -m "feat: required monthly contribution back-solve"
```

---

## Task 4: Projection series for the chart (`projection.ts`)

**Files:**
- Modify: `src/lib/fire/projection.ts`
- Test: `src/lib/fire/projection.test.ts` (append)

- [ ] **Step 1: Write the failing test**

Append to `src/lib/fire/projection.test.ts` (add imports `projectionSeries` and, if not present, `DateTime` from `luxon`):

```ts
import { projectionSeries } from './projection'
import { DateTime } from 'luxon'

describe('projectionSeries', () => {
  const from = DateTime.fromISO('2026-01-01')

  it('emits months+1 points starting at the present value', () => {
    const pts = projectionSeries(1000, 100, 0.07, 0, 3, from)
    expect(pts).toHaveLength(4)
    expect(pts[0]).toEqual({ date: '2026-01-01', value: 1000 })
  })

  it('compounds each month then adds the contribution', () => {
    const r = Math.pow(1.07, 1 / 12) - 1
    const pts = projectionSeries(1000, 100, 0.07, 0, 1, from)
    expect(pts[1].value).toBeCloseTo(1000 * (1 + r) + 100, 6)
    expect(pts[1].date).toBe('2026-02-01')
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test -- projection`
Expected: FAIL — `projectionSeries` is not exported.

- [ ] **Step 3: Write minimal implementation**

Add to `src/lib/fire/projection.ts` (the file already imports `DateTime` from `luxon` and defines `realMonthlyReturn`):

```ts
export interface ProjectionPoint { date: string; value: number }

/**
 * Month-by-month projected investable value, `months + 1` points (index 0 = the
 * present value at `from`). Each subsequent month compounds at the monthly real
 * return then adds the monthly contribution.
 */
export function projectionSeries(
  presentValue: number, monthlyContribution: number,
  expectedReturnRate: number, inflationRate: number,
  months: number, from: DateTime = DateTime.now(),
): ProjectionPoint[] {
  const mr = realMonthlyReturn(expectedReturnRate, inflationRate)
  const pts: ProjectionPoint[] = []
  let fv = presentValue
  for (let m = 0; m <= months; m++) {
    if (m > 0) fv = fv * (1 + mr) + monthlyContribution
    pts.push({ date: from.plus({ months: m }).toISODate()!, value: fv })
  }
  return pts
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm run test -- projection`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/fire/projection.ts src/lib/fire/projection.test.ts
git commit -m "feat: projectionSeries for forecast growth chart"
```

---

## Task 5: Forecast aggregator (`forecast.ts`)

**Files:**
- Create: `src/lib/fire/forecast.ts`
- Test: `src/lib/fire/forecast.test.ts`

- [ ] **Step 1: Write the failing test**

Create `src/lib/fire/forecast.test.ts`:

```ts
import { describe, it, expect } from 'vitest'
import { DateTime } from 'luxon'
import { buildForecast, variantExpenses, type ForecastInputs } from './forecast'

const from = DateTime.fromISO('2026-01-01')

const base: ForecastInputs = {
  currentAge: 40,
  targetRetirementAge: 55,
  annualExpensesTarget: 60_000,
  leanFireAnnualExpenses: 40_000,
  fatFireAnnualExpenses: 100_000,
  expectedReturnRate: 0.07,
  inflationRate: 0.03,
  investable: 300_000,
  monthlyContribution: 2_000,
}

describe('variantExpenses', () => {
  it('picks lean/regular/fat expenses', () => {
    expect(variantExpenses(base, 'lean')).toBe(40_000)
    expect(variantExpenses(base, 'regular')).toBe(60_000)
    expect(variantExpenses(base, 'fat')).toBe(100_000)
  })
  it('falls back to annualExpensesTarget when a variant field is null', () => {
    const noLean = { ...base, leanFireAnnualExpenses: null, fatFireAnnualExpenses: null }
    expect(variantExpenses(noLean, 'lean')).toBe(60_000)
    expect(variantExpenses(noLean, 'fat')).toBe(60_000)
  })
})

describe('buildForecast', () => {
  it('produces three variants with the expected FIRE numbers', () => {
    const f = buildForecast(base, from)
    expect(f.map(v => v.variant)).toEqual(['lean', 'regular', 'fat'])
    expect(f[0].fireNumber).toBe(40_000 * 25)
    expect(f[1].fireNumber).toBe(60_000 * 25)
    expect(f[2].fireNumber).toBe(100_000 * 25)
  })

  it('lean reaches FI no later than fat', () => {
    const f = buildForecast(base, from)
    const lean = f[0].fiDate, fat = f[2].fiDate
    expect(lean).not.toBeNull()
    if (lean && fat) expect(lean.toMillis()).toBeLessThanOrEqual(fat.toMillis())
  })

  it('fills coast and required-contribution fields for each variant', () => {
    const f = buildForecast(base, from)
    for (const v of f) {
      expect(typeof v.coastNumber).toBe('number')
      expect(typeof v.coasting).toBe('boolean')
      expect(v.requiredMonthly === null || typeof v.requiredMonthly === 'number').toBe(true)
    }
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test -- forecast`
Expected: FAIL — cannot find module `./forecast`.

- [ ] **Step 3: Write minimal implementation**

Create `src/lib/fire/forecast.ts`:

```ts
import { DateTime } from 'luxon'
import { fireNumber } from './metrics'
import { monthsToFire, projectedFiDate, realMonthlyReturn } from './projection'
import { coastStatus } from './coast'
import { requiredMonthlyContribution } from './requiredContribution'

export type FireVariant = 'lean' | 'regular' | 'fat'

export interface ForecastInputs {
  currentAge: number
  targetRetirementAge: number
  annualExpensesTarget: number
  leanFireAnnualExpenses: number | null
  fatFireAnnualExpenses: number | null
  expectedReturnRate: number
  inflationRate: number
  investable: number
  monthlyContribution: number
}

export interface VariantForecast {
  variant: FireVariant
  expenses: number
  fireNumber: number
  fiDate: DateTime | null
  yearsToFi: number | null
  coastNumber: number
  coasting: boolean
  coastCrossingDate: DateTime | null
  requiredMonthly: number | null
}

export function variantExpenses(inputs: ForecastInputs, variant: FireVariant): number {
  if (variant === 'lean') return inputs.leanFireAnnualExpenses ?? inputs.annualExpensesTarget
  if (variant === 'fat') return inputs.fatFireAnnualExpenses ?? inputs.annualExpensesTarget
  return inputs.annualExpensesTarget
}

export function buildVariantForecast(
  inputs: ForecastInputs, variant: FireVariant, from: DateTime = DateTime.now(),
): VariantForecast {
  const expenses = variantExpenses(inputs, variant)
  const fireNum = fireNumber(expenses)
  const mr = realMonthlyReturn(inputs.expectedReturnRate, inputs.inflationRate)
  const months = monthsToFire(inputs.investable, inputs.monthlyContribution, mr, fireNum)
  const fiDate = projectedFiDate(
    inputs.investable, inputs.monthlyContribution,
    inputs.expectedReturnRate, inputs.inflationRate, fireNum, from,
  )
  const cs = coastStatus(
    inputs.investable, inputs.monthlyContribution, fireNum,
    inputs.currentAge, inputs.targetRetirementAge,
    inputs.expectedReturnRate, inputs.inflationRate, from,
  )
  const monthsToRetirement = (inputs.targetRetirementAge - inputs.currentAge) * 12
  const requiredMonthly = requiredMonthlyContribution(
    inputs.investable, fireNum,
    inputs.expectedReturnRate, inputs.inflationRate, monthsToRetirement,
  )
  return {
    variant, expenses, fireNumber: fireNum, fiDate,
    yearsToFi: months === null ? null : months / 12,
    coastNumber: cs.coastNumber, coasting: cs.coasting, coastCrossingDate: cs.crossingDate,
    requiredMonthly,
  }
}

export function buildForecast(inputs: ForecastInputs, from: DateTime = DateTime.now()): VariantForecast[] {
  const variants: FireVariant[] = ['lean', 'regular', 'fat']
  return variants.map(v => buildVariantForecast(inputs, v, from))
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm run test -- forecast`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/fire/forecast.ts src/lib/fire/forecast.test.ts
git commit -m "feat: buildForecast aggregator for Lean/Regular/Fat variants"
```

---

## Task 6: Re-export new modules (`index.ts`)

**Files:**
- Modify: `src/lib/fire/index.ts`

- [ ] **Step 1: Add exports**

Replace `src/lib/fire/index.ts` with:

```ts
export * from './types'
export * from './metrics'
export * from './netWorthSeries'
export * from './projection'
export * from './activeInputs'
export * from './contributionRate'
export * from './coast'
export * from './requiredContribution'
export * from './forecast'
```

- [ ] **Step 2: Verify the whole unit suite still passes**

Run: `npm run test`
Expected: PASS — all prior suites plus the four new ones (contributionRate, coast, requiredContribution, forecast) and the extended projection suite.

- [ ] **Step 3: Commit**

```bash
git add src/lib/fire/index.ts
git commit -m "chore: re-export Phase 3 forecast modules"
```

---

## Task 7: Forecast chart component (`ForecastChart.vue`)

**Files:**
- Create: `src/components/ForecastChart.vue`

Follows the `NetWorthChart.vue` unovis pattern. Plots the growth curve plus flat FIRE-number and Coast reference lines.

- [ ] **Step 1: Create the component**

Create `src/components/ForecastChart.vue`:

```vue
<script setup lang="ts">
import { VisXYContainer, VisLine, VisAxis } from '@unovis/vue'
import type { ProjectionPoint } from '../lib/fire/projection'
import { DateTime } from 'luxon'

const props = defineProps<{
  points: ProjectionPoint[]
  fireNumber: number
  coastNumber: number
}>()

type D = { t: number; v: number; fire: number; coast: number }

const data = (): D[] => props.points.map(p => ({
  t: DateTime.fromISO(p.date).toMillis(),
  v: p.value,
  fire: props.fireNumber,
  coast: props.coastNumber,
}))

const x = (d: D) => d.t
const yValue = (d: D) => d.v
const yFire = (d: D) => d.fire
const yCoast = (d: D) => d.coast
const tickFormatX = (t: number | Date) =>
  DateTime.fromMillis(typeof t === 'number' ? t : t.getTime()).toFormat('yyyy')
</script>

<template>
  <VisXYContainer :data="data()" :height="280">
    <VisLine :x="x" :y="yValue" color="#6366f1" />
    <VisLine :x="x" :y="yFire" color="#22c55e" :line-dash-array="[4, 4]" />
    <VisLine :x="x" :y="yCoast" color="#f59e0b" :line-dash-array="[4, 4]" />
    <VisAxis type="x" :tick-format="tickFormatX" />
    <VisAxis type="y" />
  </VisXYContainer>
</template>
```

- [ ] **Step 2: Type-check**

Run: `npm run build` (runs `vue-tsc` then `vite build`)
Expected: no type errors from `ForecastChart.vue`. (Component is not yet routed; this only confirms it compiles.)

- [ ] **Step 3: Commit**

```bash
git add src/components/ForecastChart.vue
git commit -m "feat: ForecastChart with FIRE-number and coast reference lines"
```

---

## Task 8: Forecast page + route + nav (`Forecast.vue`)

**Files:**
- Create: `src/pages/Forecast.vue`
- Modify: `src/router.ts`
- Modify: `src/App.vue`

- [ ] **Step 1: Create the page**

Create `src/pages/Forecast.vue`:

```vue
<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { DateTime } from 'luxon'
import { useFireProfileStore } from '../stores/fireProfile'
import { useAccountsStore } from '../stores/accounts'
import { useContributionsStore } from '../stores/contributions'
import {
  activeFireInputs, investableNetWorth, savingsRate,
  derivedMonthlyContribution, buildForecast, projectionSeries, monthsToFire,
  realMonthlyReturn,
  type ForecastInputs, type VariantForecast, type FireVariant,
} from '../lib/fire'
import ForecastChart from '../components/ForecastChart.vue'

const fp = useFireProfileStore()
const acc = useAccountsStore()
const contrib = useContributionsStore()

const open = ref(false) // what-if drawer

onMounted(async () => {
  await Promise.all([fp.load(), acc.load(), contrib.load(DateTime.now().year)])
})

const inputs = computed(() => activeFireInputs(acc.accounts, acc.allBalances))
const investable = computed(() => investableNetWorth(inputs.value.accounts, inputs.value.balances))
const asOf = computed(() => DateTime.now().toISODate()!)

// Baseline derived monthly contribution (actual trailing-12mo, else savingsRate estimate).
const baseline = computed(() => {
  if (!fp.profile) return { monthly: 0, estimated: true }
  const rate = savingsRate(inputs.value.accounts, inputs.value.balances, fp.profile.annualIncome, asOf.value)
  const estimateMonthly = (fp.profile.annualIncome * rate) / 12
  return derivedMonthlyContribution(contrib.txns, asOf.value, estimateMonthly)
})

// What-if override state. null = use baseline/profile value.
const ov = reactive<{ monthly: number | null; returnRate: number | null; inflation: number | null; retireAge: number | null }>({
  monthly: null, returnRate: null, inflation: null, retireAge: null,
})
const isScenario = computed(() =>
  ov.monthly !== null || ov.returnRate !== null || ov.inflation !== null || ov.retireAge !== null)

function reset() {
  ov.monthly = null; ov.returnRate = null; ov.inflation = null; ov.retireAge = null
}

// Effective slider values (override ?? baseline/profile).
const effMonthly = computed(() => ov.monthly ?? baseline.value.monthly)
const effReturn = computed(() => ov.returnRate ?? fp.profile?.expectedReturnRate ?? 0)
const effInflation = computed(() => ov.inflation ?? fp.profile?.inflationRate ?? 0)
const effRetireAge = computed(() => ov.retireAge ?? fp.profile?.targetRetirementAge ?? 0)

const forecastInputs = computed<ForecastInputs | null>(() => {
  if (!fp.profile) return null
  return {
    currentAge: fp.profile.currentAge,
    targetRetirementAge: effRetireAge.value,
    annualExpensesTarget: fp.profile.annualExpensesTarget,
    leanFireAnnualExpenses: fp.profile.leanFireAnnualExpenses,
    fatFireAnnualExpenses: fp.profile.fatFireAnnualExpenses,
    expectedReturnRate: effReturn.value,
    inflationRate: effInflation.value,
    investable: investable.value,
    monthlyContribution: effMonthly.value,
  }
})

const forecasts = computed<VariantForecast[]>(() =>
  forecastInputs.value ? buildForecast(forecastInputs.value) : [])

const regular = computed(() => forecasts.value.find(f => f.variant === 'regular') ?? null)

// Chart horizon: months to the Regular FI date, else months to retirement, else 30y. Padded.
const chartPoints = computed(() => {
  const fi = forecastInputs.value
  const reg = regular.value
  if (!fi || !reg) return []
  const mr = realMonthlyReturn(fi.expectedReturnRate, fi.inflationRate)
  const toFi = monthsToFire(fi.investable, fi.monthlyContribution, mr, reg.fireNumber)
  const toRet = Math.max(0, (fi.targetRetirementAge - fi.currentAge) * 12)
  const horizon = Math.min(1200, Math.round((toFi ?? toRet ?? 360) * 1.1) + 12)
  return projectionSeries(fi.investable, fi.monthlyContribution, fi.expectedReturnRate, fi.inflationRate, horizon)
})

const fmt = (n: number) => n.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })
const labels: Record<FireVariant, string> = { lean: 'Lean FIRE', regular: 'FIRE', fat: 'Fat FIRE' }

function coastText(v: VariantForecast): string {
  if (v.coasting) return 'Coasting ✓'
  return v.coastCrossingDate ? `by ${v.coastCrossingDate.toFormat('LLL yyyy')}` : '—'
}

// Slider models bound to override values, seeded from effective values on open.
const sMonthly = computed({ get: () => effMonthly.value, set: v => { ov.monthly = v } })
const sReturn = computed({ get: () => effReturn.value, set: v => { ov.returnRate = v } })
const sInflation = computed({ get: () => effInflation.value, set: v => { ov.inflation = v } })
const sRetire = computed({ get: () => effRetireAge.value, set: v => { ov.retireAge = v } })
</script>

<template>
  <div class="p-6 space-y-6">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold">Forecast</h1>
      <UButton icon="i-lucide-sliders-horizontal" color="neutral" variant="outline" @click="open = true">
        What-if
      </UButton>
    </div>

    <div v-if="isScenario" class="flex items-center gap-3 text-sm rounded-lg border border-warning/40 bg-warning/10 px-3 py-2">
      <span class="text-warning font-medium">Scenario — not saved</span>
      <UButton size="xs" color="neutral" variant="ghost" @click="reset">Reset to baseline</UButton>
    </div>

    <div v-if="regular" class="border border-default rounded-lg p-4">
      <h2 class="font-semibold mb-2">Projected growth — {{ labels.regular }}</h2>
      <ForecastChart :points="chartPoints" :fire-number="regular.fireNumber" :coast-number="regular.coastNumber" />
      <div class="flex gap-4 text-xs text-muted mt-2">
        <span><span class="inline-block w-3 border-t-2 align-middle" style="border-color:#6366f1" /> Investable</span>
        <span><span class="inline-block w-3 border-t-2 border-dashed align-middle" style="border-color:#22c55e" /> FIRE number</span>
        <span><span class="inline-block w-3 border-t-2 border-dashed align-middle" style="border-color:#f59e0b" /> Coast number</span>
      </div>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-3 gap-4">
      <UCard v-for="v in forecasts" :key="v.variant" :class="v.variant === 'regular' ? 'ring-1 ring-primary' : ''">
        <div class="text-sm text-muted uppercase tracking-wide">{{ labels[v.variant] }}</div>
        <div class="text-xl font-semibold mt-1">{{ fmt(v.fireNumber) }}</div>
        <dl class="mt-3 space-y-1 text-sm">
          <div class="flex justify-between"><dt class="text-muted">Projected FI</dt>
            <dd>{{ v.fiDate ? v.fiDate.toFormat('LLL yyyy') : '—' }}</dd></div>
          <div class="flex justify-between"><dt class="text-muted">Years to FI</dt>
            <dd>{{ v.yearsToFi !== null ? v.yearsToFi.toFixed(1) : '—' }}</dd></div>
          <div class="flex justify-between"><dt class="text-muted">Coast number</dt>
            <dd>{{ fmt(v.coastNumber) }}</dd></div>
          <div class="flex justify-between"><dt class="text-muted">Coast status</dt>
            <dd :class="v.coasting ? 'text-success' : ''">{{ coastText(v) }}</dd></div>
          <div class="flex justify-between"><dt class="text-muted">Required / mo</dt>
            <dd>{{ v.requiredMonthly !== null ? fmt(v.requiredMonthly) : '—' }}</dd></div>
        </dl>
      </UCard>
    </div>

    <USlideover v-model:open="open" title="What-if scenario" side="right">
      <template #body>
        <div class="space-y-6">
          <div>
            <div class="flex justify-between text-sm mb-1">
              <label>Monthly contribution</label><span>{{ fmt(sMonthly) }}</span>
            </div>
            <USlider v-model="sMonthly" :min="0" :max="20000" :step="100" />
          </div>
          <div>
            <div class="flex justify-between text-sm mb-1">
              <label>Expected return</label><span>{{ (sReturn * 100).toFixed(1) }}%</span>
            </div>
            <USlider v-model="sReturn" :min="0" :max="0.15" :step="0.005" />
          </div>
          <div>
            <div class="flex justify-between text-sm mb-1">
              <label>Inflation</label><span>{{ (sInflation * 100).toFixed(1) }}%</span>
            </div>
            <USlider v-model="sInflation" :min="0" :max="0.1" :step="0.005" />
          </div>
          <div>
            <div class="flex justify-between text-sm mb-1">
              <label>Retirement age</label><span>{{ sRetire }}</span>
            </div>
            <USlider v-model="sRetire" :min="fp.profile?.currentAge ?? 18" :max="80" :step="1" />
          </div>
          <div v-if="isScenario" class="pt-2 border-t border-default">
            <UButton block color="neutral" variant="soft" @click="reset">Reset to baseline</UButton>
          </div>
          <p v-if="baseline.estimated" class="text-xs text-muted">
            Baseline contribution is estimated — less than 12 months of contribution history.
          </p>
        </div>
      </template>
    </USlideover>
  </div>
</template>
```

- [ ] **Step 2: Add the route**

In `src/router.ts`, add this entry to the `routes` array immediately before the `settings` route:

```ts
  { path: '/forecast', name: 'forecast', component: () => import('./pages/Forecast.vue') },
```

- [ ] **Step 3: Enable the nav link**

In `src/App.vue`, replace the disabled Forecast link in the `links` array:

```ts
  { label: 'Forecast', icon: 'i-lucide-trending-up', disabled: true },
```

with:

```ts
  { label: 'Forecast', to: '/forecast', icon: 'i-lucide-trending-up' },
```

- [ ] **Step 4: Type-check and build**

Run: `npm run build`
Expected: `vue-tsc` and `vite build` both succeed with no errors.

> If `USlider` or `USlideover` props differ in this NuxtUI 4 build, check the installed component's prop names (e.g. `v-model:open` vs `v-model`, `side` vs `direction`) in `node_modules/@nuxt/ui` and adjust. Keep the override/reset logic unchanged.

- [ ] **Step 5: Register components if needed**

If `npm run build` reports `USlideover` or `USlider` as unknown in `components.d.ts`, add them following the existing entries in `components.d.ts` (mirror how `UCard`/`UButton`/`UAlert` are declared). Re-run `npm run build`.

- [ ] **Step 6: Commit**

```bash
git add src/pages/Forecast.vue src/router.ts src/App.vue components.d.ts
git commit -m "feat: Forecast page with variant cards, chart, and what-if drawer"
```

---

## Task 9: Dashboard reconciliation (`Dashboard.vue`)

**Files:**
- Modify: `src/pages/Dashboard.vue`

Replace the crude savings-rate-derived monthly figure with the derived contribution; recompute Savings Rate from it; drop the permanent "Approximate" hint (show an "Estimated" hint only when falling back).

- [ ] **Step 1: Update the script**

In `src/pages/Dashboard.vue`:

a) Extend the import list to add `derivedMonthlyContribution` and import the contributions store. The import block becomes:

```ts
import {
  fireNumber, currentNetWorth, investableNetWorth, fiProgress,
  netWorthOverTime, projectedFiDate, savingsRate, activeFireInputs,
  derivedMonthlyContribution,
} from '../lib/fire'
import { useContributionsStore } from '../stores/contributions'
```

b) Add the store and load it. Replace:

```ts
const fp = useFireProfileStore()
const acc = useAccountsStore()
onMounted(async () => { await Promise.all([fp.load(), acc.load()]) })
```

with:

```ts
const fp = useFireProfileStore()
const acc = useAccountsStore()
const contrib = useContributionsStore()
onMounted(async () => {
  await Promise.all([fp.load(), acc.load(), contrib.load(DateTime.now().year)])
})
```

c) Replace the `rate` and `fiDate` computeds:

```ts
const rate = computed(() => fp.profile
  ? savingsRate(fireAccounts.value, fireBalances.value, fp.profile.annualIncome, DateTime.now().toISODate()!)
  : 0)
const fiDate = computed(() => {
  if (!fp.profile) return null
  const monthly = (fp.profile.annualIncome * rate.value) / 12
  return projectedFiDate(investable.value, monthly, fp.profile.expectedReturnRate, fp.profile.inflationRate, fireNum.value)
})
```

with:

```ts
const asOf = computed(() => DateTime.now().toISODate()!)
const contribution = computed(() => {
  if (!fp.profile) return { monthly: 0, estimated: true }
  const estRate = savingsRate(fireAccounts.value, fireBalances.value, fp.profile.annualIncome, asOf.value)
  const estimateMonthly = (fp.profile.annualIncome * estRate) / 12
  return derivedMonthlyContribution(contrib.txns, asOf.value, estimateMonthly)
})
const rate = computed(() => fp.profile && fp.profile.annualIncome > 0
  ? (contribution.value.monthly * 12) / fp.profile.annualIncome
  : 0)
const fiDate = computed(() => fp.profile
  ? projectedFiDate(investable.value, contribution.value.monthly, fp.profile.expectedReturnRate, fp.profile.inflationRate, fireNum.value)
  : null)
```

- [ ] **Step 2: Update the Savings Rate card hint**

In the template, replace:

```html
<StatCard label="Savings Rate" :value="`${(rate * 100).toFixed(1)}%`" hint="Approximate — refined in Phase 2" />
```

with:

```html
<StatCard
  label="Savings Rate"
  :value="`${(rate * 100).toFixed(1)}%`"
  :hint="contribution.estimated ? 'Estimated — under 12 months of contribution history' : undefined"
/>
```

- [ ] **Step 3: Type-check and build**

Run: `npm run build`
Expected: success, no type errors.

- [ ] **Step 4: Verify unit suite still green**

Run: `npm run test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/pages/Dashboard.vue
git commit -m "feat: Dashboard uses derived contribution; refine savings rate"
```

---

## Task 10: Full verification + manual smoke test

**Files:** none (verification only)

- [ ] **Step 1: Run all automated gates**

```bash
npm run test          # all Vitest suites green
npm run build         # vue-tsc + vite build clean
cd src-tauri && cargo build && cargo test && cd ..   # Rust unaffected, still green
```

Expected: every command exits 0. (Rust is unchanged this phase but must stay green.)

- [ ] **Step 2: Manual GUI smoke test**

```bash
npm run tauri dev
```

Walk this checklist:
1. Forecast nav link is enabled and routes to the page.
2. Three variant cards render (Lean / FIRE / Fat); FIRE card is visually emphasized.
3. Each card shows FIRE number, Projected FI, Years to FI, Coast number, Coast status, Required/mo.
4. The chart renders with three lines (investable curve + FIRE + Coast reference lines).
5. Click **What-if** → drawer opens with four sliders seeded at baseline.
6. Drag a slider → "Scenario — not saved" banner appears; cards + chart update live.
7. Click **Reset to baseline** → overrides clear, banner disappears, values return to baseline.
8. Open the Dashboard → "Projected FI Date" matches the Forecast FIRE card's Projected FI; "Savings Rate" has no permanent "Approximate" hint (an "Estimated" hint appears only with <12 months of contribution data).

- [ ] **Step 3: Update project memory**

Update `project_trackmyfi_design.md`: mark Phase 3 status as BUILT/merged with the date, and note the `savingsRate` Phase 1 approximation has been superseded by `contributionRate.ts` (it now survives only as the `<12-month` fallback).

---

## Self-Review Notes

- **Spec coverage:** Coast FIRE (Task 2), Lean/Fat variants (Task 5), required contribution (Task 3), what-if planner ephemeral drawer (Task 8), projection chart (Tasks 4+7), derived contribution baseline + Dashboard reconciliation (Tasks 1+9), nav slot (Task 8). All spec sections map to a task.
- **No new tables/migrations/Rust:** confirmed — Task 1 reuses `contrib.load()` → existing `list_contribution_txns_cmd`.
- **Type consistency:** `ForecastInputs`/`VariantForecast`/`FireVariant` defined in Task 5 are consumed unchanged in Task 8; `ProjectionPoint` defined in Task 4 consumed in Task 7; `DerivedContribution` from Task 1 consumed in Tasks 8 & 9.
- **Known risk:** NuxtUI 4 `USlider`/`USlideover` prop names — Task 8 Steps 4–5 cover verifying/adjusting and registering them.
