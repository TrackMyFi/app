# CSV Import — Generate Balance Snapshots Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an opt-in "Generate balance snapshots from these transactions" toggle to the CSV import wizard that cascades account balance history through each imported transaction, with a conditional seed balance prompt when no prior snapshot exists.

**Architecture:** All changes are confined to a new pure-TS utility (`balanceProjection.ts`) and the existing `ImportWizard.vue` component. The Rust `create_transaction` command already handles snapshot generation via `materialize_snapshots` — we switch from `bulkCreateTransactions` to sequential `createTransaction` calls when the toggle is on. No Rust changes, no new API endpoints, no migrations.

**Tech Stack:** Vue 3, TypeScript, Pinia, NuxtUI, Vitest — same as rest of frontend.

---

## File Map

| Action | File | Responsibility |
|---|---|---|
| Create | `src/lib/csv/balanceProjection.ts` | Pure function: project running balances through a list of CSV rows |
| Create | `src/lib/csv/balanceProjection.test.ts` | Unit tests for `projectRunningBalances` |
| Modify | `src/components/ImportWizard.vue` | Reactive state, computeds, Step 2 sidebar section, Step 3 balance column, `confirmImport` update |

---

## Task 1: `projectRunningBalances` utility + tests

**Files:**
- Create: `src/lib/csv/balanceProjection.ts`
- Create: `src/lib/csv/balanceProjection.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `src/lib/csv/balanceProjection.test.ts`:

```typescript
import { describe, it, expect } from 'vitest'
import { projectRunningBalances } from './balanceProjection'

describe('projectRunningBalances', () => {
  it('returns empty array for empty input', () => {
    expect(projectRunningBalances([], [], 1000)).toEqual([])
  })

  it('adds income to the running balance', () => {
    const rows = [
      { date: '2026-03-01', amount: 1000, type: 'income' },
      { date: '2026-03-15', amount: 500, type: 'income' },
    ]
    expect(projectRunningBalances(rows, [true, true], 200)).toEqual([1200, 1700])
  })

  it('subtracts expenses from the running balance', () => {
    const rows = [
      { date: '2026-03-01', amount: 100, type: 'expense' },
      { date: '2026-03-15', amount: 50, type: 'expense' },
    ]
    expect(projectRunningBalances(rows, [true, true], 1000)).toEqual([900, 850])
  })

  it('subtracts transfer rows (source account perspective)', () => {
    const rows = [{ date: '2026-03-01', amount: 200, type: 'transfer' }]
    expect(projectRunningBalances(rows, [true], 1000)).toEqual([800])
  })

  it('excluded rows return null and do not affect running total', () => {
    const rows = [
      { date: '2026-03-01', amount: 100, type: 'expense' },
      { date: '2026-03-15', amount: 50, type: 'expense' },
    ]
    // Row 0 excluded: running total skips it, row 1 sees base 1000
    expect(projectRunningBalances(rows, [false, true], 1000)).toEqual([null, 950])
  })

  it('sorts rows by date before cascading regardless of CSV order', () => {
    // CSV order: March 15 first, March 1 second
    const rows = [
      { date: '2026-03-15', amount: 50, type: 'expense' },
      { date: '2026-03-01', amount: 100, type: 'expense' },
    ]
    // Date-sorted processing: March 1 (-100 → 900), March 15 (-50 → 850)
    // Row 0 (March 15) shows its post-date balance: 850
    // Row 1 (March 1) shows its post-date balance: 900
    expect(projectRunningBalances(rows, [true, true], 1000)).toEqual([850, 900])
  })

  it('handles a mix of included and excluded rows in unsorted date order', () => {
    const rows = [
      { date: '2026-03-15', amount: 50, type: 'expense' },  // included
      { date: '2026-03-01', amount: 100, type: 'expense' }, // excluded
      { date: '2026-03-10', amount: 200, type: 'income' },  // included
    ]
    // Only included: March 10 (+200 → 700), March 15 (-50 → 650)
    expect(projectRunningBalances(rows, [true, false, true], 500)).toEqual([650, null, 700])
  })
})
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
npx vitest run src/lib/csv/balanceProjection.test.ts
```

Expected: error — module not found.

- [ ] **Step 3: Implement `projectRunningBalances`**

Create `src/lib/csv/balanceProjection.ts`:

```typescript
import { signedDelta } from '../transactions/constants'

export interface ProjectionRow {
  date: string
  amount: number
  type: string
}

/**
 * Returns a running balance for each row after cascading through included rows
 * in date order. Excluded rows return null and do not affect the running total.
 * Rows are keyed by their original array index so the result maps 1:1 to input order.
 */
export function projectRunningBalances(
  rows: ProjectionRow[],
  included: boolean[],
  baseBalance: number,
): (number | null)[] {
  // Build (originalIndex, row) pairs for included rows only, sorted oldest-first
  const sorted = rows
    .map((row, i) => ({ row, i }))
    .filter(({ i }) => included[i])
    .sort((a, b) => a.row.date.localeCompare(b.row.date))

  // Walk in date order, accumulate balance, record result per original index
  const balanceAt = new Map<number, number>()
  let running = baseBalance
  for (const { row, i } of sorted) {
    running += signedDelta(row.type, row.amount)
    balanceAt.set(i, running)
  }

  return rows.map((_, i) => balanceAt.get(i) ?? null)
}
```

- [ ] **Step 4: Run tests to confirm they pass**

```bash
npx vitest run src/lib/csv/balanceProjection.test.ts
```

Expected: all 7 tests pass.

- [ ] **Step 5: Run full test suite to check for regressions**

```bash
npx vitest run
```

Expected: all tests pass (95+ Vitest).

- [ ] **Step 6: Commit**

```bash
git add src/lib/csv/balanceProjection.ts src/lib/csv/balanceProjection.test.ts
git commit -m "feat: add projectRunningBalances utility for CSV import balance preview"
```

---

## Task 2: Reactive state, computeds, and watches in ImportWizard

**Files:**
- Modify: `src/components/ImportWizard.vue`

This task adds all the logic — no template changes yet.

- [ ] **Step 1: Add new imports at the top of `<script setup>`**

In `src/components/ImportWizard.vue`, update the import block. Add `watch` to the Vue import (it's already there — confirm), and add the two new API imports:

```typescript
import { projectRunningBalances } from '../lib/csv/balanceProjection'
import { createTransaction } from '../lib/api/transactions'
import { addBalance } from '../lib/api/accounts'
```

The existing transaction import line currently reads:
```typescript
import { bulkCreateTransactions } from '../lib/api/transactions'
```

Replace it with:
```typescript
import { bulkCreateTransactions, createTransaction } from '../lib/api/transactions'
```

Add the accounts API import after the existing accounts store import:
```typescript
import { addBalance } from '../lib/api/accounts'
```

- [ ] **Step 2: Add new reactive state refs**

Add these two refs after the existing `newTransferAccountId` ref declaration (around line 114):

```typescript
const generateSnapshots = ref(false)
const seedBalance = ref(0)
```

- [ ] **Step 3: Add `allParsedRows` computed**

The existing `parsed` computed is gated on `step.value === 3`. We need the parsed rows available in Step 2 for the balance snapshot check. Add `allParsedRows` immediately before the `parsed` computed:

```typescript
const allParsedRows = computed(() =>
  rawRows.value.length > 0 && config.value.dateColumn
    ? applyMapping(rawRows.value, config.value, isLiabilityAccount.value, categoryRules.value)
    : [],
)
```

Then update `parsed` to derive from `allParsedRows` instead of calling `applyMapping` again:

```typescript
const parsed = computed(() => (step.value === 3 ? allParsedRows.value : []))
```

- [ ] **Step 4: Add `priorSnapshot`, `needsSeed`, `baseBalance`, and `runningBalances` computeds**

Add these after `allParsedRows` and the updated `parsed`:

```typescript
const earliestDate = computed(() => {
  const dates = allParsedRows.value.map((r) => r.date).filter(Boolean)
  return dates.length ? dates.reduce((min, d) => (d < min ? d : min)) : ''
})

const priorSnapshot = computed(() => {
  if (!accountId.value || !earliestDate.value || !generateSnapshots.value) return null
  const candidates = accountsStore.allBalances.filter(
    (b) => b.accountId === accountId.value && b.recordedAt <= earliestDate.value,
  )
  if (!candidates.length) return null
  return candidates.reduce((latest, b) =>
    b.recordedAt > latest.recordedAt || (b.recordedAt === latest.recordedAt && b.id > latest.id)
      ? b
      : latest,
  )
})

const needsSeed = computed(() => generateSnapshots.value && priorSnapshot.value === null && earliestDate.value !== '')

const baseBalance = computed(() => priorSnapshot.value?.balance ?? seedBalance.value)

const runningBalances = computed(() =>
  generateSnapshots.value
    ? projectRunningBalances(allParsedRows.value, include.value, baseBalance.value)
    : [],
)
```

- [ ] **Step 5: Add watches to reset state when account changes**

Add these two watches after the existing `watch(parsed, ...)` block:

```typescript
watch(accountId, (newId) => {
  if (newId == null) return
  const acct = accountsStore.accounts.find((a) => a.id === newId)
  if (acct && isInvestment(acct.type)) generateSnapshots.value = false
})

watch(priorSnapshot, (snap) => {
  if (snap !== null) seedBalance.value = 0
})
```

- [ ] **Step 6: Verify the app still compiles (no UI changes yet)**

```bash
npx vue-tsc --noEmit
```

Expected: no errors.

- [ ] **Step 7: Commit**

```bash
git add src/components/ImportWizard.vue
git commit -m "feat: add generateSnapshots state and balance projection computeds to ImportWizard"
```

---

## Task 3: Step 2 sidebar — Generate Balance Snapshots section

**Files:**
- Modify: `src/components/ImportWizard.vue` (template only)

- [ ] **Step 1: Add `isInvestmentAccount` computed to `<script setup>`**

Add this alongside the existing `isLiabilityAccount` computed (around line 46 in the original file):

```typescript
const isInvestmentAccount = computed(() => {
  if (accountId.value == null) return false
  const acct = accountsStore.accounts.find((a) => a.id === accountId.value)
  return acct ? isInvestment(acct.type) : false
})
```

- [ ] **Step 2: Add the Generate Balance Snapshots section to the Step 2 sidebar**

In the `<template>`, locate the Step 2 sidebar `<div>` (the `w-72` panel). It currently ends with the `<!-- SAVE MAPPING -->` section followed by `</div>`. Add the new section immediately after the Save Mapping section, before the closing `</div>` of the sidebar:

```html
<!-- GENERATE BALANCE SNAPSHOTS -->
<div v-if="!isInvestmentAccount && rawRows.length > 0" class="space-y-2">
  <p class="text-xs font-semibold uppercase tracking-wide text-muted">Balance Snapshots</p>
  <USwitch v-model="generateSnapshots" label="Generate balance snapshots" />
  <template v-if="generateSnapshots">
    <div v-if="priorSnapshot" class="text-xs text-muted">
      Will cascade from your
      {{ priorSnapshot.recordedAt }} snapshot of {{ money(priorSnapshot.balance) }}.
    </div>
    <div v-else-if="earliestDate" class="space-y-1">
      <p class="text-xs text-muted">
        No balance found before {{ earliestDate }}. Enter a starting balance:
      </p>
      <UInput
        v-model.number="seedBalance"
        type="number"
        size="xs"
        class="w-full"
        placeholder="0.00"
      />
    </div>
  </template>
</div>
```

The section is visible for all non-investment account types (checking, savings, credit cards/liability) since credit card transaction imports can also generate balance snapshots.

- [ ] **Step 2: Verify the app compiles**

```bash
npx vue-tsc --noEmit
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/ImportWizard.vue
git commit -m "feat: add Generate Balance Snapshots section to ImportWizard Step 2 sidebar"
```

---

## Task 4: Step 3 balance column + updated `confirmImport`

**Files:**
- Modify: `src/components/ImportWizard.vue`

- [ ] **Step 1: Make `previewColumns` a computed so the Balance column can be toggled reactively**

Replace the existing `const previewColumns = [...]` (currently a plain array) with a computed:

```typescript
const previewColumns = computed(() => [
  { id: 'include', header: '' },
  { accessorKey: 'date', header: 'Date' },
  { accessorKey: 'description', header: 'Description' },
  { id: 'type', header: 'Type' },
  { id: 'category', header: 'Category' },
  { id: 'amount', header: 'Amount', meta: { class: { th: 'text-right', td: 'text-right tabular-nums' } } },
  ...(generateSnapshots.value
    ? [{ id: 'balance', header: 'Balance', meta: { class: { th: 'text-right', td: 'text-right tabular-nums' } } }]
    : []),
])
```

- [ ] **Step 2: Add the `#balance-cell` slot to the Step 3 `<UTable>`**

Inside the Step 3 `<UTable>` template (the one with `#include-cell`, `#description-cell`, etc.), add the balance cell slot after `#amount-cell`:

```html
<template #balance-cell="{ row }">
  <span v-if="runningBalances[row.index] != null">
    {{ money(runningBalances[row.index]!) }}
  </span>
  <span v-else class="text-muted">—</span>
</template>
```

- [ ] **Step 3: Update `confirmImport` to handle the snapshot generation path**

Replace the entire `confirmImport` function with:

```typescript
async function confirmImport() {
  if (accountId.value == null) return
  const now = DateTime.now().toISO()!

  const selectedRows = parsed.value
    .map((p, i) => ({ p, i }))
    .filter(({ i }) => include.value[i])
    .map(({ p, i }) => ({
      accountId: accountId.value!,
      transferAccountId: p.transferAccountId ?? null,
      amount: p.amount,
      description: p.description,
      date: p.date,
      type: p.type,
      category: p.type === 'transfer' ? 'uncategorized' : (rowCategories.value[i] ?? p.category),
      isContribution: false,
      importSource: 'csv',
      createdAt: now,
    }))

  if (!generateSnapshots.value) {
    await bulkCreateTransactions(selectedRows.map((r) => ({ ...r, updateBalance: false })))
  } else {
    if (needsSeed.value) {
      await addBalance({
        accountId: accountId.value!,
        balance: seedBalance.value,
        recordedAt: earliestDate.value,
      })
    }
    const sorted = [...selectedRows].sort((a, b) => a.date.localeCompare(b.date))
    for (const row of sorted) {
      await createTransaction({ ...row, updateBalance: true })
    }
    await accountsStore.load()
  }

  await txnStore.load()
  emit('done')
}
```

- [ ] **Step 4: Verify the app compiles**

```bash
npx vue-tsc --noEmit
```

Expected: no errors.

- [ ] **Step 5: Run full test suite**

```bash
npx vitest run
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/components/ImportWizard.vue
git commit -m "feat: add balance preview column and snapshot generation to CSV import"
```

---

## Self-Review Checklist

After implementation, verify:

- [ ] `projectRunningBalances` is unit tested with all 7 cases including out-of-order CSV dates
- [ ] Balance Snapshots section in Step 2 sidebar is hidden for investment accounts
- [ ] Section is hidden until a file has been loaded (`rawRows.length > 0`)
- [ ] Prior snapshot confirmation line shows correct date and amount
- [ ] Seed balance input appears when no prior snapshot exists before earliest CSV date
- [ ] Balance column in Step 3 is hidden when toggle is off
- [ ] Balance column reacts to row include/exclude toggles
- [ ] `confirmImport` with toggle off uses the original `bulkCreateTransactions` path
- [ ] `confirmImport` with toggle on: inserts seed (if needed), processes rows oldest-first, each with `updateBalance: true`
- [ ] `accountsStore.load()` is called after snapshot generation to refresh `allBalances`
