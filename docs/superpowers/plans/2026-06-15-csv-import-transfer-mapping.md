# CSV Import Transfer Mapping Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the CSV import wizard so rows matching a keyword can be marked as a transfer to a specific account, rather than being imported as income.

**Architecture:** Transfer rules live inside `MappingConfig` (per-mapping, serialized with the mapping name) so a "pay Chase card" keyword maps to the user's specific checking account. `applyMapping` reads `config.transferRules` and — after resolving the category — checks if any rule matches; a match overrides `type` to `'transfer'`, sets `transferAccountId`, and forces `category` to `'uncategorized'`. The wizard Step 2 gains a UI for adding/removing rules; Step 3 renders transfer rows differently and disables the category select for them.

**Tech Stack:** TypeScript, Vue 3 (Composition API), Vitest, Nuxt UI (USelect / UInput / UButton), Pinia

---

## File Map

| File | Change |
|---|---|
| `src/lib/csv/mapping.ts` | Add `TransferRuleInput`, extend `MappingConfig` + `ParsedTransaction`, update `applyMapping` |
| `src/lib/csv/mapping.test.ts` | Update 4 `toEqual` assertions + add transfer rule describe block |
| `src/components/ImportWizard.vue` | Config init, Step 2 transfer rules UI, Step 3 type display + category hiding, `confirmImport` |

---

## Task 1: Extend types in `mapping.ts`

**Files:**
- Modify: `src/lib/csv/mapping.ts`

- [ ] **Step 1: Add `TransferRuleInput` interface and extend `MappingConfig`**

Replace the current `MappingConfig` interface (lines 6–17) with:

```typescript
export interface TransferRuleInput {
  keyword: string
  transferAccountId: number
}

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
  transferRules: TransferRuleInput[]
}
```

- [ ] **Step 2: Extend `ParsedTransaction` to include `'transfer'` type and `transferAccountId`**

Replace the current `ParsedTransaction` interface (lines 19–25) with:

```typescript
export interface ParsedTransaction {
  date: string
  amount: number
  description: string
  type: 'income' | 'expense' | 'transfer'
  category: string
  transferAccountId: number | null
}
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/csv/mapping.ts
git commit -m "feat: add TransferRuleInput, extend MappingConfig and ParsedTransaction for transfer support"
```

---

## Task 2: Update existing tests + write failing transfer rule tests

**Files:**
- Modify: `src/lib/csv/mapping.test.ts`

The `ParsedTransaction` type now includes `transferAccountId`. Four existing tests use `.toEqual([...])` with full objects — they will fail until updated. All other tests use `.toMatchObject` or only assert specific properties, so they're fine.

- [ ] **Step 1: Update `applyMapping maps rows to parsed transactions` (line 44)**

Change the `toEqual` expectation to include `transferAccountId: null` on both objects:

```typescript
expect(applyMapping(rows, config)).toEqual([
  { date: '2026-03-01', amount: 40, description: 'Coffee', type: 'expense', category: 'uncategorized', transferAccountId: null },
  { date: '2026-03-02', amount: 1500, description: 'Paycheck', type: 'income', category: 'uncategorized', transferAccountId: null },
])
```

- [ ] **Step 2: Update the three split-mode `toEqual` tests (lines ~94, ~101, ~108)**

```typescript
// "maps debit to expense and credit to income for a non-liability account"
expect(applyMapping(rows, splitConfig, false)).toEqual([
  { date: '2026-03-01', amount: 42.5, description: 'Coffee', type: 'expense', category: 'uncategorized', transferAccountId: null },
  { date: '2026-03-02', amount: 1500, description: 'Paycheck', type: 'income', category: 'uncategorized', transferAccountId: null },
])

// "flips direction for a liability account (credit = expense, debit = income)"
expect(applyMapping(rows, splitConfig, true)).toEqual([
  { date: '2026-03-01', amount: 42.5, description: 'Coffee', type: 'income', category: 'uncategorized', transferAccountId: null },
  { date: '2026-03-02', amount: 1500, description: 'Paycheck', type: 'expense', category: 'uncategorized', transferAccountId: null },
])

// "inverts direction when invertSplit is true (non-liability: credit becomes expense)"
expect(applyMapping(rows, { ...splitConfig, invertSplit: true }, false)).toEqual([
  { date: '2026-03-01', amount: 42.5, description: 'Coffee', type: 'income', category: 'uncategorized', transferAccountId: null },
  { date: '2026-03-02', amount: 1500, description: 'Paycheck', type: 'expense', category: 'uncategorized', transferAccountId: null },
])
```

- [ ] **Step 3: Run tests — expect these 4 to fail (implementation not done yet), all others to pass**

```bash
npx vitest run src/lib/csv/mapping.test.ts
```

Expected: 4 failures mentioning `transferAccountId`, all other tests green.

- [ ] **Step 4: Add new describe block for transfer rules at the end of `mapping.test.ts`**

The config fixture below is the same one at the top of the file but extended with `transferRules`. A helper `baseConfig` is the standard config with `transferRules: []` added for consistency.

```typescript
describe('applyMapping with transfer rules', () => {
  const transferConfig: MappingConfig = {
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
    transferRules: [{ keyword: 'payment thank you', transferAccountId: 42 }],
  }

  it('marks a matching row as transfer and sets transferAccountId', () => {
    const result = applyMapping(
      [{ 'Posting Date': '03/05/2026', Amount: '1200.00', Description: 'PAYMENT THANK YOU' }],
      transferConfig,
    )
    expect(result[0].type).toBe('transfer')
    expect(result[0].transferAccountId).toBe(42)
  })

  it('forces category to uncategorized for transfer rows', () => {
    const result = applyMapping(
      [{ 'Posting Date': '03/05/2026', Amount: '1200.00', Description: 'PAYMENT THANK YOU' }],
      { ...transferConfig, defaultCategory: 'discretionary' },
    )
    expect(result[0].category).toBe('uncategorized')
  })

  it('transfer rule matching is case-insensitive', () => {
    const result = applyMapping(
      [{ 'Posting Date': '03/05/2026', Amount: '1200.00', Description: 'payment thank you' }],
      transferConfig,
    )
    expect(result[0].type).toBe('transfer')
    expect(result[0].transferAccountId).toBe(42)
  })

  it('transfer rule takes priority over category rule', () => {
    const result = applyMapping(
      [{ 'Posting Date': '03/05/2026', Amount: '1200.00', Description: 'PAYMENT THANK YOU' }],
      transferConfig,
      false,
      [{ keyword: 'payment', category: 'fixed' }],
    )
    expect(result[0].type).toBe('transfer')
    expect(result[0].category).toBe('uncategorized')
  })

  it('non-matching rows have transferAccountId null', () => {
    const result = applyMapping(
      [{ 'Posting Date': '03/01/2026', Amount: '-40.00', Description: 'Coffee' }],
      transferConfig,
    )
    expect(result[0].type).toBe('expense')
    expect(result[0].transferAccountId).toBeNull()
  })

  it('first matching transfer rule wins', () => {
    const result = applyMapping(
      [{ 'Posting Date': '03/05/2026', Amount: '1200.00', Description: 'PAYMENT THANK YOU ACH' }],
      {
        ...transferConfig,
        transferRules: [
          { keyword: 'payment thank you', transferAccountId: 42 },
          { keyword: 'ach', transferAccountId: 99 },
        ],
      },
    )
    expect(result[0].transferAccountId).toBe(42)
  })

  it('empty transferRules leaves type and transferAccountId unchanged', () => {
    const result = applyMapping(
      [{ 'Posting Date': '03/01/2026', Amount: '-40.00', Description: 'Coffee' }],
      { ...transferConfig, transferRules: [] },
    )
    expect(result[0].type).toBe('expense')
    expect(result[0].transferAccountId).toBeNull()
  })
})
```

- [ ] **Step 5: Run tests — expect all 7 new tests to fail (no impl yet)**

```bash
npx vitest run src/lib/csv/mapping.test.ts
```

Expected: the 4 previously failing tests still fail, plus 7 new failures.

- [ ] **Step 6: Commit tests**

```bash
git add src/lib/csv/mapping.test.ts
git commit -m "test: update ParsedTransaction expectations and add transfer rule tests"
```

---

## Task 3: Implement transfer rules in `applyMapping`

**Files:**
- Modify: `src/lib/csv/mapping.ts`

- [ ] **Step 1: Add `transferRules: []` to `MappingConfig` default in `config` constant at top of test file**

Wait — this is handled in the test file by the `transferConfig` fixture, not in `mapping.ts` itself. The field is optional in the interface. No separate default needed in `mapping.ts`.

- [ ] **Step 2: Update `applyMapping` to apply transfer rules**

Replace the `applyMapping` function body. The key change: after resolving `category` from category rules, check `config.transferRules`. A transfer match overrides `type`, `category`, and adds `transferAccountId`. Non-transfer rows get `transferAccountId: null`.

```typescript
/** Transform raw CSV objects into parsed transactions using a mapping config. */
export function applyMapping(
  rows: Record<string, string>[],
  config: MappingConfig,
  isLiabilityAccount = false,
  rules: CategoryRuleInput[] = [],
): ParsedTransaction[] {
  const transferRules = config.transferRules ?? []

  return rows.map((row) => {
    const date = isoDate(row[config.dateColumn] ?? '', config.dateFormat)
    const description = row[config.descriptionColumn] ?? ''
    const descLower = description.toLowerCase()

    const matchedCategoryRule = rules.find((r) => descLower.includes(r.keyword.toLowerCase()))
    const category = matchedCategoryRule ? matchedCategoryRule.category : config.defaultCategory

    const matchedTransferRule = transferRules.find((r) => descLower.includes(r.keyword.toLowerCase()))

    if (matchedTransferRule) {
      // Determine amount (still need to parse it even for transfers)
      let amount: number
      if (config.amountMode === 'split') {
        const { amount: a } = resolveSplit(row, config, isLiabilityAccount)
        amount = a
      } else {
        amount = Math.abs(parseAmount(row[config.amountColumn] ?? '0'))
      }
      return {
        date,
        amount,
        description,
        type: 'transfer' as const,
        category: 'uncategorized',
        transferAccountId: matchedTransferRule.transferAccountId,
      }
    }

    if (config.amountMode === 'split') {
      const { amount, type } = resolveSplit(row, config, isLiabilityAccount)
      return { date, amount, description, type, category, transferAccountId: null }
    }

    const signed = parseAmount(row[config.amountColumn] ?? '0')
    const isExpense = config.amountSign === 'negative-is-expense' ? signed < 0 : signed > 0
    return {
      date,
      amount: Math.abs(signed),
      description,
      type: isExpense ? 'expense' : 'income',
      category,
      transferAccountId: null,
    }
  })
}
```

- [ ] **Step 3: Run all tests — expect all to pass**

```bash
npx vitest run src/lib/csv/mapping.test.ts
```

Expected: all tests green, 0 failures.

- [ ] **Step 4: Commit**

```bash
git add src/lib/csv/mapping.ts
git commit -m "feat: applyMapping checks transfer rules, sets type='transfer' and transferAccountId on match"
```

---

## Task 4: ImportWizard.vue — config init + Step 2 Transfer Rules UI

**Files:**
- Modify: `src/components/ImportWizard.vue`

The config default needs `transferRules: []`. Step 2 gets a new "Transfer Rules" section after Category Defaults.

- [ ] **Step 1: Add `transferRules: []` to config default and add local refs for the add-rule form**

In the `<script setup>` block, update the `config` ref default and add two new refs:

```typescript
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
  transferRules: [],
})

// after the existing newRuleKeyword / newRuleCategory refs:
const newTransferKeyword = ref('')
const newTransferAccountId = ref<number | undefined>(undefined)
```

- [ ] **Step 2: Add `addTransferRule` and `removeTransferRule` functions**

Place these after `saveMapping`:

```typescript
function addTransferRule() {
  if (!newTransferKeyword.value.trim() || newTransferAccountId.value == null) return
  config.value.transferRules.push({
    keyword: newTransferKeyword.value.trim().toLowerCase(),
    transferAccountId: newTransferAccountId.value,
  })
  newTransferKeyword.value = ''
  newTransferAccountId.value = undefined
}

function removeTransferRule(index: number) {
  config.value.transferRules.splice(index, 1)
}
```

- [ ] **Step 3: Add the Transfer Rules section to the Step 2 template**

Insert this after the closing `</div>` of the CATEGORY DEFAULTS section and before the SAVE MAPPING section:

```html
<!-- TRANSFER RULES -->
<div class="space-y-3">
  <p class="text-xs font-semibold uppercase tracking-wide text-muted">Transfer Rules</p>
  <p class="text-xs text-muted">Mark rows matching a keyword as a transfer to an account.</p>

  <div v-if="config.transferRules.length" class="space-y-1">
    <div
      v-for="(rule, i) in config.transferRules"
      :key="i"
      class="flex items-center gap-2 text-sm"
    >
      <span class="flex-1 truncate">{{ rule.keyword }}</span>
      <span class="text-muted">→</span>
      <span class="flex-1 truncate">
        {{ accountsStore.accounts.find((a) => a.id === rule.transferAccountId)?.name ?? 'Unknown account' }}
      </span>
      <UButton size="xs" variant="ghost" color="red" @click="removeTransferRule(i)">Remove</UButton>
    </div>
  </div>

  <div class="flex gap-2 items-center">
    <UInput
      v-model="newTransferKeyword"
      placeholder="keyword (e.g. payment thank you)"
      size="xs"
      class="flex-1"
    />
    <USelect
      v-model="newTransferAccountId"
      :items="accountsStore.accounts.map((a) => ({ label: a.name, value: a.id }))"
      placeholder="Transfer to account"
      size="xs"
      class="w-44"
    />
    <UButton
      size="xs"
      variant="soft"
      :disabled="!newTransferKeyword.trim() || newTransferAccountId == null"
      @click="addTransferRule"
    >
      Add rule
    </UButton>
  </div>
</div>
```

- [ ] **Step 4: Commit**

```bash
git add src/components/ImportWizard.vue
git commit -m "feat: add Transfer Rules section to import wizard Step 2"
```

---

## Task 5: ImportWizard.vue — Step 3 display + `confirmImport`

**Files:**
- Modify: `src/components/ImportWizard.vue`

Transfer rows need to display "transfer → Account Name" in the Type column, hide the Category select, and pass `transferAccountId` through on import.

- [ ] **Step 1: Update the Step 3 Type column cell**

Find the `<td>{{ p.type }}</td>` line in the Step 3 table and replace it:

```html
<td>
  <template v-if="p.type === 'transfer'">
    transfer → {{ accountsStore.accounts.find((a) => a.id === p.transferAccountId)?.name ?? '?' }}
  </template>
  <template v-else>{{ p.type }}</template>
</td>
```

- [ ] **Step 2: Conditionally hide the Category select for transfer rows**

Find the `<td>` containing the `USelect` for `rowCategories[i]` and wrap it:

```html
<td>
  <USelect
    v-if="p.type !== 'transfer'"
    v-model="rowCategories[i]"
    :items="categoryItems"
    size="xs"
    class="w-36"
    @update:model-value="manuallyOverridden[i] = true"
  />
  <span v-else class="text-xs text-muted">—</span>
</td>
```

- [ ] **Step 3: Update `confirmImport` to use `transferAccountId` and force category for transfer rows**

Find the `.map(({ p, i }) => ({` block inside `confirmImport` and update the `category` and `transferAccountId` lines:

```typescript
async function confirmImport() {
  if (accountId.value == null) return
  const now = DateTime.now().toISO()!
  const toInsert = parsed.value
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
      updateBalance: false,
      createdAt: now,
    }))
  await bulkCreateTransactions(toInsert)
  await txnStore.load()
  emit('done')
}
```

- [ ] **Step 4: Verify transfer rows initialize correctly in `goToPreview`**

Check `goToPreview` — transfer rows should be **included** by default (not filtered by duplicate detection, which is fine — duplicates are detected by date+amount+description and a transfer row is a real transaction). The `rowCategories` for transfer rows initializes to `p.category` which `applyMapping` already sets to `'uncategorized'`. No code changes needed here, just confirm the existing logic is correct:

```typescript
function goToPreview() {
  step.value = 3
  include.value = parsed.value.map((_, i) => !dupes.value[i])  // transfer rows included unless exact dupe
  rowCategories.value = parsed.value.map((p) => p.category)     // transfer rows get 'uncategorized'
  manuallyOverridden.value = parsed.value.map(() => false)
}
```

This is correct as-is. The category select is hidden for transfer rows in the template, so `rowCategories[i]` won't be user-editable for them. `confirmImport` ignores `rowCategories[i]` for transfer rows anyway.

- [ ] **Step 5: Commit**

```bash
git add src/components/ImportWizard.vue
git commit -m "feat: show transfer rows in Step 3 preview, pass transferAccountId on import"
```

---

## Task 6: Run full test suite

- [ ] **Step 1: Run all tests**

```bash
npx vitest run
```

Expected: all tests pass, 0 failures.

- [ ] **Step 2: If any test fails, read the error output and fix the root cause before marking complete**

Common failure modes:
- TypeScript type error in `applyMapping` because a `ParsedTransaction` return site is missing `transferAccountId` — add `transferAccountId: null` to that return object
- A `toEqual` assertion in `mapping.test.ts` missing `transferAccountId` — add it
- `config.transferRules` undefined error in `applyMapping` — ensure `config.transferRules ?? []` is used
