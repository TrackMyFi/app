# CSV Import Wizard Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign the CSV import wizard Step 2 to match the visual style of existing forms and add support for split Credit + Debit columns alongside the existing single Amount column.

**Architecture:** Extend `MappingConfig` in `mapping.ts` with split-mode fields and update `applyMapping` to handle both modes (single amount column vs credit/debit columns). `ImportWizard.vue` gains a mode toggle, conditional fields, a live example card, and a full Step 2 layout overhaul using the section-header/field-label style from `PaycheckForm.vue`.

**Tech Stack:** Vue 3 (Composition API), TypeScript, Nuxt UI components (`UButton`, `USelect`, `UInput`, `USwitch`), Vitest, Luxon

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `src/lib/csv/mapping.ts` | Modify | Add `AmountMode` type, new `MappingConfig` fields, update `applyMapping` signature and split-mode logic, export `parseAmount` |
| `src/lib/csv/mapping.test.ts` | Modify | Update existing config fixture with new required fields; add split-mode test suite |
| `src/components/ImportWizard.vue` | Modify | New config defaults, `isLiabilityAccount` / `exampleEntry` / `canPreview` computeds, full Step 2 template redesign |

---

### Task 1: Extend `MappingConfig` and `applyMapping` with split-mode support (TDD)

**Files:**
- Modify: `src/lib/csv/mapping.ts`
- Modify: `src/lib/csv/mapping.test.ts`

- [ ] **Step 1: Update existing test fixture to include the new required fields**

The `config` constant in the test file must include the four new fields (`amountMode`, `creditColumn`, `debitColumn`, `invertSplit`) or TypeScript will error once the interface is updated.

Open `src/lib/csv/mapping.test.ts` and replace the `config` constant:

```ts
const config: MappingConfig = {
  dateColumn: 'Posting Date',
  descriptionColumn: 'Description',
  dateFormat: 'MM/dd/yyyy',
  amountMode: 'single',
  amountColumn: 'Amount',
  amountSign: 'negative-is-expense',
  creditColumn: '',
  debitColumn: '',
  invertSplit: false,
  defaultCategory: 'uncategorized',
}
```

- [ ] **Step 2: Write failing split-mode tests**

Append this describe block to `src/lib/csv/mapping.test.ts` (after the `detectDuplicates` block):

```ts
describe('applyMapping split mode', () => {
  const splitConfig: MappingConfig = {
    dateColumn: 'Date',
    descriptionColumn: 'Description',
    dateFormat: 'MM/dd/yyyy',
    amountMode: 'split',
    amountColumn: '',
    amountSign: 'negative-is-expense',
    creditColumn: 'Credit',
    debitColumn: 'Debit',
    invertSplit: false,
    defaultCategory: 'uncategorized',
  }

  const rows = [
    { Date: '03/01/2026', Credit: '0', Debit: '42.50', Description: 'Coffee' },
    { Date: '03/02/2026', Credit: '1500.00', Debit: '0', Description: 'Paycheck' },
  ]

  it('maps debit to expense and credit to income for a non-liability account', () => {
    expect(applyMapping(rows, splitConfig, false)).toEqual([
      { date: '2026-03-01', amount: 42.5, description: 'Coffee', type: 'expense', category: 'uncategorized' },
      { date: '2026-03-02', amount: 1500, description: 'Paycheck', type: 'income', category: 'uncategorized' },
    ])
  })

  it('flips direction for a liability account (credit = expense, debit = income)', () => {
    expect(applyMapping(rows, splitConfig, true)).toEqual([
      { date: '2026-03-01', amount: 42.5, description: 'Coffee', type: 'income', category: 'uncategorized' },
      { date: '2026-03-02', amount: 1500, description: 'Paycheck', type: 'expense', category: 'uncategorized' },
    ])
  })

  it('inverts direction when invertSplit is true (non-liability: credit becomes expense)', () => {
    expect(applyMapping(rows, { ...splitConfig, invertSplit: true }, false)).toEqual([
      { date: '2026-03-01', amount: 42.5, description: 'Coffee', type: 'income', category: 'uncategorized' },
      { date: '2026-03-02', amount: 1500, description: 'Paycheck', type: 'expense', category: 'uncategorized' },
    ])
  })

  it('uses the larger column when both credit and debit are non-zero', () => {
    const r = [{ Date: '03/01/2026', Credit: '5.00', Debit: '42.50', Description: 'Mixed' }]
    expect(applyMapping(r, splitConfig, false)[0]).toMatchObject({ amount: 42.5, type: 'expense' })
  })

  it('falls back to amount 0 type expense when both columns are zero or blank', () => {
    const r = [{ Date: '03/01/2026', Credit: '0', Debit: '0', Description: 'Zero' }]
    expect(applyMapping(r, splitConfig, false)[0]).toMatchObject({ amount: 0, type: 'expense' })
  })
})
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
npm test
```

Expected: several FAIL errors under `applyMapping split mode` because `amountMode` and `isLiabilityAccount` don't exist yet.

- [ ] **Step 4: Update `mapping.ts` — add `AmountMode` type and extend `MappingConfig`**

Replace the type section at the top of `src/lib/csv/mapping.ts`:

```ts
export type AmountSign = 'negative-is-expense' | 'positive-is-expense'
export type AmountMode = 'single' | 'split'

export interface MappingConfig {
  dateColumn: string
  descriptionColumn: string
  dateFormat: string
  amountMode: AmountMode
  amountColumn: string
  amountSign: AmountSign
  creditColumn: string
  debitColumn: string
  invertSplit: boolean
  defaultCategory: string
}
```

- [ ] **Step 5: Export `parseAmount` and update `applyMapping`**

Replace `src/lib/csv/mapping.ts` in full:

```ts
import { DateTime } from 'luxon'

export type AmountSign = 'negative-is-expense' | 'positive-is-expense'
export type AmountMode = 'single' | 'split'

export interface MappingConfig {
  dateColumn: string
  descriptionColumn: string
  dateFormat: string
  amountMode: AmountMode
  amountColumn: string
  amountSign: AmountSign
  creditColumn: string
  debitColumn: string
  invertSplit: boolean
  defaultCategory: string
}

export interface ParsedTransaction {
  date: string
  amount: number
  description: string
  type: 'income' | 'expense'
  category: string
}

export interface ExistingRef {
  accountId: number
  date: string
  amount: number
  description: string
}

export function parseAmount(raw: string): number {
  return Number((raw ?? '').replace(/[$,\s]/g, ''))
}

function isoDate(raw: string, format: string): string {
  return DateTime.fromFormat(raw ?? '', format).toISODate() ?? (raw ?? '')
}

function resolveSplit(
  row: Record<string, string>,
  config: MappingConfig,
  isLiabilityAccount: boolean,
): { amount: number; type: 'income' | 'expense' } {
  const credit = parseAmount(row[config.creditColumn])
  const debit = parseAmount(row[config.debitColumn])
  // For a non-liability account: credit = income, debit = expense.
  // For a liability account: credit = expense, debit = income.
  // invertSplit flips the base rule.
  const creditIsIncome = !isLiabilityAccount !== config.invertSplit

  if (credit === 0 && debit === 0) return { amount: 0, type: 'expense' }

  if (credit !== 0 && debit !== 0) {
    if (Math.abs(credit) >= Math.abs(debit)) {
      return { amount: Math.abs(credit), type: creditIsIncome ? 'income' : 'expense' }
    }
    return { amount: Math.abs(debit), type: creditIsIncome ? 'expense' : 'income' }
  }

  if (credit !== 0) return { amount: Math.abs(credit), type: creditIsIncome ? 'income' : 'expense' }
  return { amount: Math.abs(debit), type: creditIsIncome ? 'expense' : 'income' }
}

/** Transform raw CSV objects into parsed transactions using a mapping config. */
export function applyMapping(
  rows: Record<string, string>[],
  config: MappingConfig,
  isLiabilityAccount = false,
): ParsedTransaction[] {
  return rows.map((row) => {
    const date = isoDate(row[config.dateColumn] ?? '', config.dateFormat)
    const description = row[config.descriptionColumn] ?? ''

    if (config.amountMode === 'split') {
      const { amount, type } = resolveSplit(row, config, isLiabilityAccount)
      return { date, amount, description, type, category: config.defaultCategory }
    }

    const signed = parseAmount(row[config.amountColumn] ?? '0')
    const isExpense = config.amountSign === 'negative-is-expense' ? signed < 0 : signed > 0
    return {
      date,
      amount: Math.abs(signed),
      description,
      type: isExpense ? 'expense' : 'income',
      category: config.defaultCategory,
    }
  })
}

/** Return a parallel array: true where the parsed row duplicates an existing transaction. */
export function detectDuplicates(
  parsed: ParsedTransaction[],
  existing: ExistingRef[],
  accountId: number,
): boolean[] {
  const key = (date: string, amount: number, description: string) =>
    `${date}|${amount}|${description}`
  const seen = new Set(
    existing
      .filter((e) => e.accountId === accountId)
      .map((e) => key(e.date, e.amount, e.description)),
  )
  return parsed.map((p) => seen.has(key(p.date, p.amount, p.description)))
}
```

- [ ] **Step 6: Run tests to verify they all pass**

```bash
npm test
```

Expected: all tests PASS, including the new split-mode suite.

- [ ] **Step 7: Commit**

```bash
git add src/lib/csv/mapping.ts src/lib/csv/mapping.test.ts
git commit -m "feat(csv): extend MappingConfig with split credit/debit mode"
```

---

### Task 2: Update `ImportWizard.vue` script section

**Files:**
- Modify: `src/components/ImportWizard.vue`

- [ ] **Step 1: Update the import line to include `isLiability` and `parseAmount`**

Replace the script imports block (lines 1–11):

```ts
import { computed, onMounted, ref } from 'vue'
import { DateTime } from 'luxon'
import { parseCsv } from '../lib/csv/parse'
import { applyMapping, detectDuplicates, parseAmount, type MappingConfig } from '../lib/csv/mapping'
import { bulkCreateTransactions } from '../lib/api/transactions'
import * as mappingApi from '../lib/api/importMappings'
import { useAccountsStore } from '../stores/accounts'
import { useTransactionsStore } from '../stores/transactions'
import { isLiability } from '../lib/accountTypes'
import type { ImportMapping } from '../lib/types/ImportMapping'
```

- [ ] **Step 2: Update the `config` ref defaults to include the new fields**

Replace the `config` ref declaration (currently lines 23–30):

```ts
const config = ref<MappingConfig>({
  dateColumn: '',
  descriptionColumn: '',
  dateFormat: 'MM/dd/yyyy',
  amountMode: 'single',
  amountColumn: '',
  amountSign: 'negative-is-expense',
  creditColumn: '',
  debitColumn: '',
  invertSplit: false,
  defaultCategory: 'uncategorized',
})
```

- [ ] **Step 3: Add `isLiabilityAccount`, `canPreview`, `exampleEntry`, and `money` below the existing computed declarations**

Add after the `dupes` computed (after line 47):

```ts
const isLiabilityAccount = computed(() => {
  if (accountId.value == null) return false
  const account = accountsStore.accounts.find((a) => a.id === accountId.value)
  return account ? isLiability(account.type) : false
})

const canPreview = computed(() => {
  if (!config.value.dateColumn) return false
  if (config.value.amountMode === 'single') return !!config.value.amountColumn
  return !!config.value.creditColumn && !!config.value.debitColumn
})

const exampleEntry = computed(() => {
  if (rawRows.value.length === 0) return null
  const cfg = config.value

  let row: Record<string, string> | undefined
  if (cfg.amountMode === 'single') {
    if (!cfg.amountColumn) return null
    row = rawRows.value.find((r) => parseAmount(r[cfg.amountColumn]) !== 0)
  } else {
    if (!cfg.creditColumn && !cfg.debitColumn) return null
    row = rawRows.value.find((r) => {
      const c = cfg.creditColumn ? parseAmount(r[cfg.creditColumn]) : 0
      const d = cfg.debitColumn ? parseAmount(r[cfg.debitColumn]) : 0
      return c !== 0 || d !== 0
    })
  }

  if (!row) return null
  return {
    raw: row,
    parsed: applyMapping([row], cfg, isLiabilityAccount.value)[0],
  }
})

function money(n: number): string {
  return n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
}
```

- [ ] **Step 4: Update `parsed` computed to pass `isLiabilityAccount`**

Replace the `parsed` computed (currently line 34–36):

```ts
const parsed = computed(() =>
  step.value === 3 ? applyMapping(rawRows.value, config.value, isLiabilityAccount.value) : [],
)
```

- [ ] **Step 5: Update the Preview button disabled condition in `goToPreview`**

The `goToPreview` function itself doesn't need changes — the disabled state is handled by `canPreview` in the template (next task). No script change needed here.

- [ ] **Step 6: Commit**

```bash
git add src/components/ImportWizard.vue
git commit -m "feat(csv): add split-mode computeds and isLiabilityAccount to ImportWizard"
```

---

### Task 3: Redesign the Step 2 template in `ImportWizard.vue`

**Files:**
- Modify: `src/components/ImportWizard.vue`

- [ ] **Step 1: Replace the entire Step 2 `<div v-else-if="step === 2">` block**

In the `<template>`, find and replace everything from `<!-- Step 2: map columns -->` through the closing `</div>` of that step with:

```html
<!-- Step 2: map columns -->
<div v-else-if="step === 2" class="space-y-5">

  <!-- COLUMN MAPPING -->
  <div class="space-y-3">
    <p class="text-xs font-semibold uppercase tracking-wide text-muted">Column Mapping</p>
    <div>
      <p class="text-xs text-muted mb-1">Date column</p>
      <USelect v-model="config.dateColumn" :items="headerItems" placeholder="Select column" class="w-full" />
    </div>
    <div>
      <p class="text-xs text-muted mb-1">Description column</p>
      <USelect v-model="config.descriptionColumn" :items="headerItems" placeholder="Select column" class="w-full" />
    </div>
  </div>

  <!-- AMOUNT -->
  <div class="space-y-3">
    <p class="text-xs font-semibold uppercase tracking-wide text-muted">Amount</p>

    <div class="flex gap-1 p-1 rounded-lg bg-muted">
      <UButton
        type="button"
        :variant="config.amountMode === 'single' ? 'solid' : 'ghost'"
        size="sm"
        class="flex-1"
        @click="config.amountMode = 'single'"
      >Single column</UButton>
      <UButton
        type="button"
        :variant="config.amountMode === 'split' ? 'solid' : 'ghost'"
        size="sm"
        class="flex-1"
        @click="config.amountMode = 'split'"
      >Credit + Debit columns</UButton>
    </div>

    <template v-if="config.amountMode === 'single'">
      <div>
        <p class="text-xs text-muted mb-1">Amount column</p>
        <USelect v-model="config.amountColumn" :items="headerItems" placeholder="Select column" class="w-full" />
      </div>
      <div>
        <p class="text-xs text-muted mb-1">Amount sign</p>
        <USelect
          v-model="config.amountSign"
          :items="[
            { label: 'Negative amounts are expenses', value: 'negative-is-expense' },
            { label: 'Positive amounts are expenses', value: 'positive-is-expense' },
          ]"
          class="w-full"
        />
      </div>
    </template>

    <template v-else>
      <div>
        <p class="text-xs text-muted mb-1">Credit column</p>
        <USelect v-model="config.creditColumn" :items="headerItems" placeholder="Select column" class="w-full" />
      </div>
      <div>
        <p class="text-xs text-muted mb-1">Debit column</p>
        <USelect v-model="config.debitColumn" :items="headerItems" placeholder="Select column" class="w-full" />
      </div>
      <USwitch v-model="config.invertSplit" label="Invert credit/debit direction" />
    </template>

    <!-- Live example card -->
    <div class="rounded-lg border border-default p-3 text-sm space-y-1.5">
      <p class="text-xs text-muted">Example from your CSV</p>
      <template v-if="exampleEntry">
        <div class="flex items-center gap-2">
          <span
            class="text-xs font-medium px-2 py-0.5 rounded-full"
            :class="exampleEntry.parsed.type === 'income'
              ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400'
              : 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400'"
          >{{ exampleEntry.parsed.type }}</span>
          <span class="tabular-nums font-medium">{{ money(exampleEntry.parsed.amount) }}</span>
        </div>
        <p class="text-xs text-muted">
          {{ exampleEntry.parsed.date }} · {{ exampleEntry.parsed.description || '—' }}
          <template v-if="config.amountMode === 'split'">
            · Credit: {{ exampleEntry.raw[config.creditColumn] || '—' }}
            / Debit: {{ exampleEntry.raw[config.debitColumn] || '—' }}
          </template>
          <template v-else>
            · Raw: {{ exampleEntry.raw[config.amountColumn] || '—' }}
          </template>
        </p>
      </template>
      <p v-else class="text-xs text-muted">Select columns to see an example.</p>
    </div>
  </div>

  <!-- FORMAT -->
  <div class="space-y-3">
    <p class="text-xs font-semibold uppercase tracking-wide text-muted">Format</p>
    <div>
      <p class="text-xs text-muted mb-1">Date format</p>
      <UInput v-model="config.dateFormat" placeholder="MM/dd/yyyy" class="w-full" />
    </div>
  </div>

  <!-- SAVE MAPPING -->
  <div class="space-y-3">
    <p class="text-xs font-semibold uppercase tracking-wide text-muted">Save Mapping</p>
    <div v-if="savedMappings.length" class="flex flex-wrap gap-1">
      <UButton v-for="m in savedMappings" :key="m.id" size="xs" variant="soft"
        @click="applySavedMapping(m)">{{ m.name }}</UButton>
    </div>
    <div class="flex gap-2 items-center">
      <UInput v-model="newMappingName" placeholder="Save this mapping as…" class="flex-1" />
      <UButton size="sm" variant="soft" :disabled="!newMappingName" @click="saveMapping">Save mapping</UButton>
    </div>
  </div>

  <div class="flex justify-end pt-2">
    <UButton :disabled="!canPreview" @click="goToPreview">Preview</UButton>
  </div>

</div>
```

- [ ] **Step 2: Commit**

```bash
git add src/components/ImportWizard.vue
git commit -m "feat(csv): redesign step 2 layout and add credit/debit split mode UI"
```
