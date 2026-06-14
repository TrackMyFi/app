# Currency & Percent Input Components

**Date:** 2026-06-14  
**Status:** Approved

## Overview

Replace all raw `UInput type="number"` dollar-amount and rate fields with two thin wrapper components — `CurrencyInput` and `PercentInput` — built on NuxtUI's `UNumberInput` with Intl format options. No data model changes; both components bind to the same value types already in use.

## Components

### `src/components/CurrencyInput.vue`

Wraps `UNumberInput` with USD currency formatting.

- **Format:** `{ style: 'currency', currency: 'USD' }` — renders `$1,500.00` in the field
- **Step:** `0.01`
- **Buttons:** No +/− buttons (hidden via NuxtUI `ui` prop class override)
- **Model type:** `number | null` (covers optional lean/fat FIRE fields)
- **Props:** `modelValue: number | null`, `class?: string`, `placeholder?: string`, `min?: number`

### `src/components/PercentInput.vue`

Wraps `UNumberInput` with percent formatting.

- **Format:** `{ style: 'percent' }` — value `0.07` renders as `7%`; no conversion needed since data is stored as decimal fractions
- **Step:** `0.001` (= 0.1% display increment)
- **Buttons:** +/− buttons kept (useful for nudging rates)
- **Model type:** `number | null`
- **Props:** `modelValue: number | null`, `class?: string`

## Fields Updated

### Currency → `CurrencyInput`

| File | Field |
|------|-------|
| `TransactionForm.vue` | `form.amount` |
| `PaycheckForm.vue` | `form.grossAmount`, `form.netAmount`, `form.federalTax`, `form.stateTax`, `form.localTax`, `form.socialSecurityTax`, `form.medicareTax`, `ded.amount`, `em.amount` |
| `BalanceRow.vue` | `draftBalance` |
| `BalanceForm.vue` | `balance` |
| `Settings.vue` | `form.annualExpensesTarget`, `form.annualIncome`, `form.leanFireAnnualExpenses`, `form.fatFireAnnualExpenses` |
| `Onboarding.vue` | `form.annualExpensesTarget`, `form.annualIncome`, `form.leanFireAnnualExpenses`, `form.fatFireAnnualExpenses` |
| `Budget.vue` | `targetInput` (savings target inline edit) |

### Percent → `PercentInput`

| File | Field |
|------|-------|
| `Settings.vue` | `form.expectedReturnRate`, `form.inflationRate` |
| `Onboarding.vue` | `form.expectedReturnRate`, `form.inflationRate` |

## Budget.vue Savings Target

Currently `ref<number | string>('')`; needs to become `ref<number | null>(null)`. The `saveTarget` function updates to:

```ts
async function saveTarget() {
  if (targetInput.value !== null) {
    await store.setTarget(targetInput.value)
  }
  editingTarget.value = false
  targetInput.value = null
}
```

## Fields Not Changed

- Age fields (`currentAge`, `targetRetirementAge`) — plain integer, no formatting needed
- Forecast sliders (`sReturn`, `sInflation`) — these are `USlider` controls, not text inputs
- Budget.vue target display label — read-only formatted string, no input change
- Sync config inputs — text fields (URL, token, password), not numeric
