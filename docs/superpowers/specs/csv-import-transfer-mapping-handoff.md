# Handoff: CSV Import Transfer Mapping

## What This Is

A follow-on feature to the CSV import categorization work (landed in commits 94c6788–869737e on `main`). The goal is to let the import wizard identify payment/transfer rows in a CSV and mark them as transfers between accounts, rather than forcing users to either skip those rows or import them incorrectly as income.

## Why It Matters

When someone imports a credit card CSV, the individual purchases are expenses (discretionary, fixed, etc.). But the payment row — the money sent from checking to pay the card — is not a new expense. It's a transfer from checking → credit card. Importing it as income is wrong; it distorts spending totals. Right now the only workaround is to manually uncheck payment rows before importing.

## Current State of the Import Wizard

**File:** `src/components/ImportWizard.vue`

Three-step flow:
1. Choose account + upload CSV
2. Map columns (date, description, amount/credit+debit), set default category — now also auto-detects column names and lets you set per-mapping default category
3. Preview rows, override category per-row, quick-add keyword rules, uncheck duplicates, import

**What `confirmImport` currently produces for every row:**
```typescript
{
  accountId: accountId.value!,
  transferAccountId: null,          // ← always null, no transfer support
  amount: p.amount,
  description: p.description,
  date: p.date,
  type: p.type,                     // 'income' | 'expense' — never 'transfer'
  category: rowCategories.value[i] ?? p.category,
  isContribution: false,
  importSource: 'csv',
  updateBalance: false,
  createdAt: now,
}
```

**The DB already supports transfers:** `txn.transfer_account_id` is `INTEGER` nullable in the schema, and `Transaction.transferAccountId` is `Option<i32>` in Rust. The plumbing exists — just not exposed in the importer.

## Mapping Config (already in place)

`MappingConfig` in `src/lib/csv/mapping.ts`:
```typescript
export interface MappingConfig {
  dateColumn: string
  descriptionColumn: string
  dateFormat: string
  amountMode: 'single' | 'split'
  amountColumn: string
  amountSign: 'negative-is-expense' | 'positive-is-expense'
  creditColumn: string
  debitColumn: string
  invertSplit: boolean
  defaultCategory: string
}
```

`ParsedTransaction` in the same file:
```typescript
export interface ParsedTransaction {
  date: string
  amount: number
  description: string
  type: 'income' | 'expense'
  category: string
}
```

## What Needs to Be Built

### 1. Transfer rules (similar to category rules)

A new concept: "if description matches keyword, mark this row as a transfer to account X." Example: "PAYMENT THANK YOU" on a Chase card CSV → transfer to the checking account.

These should be stored as part of either:
- **Option A:** A global `transfer_rules` table: `keyword TEXT, transfer_account_id INTEGER` — same pattern as `category_rules`
- **Option B:** A field in `MappingConfig`: `transferRules: { keyword: string; transferAccountId: number }[]` — per-mapping, saved with the mapping name

Option B is probably better here since the "Chase card payment" keyword maps to a specific checking account, which differs per user setup. A global rule would need to know which checking account, and users might have multiple.

### 2. `ParsedTransaction` needs a `transferAccountId` field

Extend `ParsedTransaction`:
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

### 3. `applyMapping` applies transfer rules

After applying category rules, check transfer rules. If matched: set `type: 'transfer'`, `transferAccountId: rule.transferAccountId`, and optionally `category: 'uncategorized'` (transfers don't need a spending category).

### 4. Step 2 UI: transfer rules section

Below the Category Defaults section in the wizard's Step 2, add a "Transfer Rules" section. Each rule: a keyword input + an account picker (`USelect` over `accountsStore.accounts`). Users can add/remove rules. These save into the mapping config.

### 5. Step 3 preview: show transfer rows differently

Rows matched as transfers should show "transfer → [Account Name]" in the Type column rather than "income"/"expense", and the Category select should be hidden/disabled for those rows (transfers don't have a spending category).

### 6. `confirmImport` handles transfer rows

```typescript
category: p.type === 'transfer' ? 'uncategorized' : (rowCategories.value[i] ?? p.category),
transferAccountId: p.transferAccountId ?? null,
```

## Key Files

| File | Relevance |
|---|---|
| `src/components/ImportWizard.vue` | Main wizard — 3-step flow, all the logic |
| `src/lib/csv/mapping.ts` | `MappingConfig`, `ParsedTransaction`, `applyMapping`, `autoDetectMapping` |
| `src/lib/csv/mapping.test.ts` | Tests for the above — add transfer rule tests here |
| `src/lib/types/ImportMapping.ts` | TypeScript type for saved mappings |
| `src-tauri/src/models.rs` | `ImportMapping` Rust model |
| `src/stores/accounts.ts` | Account list for the transfer destination picker |
| `src/lib/transactions/constants.ts` | `TRANSACTION_TYPES` — already includes `'transfer'` |

## Open Design Questions

1. **Where do transfer rules live?** In `MappingConfig` (per-mapping) or a global table? Per-mapping feels right because "pay Chase card" → specific checking account is user+account specific.

2. **Should transfer rows be included or excluded by default in the Step 3 preview?** The duplicate-detection logic currently unchecks suspected dupes. Transfer rows probably should be included by default (the payment is real).

3. **What happens to `rowCategories[i]` for transfer rows?** Either hide the category selector entirely in the UI, or keep it but ignore it on import for `type === 'transfer'` rows. Hiding is cleaner UX.

4. **Should the transfer account auto-save?** If a user maps "PAYMENT" → checking account in the wizard, should that persist as a saved rule they can re-apply next import? Yes — this is the main value of per-mapping transfer rules.

## Suggested Starting Point

Run `/brainstorm` or describe the feature to kick off the design. The existing category rules implementation in this session (same pattern: keyword → behavior, saved with mapping) is the closest analogue to follow.
