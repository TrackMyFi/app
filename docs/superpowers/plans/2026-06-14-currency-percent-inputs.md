# Currency & Percent Input Components Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace all raw `UInput type="number"` dollar-amount and rate fields with two thin `UNumberInput`-based wrapper components — `CurrencyInput` and `PercentInput` — that display formatted values automatically.

**Architecture:** Two new single-file components live in `src/components/`. They wrap `UNumberInput` with the appropriate `formatOptions` and defaults. No store or DB changes; both bind to the same value types already in use. Callers drop in the new component exactly where `UInput` was, removing the `type`, `step`, and `placeholder` attributes.

**Tech Stack:** Vue 3 `<script setup>`, NuxtUI v4 `UNumberInput`, `Intl.NumberFormat` (used internally by reka-ui's NumberField)

---

## File Map

| Action | File | Responsibility |
|--------|------|----------------|
| Create | `src/components/CurrencyInput.vue` | USD-formatted number input, no +/− buttons |
| Create | `src/components/PercentInput.vue` | Percent-formatted number input, with +/− buttons |
| Modify | `src/components/TransactionForm.vue` | Replace `form.amount` input |
| Modify | `src/components/BalanceForm.vue` | Replace `balance` input |
| Modify | `src/components/BalanceRow.vue` | Replace `draftBalance` input |
| Modify | `src/components/PaycheckForm.vue` | Replace 9 currency inputs (gross, net, 5 taxes, deductions, employer match) |
| Modify | `src/pages/Settings.vue` | Replace 4 currency inputs + 2 percent inputs |
| Modify | `src/pages/Onboarding.vue` | Replace 4 currency inputs + 2 percent inputs |
| Modify | `src/pages/Budget.vue` | Replace savings target input; change ref type from `number | string` to `number | null` |

---

### Task 1: Create `CurrencyInput.vue`

**Files:**
- Create: `src/components/CurrencyInput.vue`

**Background:**
`UNumberInput` (NuxtUI v4 — wraps reka-ui's `NumberFieldRoot`) accepts:
- `:format-options` — passed to `Intl.NumberFormat`; `{ style: 'currency', currency: 'USD' }` renders `$1,500.00`
- `:increment="false"` / `:decrement="false"` — hides the +/− buttons (the template guards on `!!props.increment`)
- `:step` — how much each keypress up/down changes the value
- `modelValue` / `@update:model-value` — emits `number | null`; `null` when field is cleared

Attr inheritance (`inheritAttrs: true` by default) passes `class` and other attrs through to `UNumberInput`, which declares `class` as a prop and applies it to its root wrapper. So `<CurrencyInput class="w-full" />` correctly sets the width.

- [ ] **Step 1: Create the component**

```vue
<!-- src/components/CurrencyInput.vue -->
<script setup lang="ts">
const props = defineProps<{
  modelValue: number | null
  placeholder?: string
  min?: number
}>()

const emit = defineEmits<{ 'update:modelValue': [value: number | null] }>()
</script>

<template>
  <UNumberInput
    :model-value="props.modelValue"
    :format-options="{ style: 'currency', currency: 'USD' }"
    :step="0.01"
    :min="props.min"
    :placeholder="props.placeholder"
    :increment="false"
    :decrement="false"
    @update:model-value="emit('update:modelValue', $event)"
  />
</template>
```

- [ ] **Step 2: Commit**

```bash
git add src/components/CurrencyInput.vue
git commit -m "feat: add CurrencyInput component"
```

---

### Task 2: Create `PercentInput.vue`

**Files:**
- Create: `src/components/PercentInput.vue`

**Background:**
`Intl.NumberFormat` with `style: 'percent'` multiplies by 100 for display: model value `0.07` renders as `7%`. This matches how the data is stored (decimal fractions). Step `0.01` means each +/− click moves 1 percentage point (0.07 → 0.08), which is appropriate for return/inflation rates.

- [ ] **Step 1: Create the component**

```vue
<!-- src/components/PercentInput.vue -->
<script setup lang="ts">
const props = defineProps<{
  modelValue: number | null
  min?: number
  max?: number
}>()

const emit = defineEmits<{ 'update:modelValue': [value: number | null] }>()
</script>

<template>
  <UNumberInput
    :model-value="props.modelValue"
    :format-options="{ style: 'percent' }"
    :step="0.01"
    :min="props.min ?? 0"
    :max="props.max"
    @update:model-value="emit('update:modelValue', $event)"
  />
</template>
```

- [ ] **Step 2: Commit**

```bash
git add src/components/PercentInput.vue
git commit -m "feat: add PercentInput component"
```

---

### Task 3: Update `TransactionForm.vue`

**Files:**
- Modify: `src/components/TransactionForm.vue`

**Background:**
One currency field: `form.amount`. The `form` reactive object has `amount: 0` (a `number`). `UNumberInput` can emit `null` when cleared; guard with `?? 0` in the `save()` call so a cleared field never submits `NaN` or `null` to the DB.

- [ ] **Step 1: Add import and replace input**

In the `<script setup>` block, `CurrencyInput` is auto-imported (the project uses Vite auto-imports), so no explicit import is needed.

Replace the amount `UInput` in the template (around line 140):

```vue
<!-- Before -->
<UInput v-model.number="form.amount" type="number" step="0.01" placeholder="0.00" class="w-full" />

<!-- After -->
<CurrencyInput v-model="form.amount" class="w-full" />
```

- [ ] **Step 2: Guard null in `save()`**

The `save()` function passes `form.amount` directly. Add `?? 0` to prevent a null from reaching the store if the field was cleared:

```ts
// In the store.create({...}) and store.update({...}) calls, change:
amount: form.amount,
// to:
amount: form.amount ?? 0,
```

- [ ] **Step 3: Run the dev build to check for type errors**

```bash
npm run build -- --mode development 2>&1 | head -40
```

Expected: no type errors related to `TransactionForm`.

- [ ] **Step 4: Commit**

```bash
git add src/components/TransactionForm.vue
git commit -m "feat: use CurrencyInput for transaction amount"
```

---

### Task 4: Update `BalanceForm.vue` and `BalanceRow.vue`

**Files:**
- Modify: `src/components/BalanceForm.vue`
- Modify: `src/components/BalanceRow.vue`

**Background:**
Both components have a single balance currency field. `BalanceForm` has `balance = ref<number>(0)`. `BalanceRow` has `draftBalance = ref<number>(props.balance.balance)`. Guard `?? 0` at save time.

- [ ] **Step 1: Update `BalanceForm.vue`**

Replace the balance `UInput` (around line 28) and update `onSubmit`:

```vue
<!-- Template: replace -->
<UInput v-model.number="balance" type="number" step="0.01" placeholder="0.00" class="w-36" />
<!-- with -->
<CurrencyInput v-model="balance" class="w-36" />
```

Also remove the now-redundant label text `"Balance ($)"`:

```vue
<!-- Before -->
<UFormField label="Balance ($)">
<!-- After -->
<UFormField label="Balance">
```

In `onSubmit`, guard the balance:

```ts
async function onSubmit() {
  await store.addBalanceSnapshot({
    accountId: props.accountId,
    balance: balance.value ?? 0,
    recordedAt: recordedAt.value,
  })
  balance.value = 0
  recordedAt.value = DateTime.now().toISODate()!
}
```

- [ ] **Step 2: Update `BalanceRow.vue`**

Replace the balance `UInput` (around lines 55-60):

```vue
<!-- Before -->
<UInput
  v-model.number="draftBalance"
  type="number"
  step="0.01"
  class="w-32"
/>
<!-- After -->
<CurrencyInput v-model="draftBalance" class="w-32" />
```

In `save()`, guard the balance:

```ts
async function save() {
  await store.updateBalanceSnapshot({
    id: props.balance.id,
    balance: draftBalance.value ?? 0,
    recordedAt: draftDate.value,
  })
  isEditing.value = false
}
```

- [ ] **Step 3: Commit**

```bash
git add src/components/BalanceForm.vue src/components/BalanceRow.vue
git commit -m "feat: use CurrencyInput for balance fields"
```

---

### Task 5: Update `PaycheckForm.vue`

**Files:**
- Modify: `src/components/PaycheckForm.vue`

**Background:**
9 currency fields total: `grossAmount`, `netAmount`, `federalTax`, `stateTax`, `localTax`, `socialSecurityTax`, `medicareTax`, `ded.amount` (per deduction row), and `em.amount` (per employer match row). All are `number` in the form reactive state. Guard `?? 0` at save time for the flat fields; deduction/match amounts are already 0-initialized per row.

- [ ] **Step 1: Replace the Amounts section inputs (lines ~271–275)**

```vue
<!-- Before -->
<UInput v-model.number="form.grossAmount" type="number" step="0.01" placeholder="0.00" class="w-full" />
...
<UInput v-model.number="form.netAmount" type="number" step="0.01" placeholder="0.00" class="w-full" />

<!-- After -->
<CurrencyInput v-model="form.grossAmount" class="w-full" />
...
<CurrencyInput v-model="form.netAmount" class="w-full" />
```

- [ ] **Step 2: Replace the Taxes section inputs (lines ~285–303)**

```vue
<!-- Before (Federal, State, Local, Social Security, Medicare) -->
<UInput v-model.number="form.federalTax" type="number" step="0.01" placeholder="0.00" />
<UInput v-model.number="form.stateTax" type="number" step="0.01" placeholder="0.00" />
<UInput v-model.number="form.localTax" type="number" step="0.01" placeholder="0.00" />
<UInput v-model.number="form.socialSecurityTax" type="number" step="0.01" placeholder="0.00" class="w-full" />
<UInput v-model.number="form.medicareTax" type="number" step="0.01" placeholder="0.00" class="w-full" />

<!-- After -->
<CurrencyInput v-model="form.federalTax" />
<CurrencyInput v-model="form.stateTax" />
<CurrencyInput v-model="form.localTax" />
<CurrencyInput v-model="form.socialSecurityTax" class="w-full" />
<CurrencyInput v-model="form.medicareTax" class="w-full" />
```

- [ ] **Step 3: Replace the deduction amount input (line ~319)**

```vue
<!-- Before -->
<UInput v-model.number="ded.amount" type="number" step="0.01" placeholder="0.00" class="w-24" />

<!-- After -->
<CurrencyInput v-model="ded.amount" class="w-24" />
```

- [ ] **Step 4: Replace the employer match amount input (line ~351)**

```vue
<!-- Before -->
<UInput v-model.number="em.amount" type="number" step="0.01" placeholder="0.00" class="w-28" />

<!-- After -->
<CurrencyInput v-model="em.amount" class="w-28" />
```

- [ ] **Step 5: Guard nulls in `save()`**

In both the `store.update({...})` and `store.create({...})` calls, add `?? 0` to the 7 flat tax/amount fields:

```ts
grossAmount: form.grossAmount ?? 0,
netAmount: form.netAmount ?? 0,
federalTax: form.federalTax ?? 0,
stateTax: form.stateTax ?? 0,
localTax: form.localTax ?? 0,
socialSecurityTax: form.socialSecurityTax ?? 0,
medicareTax: form.medicareTax ?? 0,
```

Deductions and employer match amounts are already handled per-row (they default to `0` and are spread as-is into the arrays).

- [ ] **Step 6: Commit**

```bash
git add src/components/PaycheckForm.vue
git commit -m "feat: use CurrencyInput for paycheck amount fields"
```

---

### Task 6: Update `Settings.vue`

**Files:**
- Modify: `src/pages/Settings.vue`

**Background:**
4 currency fields: `annualExpensesTarget`, `annualIncome`, `leanFireAnnualExpenses`, `fatFireAnnualExpenses`. The lean/fat fields are `number | null` in the form type — `CurrencyInput` handles null natively, no guard needed. The first two are required and should guard `?? 0`.

2 percent fields: `expectedReturnRate`, `inflationRate` — stored as decimals (0.07 = 7%). `PercentInput` displays them as percentages automatically. The form labels (currently hint "e.g. 0.07") should be updated to reflect the new percentage display.

- [ ] **Step 1: Replace currency inputs and update form labels**

```vue
<!-- Before -->
<UFormField label="Annual expenses target">
  <UInput v-model.number="form.annualExpensesTarget" type="number" class="w-full" />
</UFormField>
<UFormField label="Annual income">
  <UInput v-model.number="form.annualIncome" type="number" class="w-full" />
</UFormField>
<UFormField label="Lean FIRE expenses (optional)">
  <UInput v-model.number="form.leanFireAnnualExpenses" type="number" class="w-full" />
</UFormField>
<UFormField label="Fat FIRE expenses (optional)">
  <UInput v-model.number="form.fatFireAnnualExpenses" type="number" class="w-full" />
</UFormField>

<!-- After -->
<UFormField label="Annual expenses target">
  <CurrencyInput v-model="form.annualExpensesTarget" class="w-full" />
</UFormField>
<UFormField label="Annual income">
  <CurrencyInput v-model="form.annualIncome" class="w-full" />
</UFormField>
<UFormField label="Lean FIRE expenses (optional)">
  <CurrencyInput v-model="form.leanFireAnnualExpenses" class="w-full" />
</UFormField>
<UFormField label="Fat FIRE expenses (optional)">
  <CurrencyInput v-model="form.fatFireAnnualExpenses" class="w-full" />
</UFormField>
```

- [ ] **Step 2: Replace percent inputs and update labels**

```vue
<!-- Before -->
<UFormField label="Expected return rate (e.g. 0.07)">
  <UInput v-model.number="form.expectedReturnRate" type="number" step="0.01" class="w-full" />
</UFormField>
<UFormField label="Inflation rate (e.g. 0.03)">
  <UInput v-model.number="form.inflationRate" type="number" step="0.01" class="w-full" />
</UFormField>

<!-- After -->
<UFormField label="Expected return rate">
  <PercentInput v-model="form.expectedReturnRate" class="w-full" />
</UFormField>
<UFormField label="Inflation rate">
  <PercentInput v-model="form.inflationRate" class="w-full" />
</UFormField>
```

- [ ] **Step 3: Guard nulls in `onSubmit()`**

```ts
async function onSubmit() {
  const profile: FireProfile = {
    currentAge: form.currentAge,
    targetRetirementAge: form.targetRetirementAge,
    annualExpensesTarget: form.annualExpensesTarget ?? 0,
    leanFireAnnualExpenses: form.leanFireAnnualExpenses,   // already number | null
    fatFireAnnualExpenses: form.fatFireAnnualExpenses,     // already number | null
    annualIncome: form.annualIncome ?? 0,
    expectedReturnRate: form.expectedReturnRate ?? 0,
    inflationRate: form.inflationRate ?? 0,
    hsaCoverage: form.hsaCoverage,
  }
  await store.save(profile)
}
```

- [ ] **Step 4: Commit**

```bash
git add src/pages/Settings.vue
git commit -m "feat: use CurrencyInput and PercentInput in Settings"
```

---

### Task 7: Update `Onboarding.vue`

**Files:**
- Modify: `src/pages/Onboarding.vue`

**Background:**
Same 4 currency + 2 percent fields as Settings. The Onboarding page also has hint text on the percent fields ("A common estimate is 7% (0.07)…") that should drop the `(0.07)` decimal notation since the input now displays the value as a percentage.

- [ ] **Step 1: Replace currency inputs in step 2 (expenses)**

```vue
<!-- Before -->
<UInput v-model.number="form.annualExpensesTarget" type="number" class="w-full" />
...
<UInput v-model.number="form.leanFireAnnualExpenses" type="number" class="w-full" placeholder="e.g. 30000" />
...
<UInput v-model.number="form.fatFireAnnualExpenses" type="number" class="w-full" placeholder="e.g. 80000" />

<!-- After -->
<CurrencyInput v-model="form.annualExpensesTarget" class="w-full" />
...
<CurrencyInput v-model="form.leanFireAnnualExpenses" class="w-full" />
...
<CurrencyInput v-model="form.fatFireAnnualExpenses" class="w-full" />
```

- [ ] **Step 2: Replace currency + percent inputs in step 3 (income & growth)**

```vue
<!-- Before -->
<UInput v-model.number="form.annualIncome" type="number" class="w-full" />
...
<UFormField label="Expected annual return rate" hint="The average yearly return on your investments. A common estimate is 7% (0.07) for a diversified portfolio.">
  <UInput v-model.number="form.expectedReturnRate" type="number" step="0.01" class="w-full" placeholder="0.07" />
</UFormField>
<UFormField label="Expected inflation rate" hint="How much purchasing power erodes each year. 3% (0.03) is a common estimate.">
  <UInput v-model.number="form.inflationRate" type="number" step="0.01" class="w-full" placeholder="0.03" />
</UFormField>

<!-- After -->
<CurrencyInput v-model="form.annualIncome" class="w-full" />
...
<UFormField label="Expected annual return rate" hint="The average yearly return on your investments. A common estimate is 7% for a diversified portfolio.">
  <PercentInput v-model="form.expectedReturnRate" class="w-full" />
</UFormField>
<UFormField label="Expected inflation rate" hint="How much purchasing power erodes each year. 3% is a common estimate.">
  <PercentInput v-model="form.inflationRate" class="w-full" />
</UFormField>
```

- [ ] **Step 3: Guard nulls in `finish()`**

```ts
const profile: FireProfile = {
  currentAge: form.currentAge,
  targetRetirementAge: form.targetRetirementAge,
  annualExpensesTarget: form.annualExpensesTarget ?? 0,
  leanFireAnnualExpenses: form.leanFireAnnualExpenses,   // already number | null
  fatFireAnnualExpenses: form.fatFireAnnualExpenses,     // already number | null
  annualIncome: form.annualIncome ?? 0,
  expectedReturnRate: form.expectedReturnRate ?? 0,
  inflationRate: form.inflationRate ?? 0,
  hsaCoverage: form.hsaCoverage,
  onboardingCompleted: false,
}
```

- [ ] **Step 4: Commit**

```bash
git add src/pages/Onboarding.vue
git commit -m "feat: use CurrencyInput and PercentInput in Onboarding"
```

---

### Task 8: Update `Budget.vue`

**Files:**
- Modify: `src/pages/Budget.vue`

**Background:**
The savings target inline edit currently uses `ref<number | string>('')` so it can represent "not yet set" as an empty string. `UNumberInput` uses `null` for empty instead of empty string. Change the ref type to `number | null` and update the three functions that touch it.

- [ ] **Step 1: Update the `targetInput` ref and related functions**

```ts
// Before
const targetInput = ref<number | string>('')

function openTargetEdit() {
  targetInput.value = store.target?.savingsTarget ?? ''
  editingTarget.value = true
}

function cancelTargetEdit() {
  editingTarget.value = false
  targetInput.value = ''
}

async function saveTarget() {
  const val = Number(targetInput.value)
  if (!isNaN(val) && val >= 0) {
    await store.setTarget(val)
  }
  editingTarget.value = false
  targetInput.value = ''
}

// After
const targetInput = ref<number | null>(null)

function openTargetEdit() {
  targetInput.value = store.target?.savingsTarget ?? null
  editingTarget.value = true
}

function cancelTargetEdit() {
  editingTarget.value = false
  targetInput.value = null
}

async function saveTarget() {
  if (targetInput.value !== null && targetInput.value >= 0) {
    await store.setTarget(targetInput.value)
  }
  editingTarget.value = false
  targetInput.value = null
}
```

- [ ] **Step 2: Replace the input in the template**

```vue
<!-- Before -->
<UInput
  v-model="targetInput"
  type="number"
  size="xs"
  class="w-24"
  placeholder="0"
  @keyup.enter="saveTarget"
  @keyup.escape="cancelTargetEdit"
/>

<!-- After -->
<CurrencyInput
  v-model="targetInput"
  size="xs"
  class="w-24"
  @keyup.enter="saveTarget"
  @keyup.escape="cancelTargetEdit"
/>
```

- [ ] **Step 3: Commit**

```bash
git add src/pages/Budget.vue
git commit -m "feat: use CurrencyInput for budget savings target"
```

---

### Task 9: Verify in the running app

**Files:** None (verification only)

- [ ] **Step 1: Start the dev server**

```bash
npm run dev
```

- [ ] **Step 2: Check currency inputs**

Open each of the following and confirm the fields display dollar-formatted values (e.g. `$1,500.00`), no +/− buttons, and accept typed input:

- **Transactions page** → Add Transaction → Amount field
- **Paychecks page** → Add Paycheck → Gross, Net, and all 5 tax fields; add a deduction row and employer match row
- **Accounts page** → open an account → balance snapshot form + any existing snapshot Edit mode
- **Budget page** → click the pencil icon next to the savings sub-label → confirm currency input appears inline
- **Settings** → FIRE Profile section → all four expense/income fields
- **Onboarding** (if accessible via route `/onboarding`) → Steps 2 and 3

- [ ] **Step 3: Check percent inputs**

In **Settings → FIRE Profile** and **Onboarding step 3**: confirm `expectedReturnRate` and `inflationRate` display as percentages (e.g. `7%`, `3%`), that the +/− buttons step by 1%, and that saving round-trips correctly (value stored as decimal, not as a whole percentage).

- [ ] **Step 4: Final commit (if any tweaks were needed)**

```bash
git add -p   # stage only intentional changes
git commit -m "fix: post-verification tweaks for currency/percent inputs"
```
