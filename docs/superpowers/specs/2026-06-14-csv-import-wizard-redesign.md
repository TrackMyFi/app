# CSV Import Wizard Redesign

**Date:** 2026-06-14  
**Status:** Approved

## Overview

Redesign the CSV import wizard's Step 2 (column mapping) to match the visual style of `TransactionForm` and `PaycheckForm`, and add support for split Credit + Debit columns in addition to the existing single Amount column.

## Scope

- `src/components/ImportWizard.vue` — layout and new amount mode UI
- `src/lib/csv/mapping.ts` — `MappingConfig` type + `applyMapping` logic
- `src/lib/csv/mapping.test.ts` — tests for split mode logic

## Layout & Structure

Step 2 is reorganized into four named sections using ALL CAPS section headers and `UFormField` labels, matching the PaycheckForm style:

### COLUMN MAPPING
- Date column (`UFormField` + `USelect`)
- Description column (`UFormField` + `USelect`)

### AMOUNT
- Mode toggle: `UButtonGroup` with two buttons — **Single column** / **Credit + Debit columns**
- Conditional fields based on selected mode (see Amount Mode below)
- Live example card (see Example Card below)

### FORMAT
- Date format (`UFormField` + `UInput`, placeholder `MM/dd/yyyy`)

### SAVE MAPPING
- Mapping name input + Save mapping button inline (same behavior as today)

The Preview button sits at bottom-right, matching the submit button position of the other forms.

## Amount Mode Toggle

### Single column mode (default)
- Amount column (`UFormField` + `USelect`)
- Amount sign (`UFormField` + `USelect`): "Negative amounts are expenses" / "Positive amounts are expenses"

### Credit + Debit columns mode
- Credit column (`UFormField` + `USelect`)
- Debit column (`UFormField` + `USelect`)
- Invert credit/debit direction (`USwitch`) — off by default

**Default credit/debit direction (switch off):**
- Non-liability account: credit column → income, debit column → expense
- Liability account: credit column → expense, debit column → income

When the inversion switch is on, the income/expense assignment above flips.

## Live Example Card

A reactive card sits directly below the amount fields in both modes. It updates whenever columns, sign, or inversion toggle change.

**Source row:** the first CSV row where the relevant column(s) have a non-zero value.

**Display:**
- Single mode: `Raw value: "-$42.50"` → badge showing `expense $42.50`
- Split mode: `Credit: $0.00 / Debit: $42.50` → badge showing `expense $42.50`

The badge uses the existing income (green) / expense (red) color conventions. The card also shows the raw date and description from the example row so the user can verify column selection.

If no usable example row exists (columns not yet selected, or all values are zero/empty), the card shows: `"Select columns to see an example."`

## Type Changes — `MappingConfig`

**File:** `src/lib/csv/mapping.ts`

```ts
export type AmountMode = 'single' | 'split'

export interface MappingConfig {
  dateColumn: string
  descriptionColumn: string
  dateFormat: string
  amountMode: AmountMode      // new; default 'single'
  amountColumn: string        // single mode only
  amountSign: AmountSign      // single mode only
  creditColumn: string        // split mode only
  debitColumn: string         // split mode only
  invertSplit: boolean        // split mode only; default false
  defaultCategory: string
}
```

The default `config` ref in `ImportWizard.vue` is updated to include the new fields with sensible defaults (`amountMode: 'single'`, `creditColumn: ''`, `debitColumn: ''`, `invertSplit: false`).

## Logic Changes — `applyMapping`

**File:** `src/lib/csv/mapping.ts`

```ts
export function applyMapping(
  rows: Record<string, string>[],
  config: MappingConfig,
  isLiabilityAccount: boolean = false,
): ParsedTransaction[]
```

`isLiabilityAccount` is **not** stored in the mapping config — it is derived at call time from the selected account's type using the existing `isLiability()` helper in `accountTypes.ts`.

**Split mode row logic:**
1. Parse both `creditColumn` and `debitColumn` values using existing `parseAmount` (strips `$`, `,`, whitespace)
2. The non-zero column's value is the transaction amount
3. Determine base income/expense direction:
   - Non-liability: credit → income, debit → expense
   - Liability: credit → expense, debit → income
4. If `config.invertSplit` is true, flip the direction
5. If both columns are non-zero, use the larger value and its column's direction
6. If both are zero, emit a row with amount `0` and type `expense` as a fallback

**Single mode** is unchanged from current behavior.

## `ImportWizard.vue` Changes

- Add computed `isLiabilityAccount` derived from the selected account's type
- Pass `isLiabilityAccount` to `applyMapping` in the `parsed` computed and `goToPreview`
- The example card is a computed `exampleRow` that finds the first usable raw row and resolves it by calling `applyMapping([firstUsableRow], config.value, isLiabilityAccount.value)[0]`
- The Preview button's disabled condition expands: single mode requires `dateColumn + amountColumn`; split mode requires `dateColumn + creditColumn + debitColumn`

## Out of Scope

- Changes to Step 1 (account selection + file upload)
- Changes to Step 3 (preview + dedup)
- Changes to the saved mapping storage format (existing saved mappings with no `amountMode` fall back to `'single'`)
