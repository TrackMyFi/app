# CSV Import Categorization & Auto-Mapping Design

**Date:** 2026-06-15

## Overview

Three improvements to the CSV import wizard:

1. **Auto-mapping column detection** — pre-fill the column mapping config from CSV header names
2. **Layered categorization** — global keyword rules + per-mapping default + per-row override
3. **Category rules management** — Settings page + quick-add shortcut in the wizard

## Categorization Priority

Each imported row's category is resolved in this order (highest to lowest):

1. **Per-row override** — user edits in Step 3 preview (always wins)
2. **Global keyword rules** — first matching rule wins (case-insensitive substring match on description)
3. **Per-mapping default category** — stored in `MappingConfig.defaultCategory`
4. `uncategorized` (ultimate fallback, already the current default)

---

## 1. Auto-Mapping Column Detection

### Where

New function `autoDetectMapping(headers: string[], rows: Record<string, string>[]): Partial<MappingConfig>` in `src/lib/csv/mapping.ts`.

Called in `ImportWizard.vue` inside `onFile`, before advancing to Step 2. Detected values are merged into `config.value` but remain fully editable.

### Header matching

Case-insensitive substring match. First alias hit wins.

| Field | Recognized aliases |
|---|---|
| `dateColumn` | `date`, `transaction date`, `trans date`, `posted date`, `posting date` |
| `descriptionColumn` | `description`, `memo`, `details`, `narrative`, `payee`, `merchant`, `name` |
| `amountColumn` | `amount`, `transaction amount`, `amt` |
| `creditColumn` | `credit`, `credits`, `deposit`, `deposits` |
| `debitColumn` | `debit`, `debits`, `withdrawal`, `withdrawals`, `charge` |

If both a credit and a debit column are detected, `amountMode` is set to `'split'`.

### Date format detection

Sample the first non-empty value in the detected date column. Try formats in order: `MM/dd/yyyy`, `yyyy-MM-dd`, `M/d/yyyy`, `dd/MM/yyyy`. Set `dateFormat` to the first that parses successfully via Luxon. If none match, leave the current default.

---

## 2. Default Category in Step 2

`MappingConfig.defaultCategory` already exists; it just isn't surfaced in the UI.

Add a "Category Defaults" section to Step 2 (between Format and Save Mapping) with a single `USelect` bound to `config.defaultCategory`. Uses the existing `categoryItems` from `src/lib/transactions/constants.ts`.

Saved mappings carry this value, so different mappings can carry different defaults (e.g. a credit card mapping defaults to `discretionary`, a checking account mapping stays `uncategorized`).

---

## 3. Global Category Rules

### Data model

New migration `src-tauri/migrations/0008_category_rules.sql`:

```sql
CREATE TABLE category_rules (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  keyword TEXT NOT NULL,
  category TEXT NOT NULL,
  created_at TEXT NOT NULL
);
```

Seed the following defaults in the migration:

| Keyword | Category |
|---|---|
| `netflix` | `fixed` |
| `spotify` | `fixed` |
| `hulu` | `fixed` |
| `disney+` | `fixed` |
| `apple.com/bill` | `fixed` |
| `amazon prime` | `fixed` |
| `youtube premium` | `fixed` |
| `amazon` | `discretionary` |
| `walmart` | `discretionary` |
| `target` | `discretionary` |
| `starbucks` | `discretionary` |
| `mcdonald` | `discretionary` |

### Backend

- New Rust model `CategoryRule` in `src-tauri/src/models.rs`
- Three Tauri commands in a new file `src-tauri/src/commands/category_rules.rs`:
  - `list_category_rules` → `Vec<CategoryRule>`
  - `create_category_rule(keyword, category)` → `CategoryRule`
  - `delete_category_rule(id)`
- Register commands and migration in `src-tauri/src/main.rs` / `src-tauri/src/migrations.rs`

### Frontend API

New file `src/lib/api/categoryRules.ts` with `listCategoryRules`, `createCategoryRule`, `deleteCategoryRule`.

### Applying rules during import

`applyMapping` signature gains an optional `rules: { keyword: string; category: string }[]` parameter. For each row, iterate rules in order; if `description.toLowerCase().includes(rule.keyword.toLowerCase())`, use that rule's category. Otherwise fall back to `config.defaultCategory`.

### Settings page

New "Category Rules" section in `src/pages/Settings.vue`:
- Table listing existing rules: keyword | category | delete button
- Inline add form below: keyword input + category `USelect` + Save button
- Rules load on mount alongside other settings

### Quick-add in wizard

At the bottom of Step 3's preview table, a compact "Add rule" row: keyword `UInput` + category `USelect` + Save button. On save, calls `createCategoryRule` and reloads the rules, which re-applies category resolution to `rowCategories` — but only for rows not yet manually overridden by the user.

---

## 4. Per-Row Category Override in Step 3

Add two parallel refs initialized when advancing to Step 3:

```ts
rowCategories.value = parsed.value.map((p) => p.category)
manuallyOverridden.value = parsed.value.map(() => false)
```

When the user changes a row's category dropdown, set `manuallyOverridden[i] = true`. When the quick-add rule is saved and rules reload, re-apply category resolution only to rows where `manuallyOverridden[i]` is false.

The preview table gains a `Category` column. Each row renders a compact `USelect` bound to `rowCategories[i]`.

On `confirmImport`, each transaction uses `rowCategories.value[i]` instead of `p.category`.

---

## Files Changed

| File | Change |
|---|---|
| `src-tauri/migrations/0008_category_rules.sql` | New migration + seed defaults |
| `src-tauri/src/models.rs` | Add `CategoryRule` model |
| `src-tauri/src/commands/category_rules.rs` | New file: list/create/delete commands |
| `src-tauri/src/migrations.rs` | Register new migration |
| `src-tauri/src/main.rs` | Register new commands |
| `src/lib/csv/mapping.ts` | Add `autoDetectMapping`, update `applyMapping` signature |
| `src/lib/csv/mapping.test.ts` | Tests for auto-detect and rule application |
| `src/lib/api/categoryRules.ts` | New API file |
| `src/components/ImportWizard.vue` | Auto-detect on load, default category in Step 2, rules quick-add + per-row override in Step 3 |
| `src/pages/Settings.vue` | Category rules management section |
