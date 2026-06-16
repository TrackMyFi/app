# CSV Import Categorization & Auto-Mapping Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add auto-mapping column detection, a layered categorization system (global keyword rules → per-mapping default → uncategorized), and per-row category override to the CSV import wizard.

**Architecture:** A new `category_rules` DB table stores global keyword→category pairs. `applyMapping` gains an optional rules parameter; first matching rule wins, falling back to `MappingConfig.defaultCategory`. A new `autoDetectMapping` function sniffs CSV headers to pre-fill the mapping config. The ImportWizard exposes the default category in Step 2, passes rules to `applyMapping` via a reactive ref, and adds per-row `rowCategories` / `manuallyOverridden` refs in Step 3.

**Tech Stack:** Rust/Tauri (libsql, ts-rs), Vue 3, Vitest, Luxon

---

## File Map

| File | Action | Purpose |
|---|---|---|
| `src-tauri/migrations/0008_category_rules.sql` | Create | Table DDL + seed defaults |
| `src-tauri/src/models.rs` | Modify | Add `CategoryRule` struct |
| `src-tauri/src/migrations.rs` | Modify | Register migration 8 |
| `src-tauri/src/commands/category_rules.rs` | Create | list/create/delete Tauri commands |
| `src-tauri/src/commands/mod.rs` | Modify | Expose new module |
| `src-tauri/src/lib.rs` | Modify | Register 3 new commands |
| `src/lib/types/CategoryRule.ts` | Create | TypeScript type mirroring Rust model |
| `src/lib/api/categoryRules.ts` | Create | invoke() wrappers |
| `src/lib/csv/mapping.ts` | Modify | Add `autoDetectMapping`, update `applyMapping` signature |
| `src/lib/csv/mapping.test.ts` | Modify | Tests for new functions |
| `src/components/ImportWizard.vue` | Modify | Auto-detect, default category, per-row override, quick-add rule |
| `src/pages/Settings.vue` | Modify | Category rules management section |

---

## Task 1: DB Migration + CategoryRule Rust model

**Files:**
- Create: `src-tauri/migrations/0008_category_rules.sql`
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/migrations.rs`

- [ ] **Step 1: Create the migration file**

Create `src-tauri/migrations/0008_category_rules.sql`:

```sql
CREATE TABLE category_rules (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  keyword TEXT NOT NULL,
  category TEXT NOT NULL,
  created_at TEXT NOT NULL
);

INSERT INTO category_rules (keyword, category, created_at) VALUES
  ('netflix', 'fixed', datetime('now')),
  ('spotify', 'fixed', datetime('now')),
  ('hulu', 'fixed', datetime('now')),
  ('disney+', 'fixed', datetime('now')),
  ('apple.com/bill', 'fixed', datetime('now')),
  ('amazon prime', 'fixed', datetime('now')),
  ('youtube premium', 'fixed', datetime('now')),
  ('amazon', 'discretionary', datetime('now')),
  ('walmart', 'discretionary', datetime('now')),
  ('target', 'discretionary', datetime('now')),
  ('starbucks', 'discretionary', datetime('now')),
  ('mcdonald', 'discretionary', datetime('now'));
```

- [ ] **Step 2: Add `CategoryRule` to models.rs**

In `src-tauri/src/models.rs`, append after the `ImportMapping` struct:

```rust
#[derive(Serialize, Deserialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct CategoryRule {
    pub id: i32,
    pub keyword: String,
    pub category: String,
    pub created_at: String,
}
```

- [ ] **Step 3: Register migration 8 in migrations.rs**

In `src-tauri/src/migrations.rs`, append to the `MIGRATIONS` slice after the `date_of_birth` entry:

```rust
    Migration {
        version: 8,
        name: "category_rules",
        sql: include_str!("../migrations/0008_category_rules.sql"),
    },
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/migrations/0008_category_rules.sql src-tauri/src/models.rs src-tauri/src/migrations.rs
git commit -m "feat: add category_rules table migration and Rust model"
```

---

## Task 2: Rust commands for category rules

**Files:**
- Create: `src-tauri/src/commands/category_rules.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create the commands file**

Create `src-tauri/src/commands/category_rules.rs`:

```rust
use crate::db::Db;
use crate::models::CategoryRule;
use libsql::params;
use tauri::State;

fn row_to_rule(row: &libsql::Row) -> Result<CategoryRule, String> {
    Ok(CategoryRule {
        id: row.get(0).map_err(|e| e.to_string())?,
        keyword: row.get(1).map_err(|e| e.to_string())?,
        category: row.get(2).map_err(|e| e.to_string())?,
        created_at: row.get(3).map_err(|e| e.to_string())?,
    })
}

#[tauri::command]
pub async fn list_category_rules_cmd(db: State<'_, Db>) -> Result<Vec<CategoryRule>, String> {
    let conn = db.conn().await?;
    let mut rows = conn
        .query(
            "SELECT id, keyword, category, created_at FROM category_rules ORDER BY keyword",
            (),
        )
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(row_to_rule(&row)?);
    }
    Ok(out)
}

#[tauri::command]
pub async fn create_category_rule_cmd(
    db: State<'_, Db>,
    keyword: String,
    category: String,
    created_at: String,
) -> Result<CategoryRule, String> {
    let conn = db.conn().await?;
    conn.execute(
        "INSERT INTO category_rules (keyword, category, created_at) VALUES (?1, ?2, ?3)",
        params![keyword.clone(), category.clone(), created_at.clone()],
    )
    .await
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid() as i32;
    Ok(CategoryRule { id, keyword, category, created_at })
}

#[tauri::command]
pub async fn delete_category_rule_cmd(db: State<'_, Db>, id: i32) -> Result<(), String> {
    let conn = db.conn().await?;
    conn.execute("DELETE FROM category_rules WHERE id = ?1", params![id])
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
```

- [ ] **Step 2: Expose the module in mod.rs**

In `src-tauri/src/commands/mod.rs`, add:

```rust
pub mod category_rules;
```

- [ ] **Step 3: Register commands in lib.rs**

In `src-tauri/src/lib.rs`, inside the `tauri::generate_handler![]` block, add after the `delete_import_mapping_cmd` line:

```rust
            commands::category_rules::list_category_rules_cmd,
            commands::category_rules::create_category_rule_cmd,
            commands::category_rules::delete_category_rule_cmd,
```

- [ ] **Step 4: Verify compilation**

```bash
cargo build --manifest-path src-tauri/Cargo.toml
```

Expected: exits 0 with no errors. Fix any type or import errors before proceeding.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/category_rules.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat: add category rule Tauri commands"
```

---

## Task 3: TypeScript type + API layer

**Files:**
- Create: `src/lib/types/CategoryRule.ts`
- Create: `src/lib/api/categoryRules.ts`

- [ ] **Step 1: Create the TypeScript type**

Create `src/lib/types/CategoryRule.ts`:

```typescript
export interface CategoryRule {
  id: number
  keyword: string
  category: string
  createdAt: string
}
```

- [ ] **Step 2: Create the API file**

Create `src/lib/api/categoryRules.ts`:

```typescript
import { invoke } from '@tauri-apps/api/core'
import type { CategoryRule } from '../types/CategoryRule'

export const listCategoryRules = () =>
  invoke<CategoryRule[]>('list_category_rules_cmd')

export const createCategoryRule = (keyword: string, category: string, createdAt: string) =>
  invoke<CategoryRule>('create_category_rule_cmd', { keyword, category, createdAt })

export const deleteCategoryRule = (id: number) =>
  invoke<void>('delete_category_rule_cmd', { id })
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/types/CategoryRule.ts src/lib/api/categoryRules.ts
git commit -m "feat: add CategoryRule TypeScript type and API layer"
```

---

## Task 4: `autoDetectMapping` — tests then implementation

**Files:**
- Modify: `src/lib/csv/mapping.test.ts`
- Modify: `src/lib/csv/mapping.ts`

- [ ] **Step 1: Write the failing tests**

In `src/lib/csv/mapping.test.ts`, import `autoDetectMapping` at the top:

```typescript
import { describe, it, expect } from 'vitest'
import { applyMapping, autoDetectMapping, detectDuplicates, parseAmount, type MappingConfig } from './mapping'
```

Then append the new test suite at the bottom of the file:

```typescript
describe('autoDetectMapping', () => {
  it('detects Date/Credit/Debit headers as split mode', () => {
    const result = autoDetectMapping(
      ['Date', 'Description', 'Credit', 'Debit'],
      [{ Date: '03/01/2026', Description: 'Coffee', Credit: '0', Debit: '42.50' }],
    )
    expect(result.dateColumn).toBe('Date')
    expect(result.descriptionColumn).toBe('Description')
    expect(result.amountMode).toBe('split')
    expect(result.creditColumn).toBe('Credit')
    expect(result.debitColumn).toBe('Debit')
  })

  it('detects single Amount column when no credit/debit present', () => {
    const result = autoDetectMapping(
      ['Posting Date', 'Memo', 'Amount'],
      [{ 'Posting Date': '03/01/2026', Memo: 'Coffee', Amount: '-42.50' }],
    )
    expect(result.dateColumn).toBe('Posting Date')
    expect(result.descriptionColumn).toBe('Memo')
    expect(result.amountMode).toBe('single')
    expect(result.amountColumn).toBe('Amount')
  })

  it('auto-detects MM/dd/yyyy date format', () => {
    const result = autoDetectMapping(
      ['Date', 'Amount'],
      [{ Date: '03/15/2026', Amount: '-42.50' }],
    )
    expect(result.dateFormat).toBe('MM/dd/yyyy')
  })

  it('auto-detects yyyy-MM-dd date format', () => {
    const result = autoDetectMapping(
      ['Date', 'Amount'],
      [{ Date: '2026-03-15', Amount: '-42.50' }],
    )
    expect(result.dateFormat).toBe('yyyy-MM-dd')
  })

  it('returns empty object when no headers match', () => {
    const result = autoDetectMapping(['Foo', 'Bar', 'Baz'], [])
    expect(result).toEqual({})
  })

  it('matching is case-insensitive', () => {
    const result = autoDetectMapping(
      ['TRANSACTION DATE', 'DESCRIPTION', 'AMOUNT'],
      [{ 'TRANSACTION DATE': '03/01/2026', DESCRIPTION: 'Coffee', AMOUNT: '-42.50' }],
    )
    expect(result.dateColumn).toBe('TRANSACTION DATE')
    expect(result.descriptionColumn).toBe('DESCRIPTION')
    expect(result.amountColumn).toBe('AMOUNT')
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd /Users/tomgobich/code/trackmyfi && npm test -- --reporter=verbose 2>&1 | tail -20
```

Expected: tests fail with "autoDetectMapping is not a function" or similar. If they pass, something is wrong — do not continue.

- [ ] **Step 3: Implement `autoDetectMapping` in mapping.ts**

In `src/lib/csv/mapping.ts`, add the function after the `MappingConfig` interface (before `parseAmount`):

```typescript
export function autoDetectMapping(
  headers: string[],
  rows: Record<string, string>[],
): Partial<MappingConfig> {
  const find = (aliases: string[]): string =>
    headers.find((h) => {
      const normalized = h.toLowerCase().trim()
      return aliases.some((a) => normalized.includes(a))
    }) ?? ''

  const dateCol = find(['date'])
  const descCol = find(['description', 'memo', 'details', 'narrative', 'payee', 'merchant'])
  const amountCol = find(['amount', 'amt'])
  const creditCol = find(['credit', 'deposit'])
  const debitCol = find(['debit', 'withdrawal', 'charge'])

  const result: Partial<MappingConfig> = {}
  if (dateCol) result.dateColumn = dateCol
  if (descCol) result.descriptionColumn = descCol

  if (creditCol && debitCol) {
    result.amountMode = 'split'
    result.creditColumn = creditCol
    result.debitColumn = debitCol
  } else if (amountCol) {
    result.amountMode = 'single'
    result.amountColumn = amountCol
  }

  if (dateCol) {
    const sample = rows.find((r) => (r[dateCol] ?? '').trim())
    if (sample) {
      const raw = sample[dateCol].trim()
      const formats = ['MM/dd/yyyy', 'yyyy-MM-dd', 'M/d/yyyy', 'dd/MM/yyyy']
      const detected = formats.find((f) => DateTime.fromFormat(raw, f).isValid)
      if (detected) result.dateFormat = detected
    }
  }

  return result
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd /Users/tomgobich/code/trackmyfi && npm test -- --reporter=verbose 2>&1 | tail -20
```

Expected: all autoDetectMapping tests pass. All previously passing tests still pass.

- [ ] **Step 5: Commit**

```bash
git add src/lib/csv/mapping.ts src/lib/csv/mapping.test.ts
git commit -m "feat: add autoDetectMapping with header sniffing and date format detection"
```

---

## Task 5: Update `applyMapping` with optional rules parameter

**Files:**
- Modify: `src/lib/csv/mapping.ts`
- Modify: `src/lib/csv/mapping.test.ts`

- [ ] **Step 1: Write the failing tests**

Append to `src/lib/csv/mapping.test.ts`:

```typescript
describe('applyMapping with category rules', () => {
  it('applies a matching rule to override the default category', () => {
    const result = applyMapping(
      [{ 'Posting Date': '03/01/2026', Amount: '-40.00', Description: 'Netflix monthly' }],
      config,
      false,
      [{ keyword: 'netflix', category: 'fixed' }],
    )
    expect(result[0].category).toBe('fixed')
  })

  it('uses defaultCategory when no rule matches', () => {
    const result = applyMapping(
      [{ 'Posting Date': '03/01/2026', Amount: '-40.00', Description: 'Coffee' }],
      { ...config, defaultCategory: 'discretionary' },
      false,
      [{ keyword: 'netflix', category: 'fixed' }],
    )
    expect(result[0].category).toBe('discretionary')
  })

  it('rule matching is case-insensitive', () => {
    const result = applyMapping(
      [{ 'Posting Date': '03/01/2026', Amount: '-40.00', Description: 'NETFLIX' }],
      config,
      false,
      [{ keyword: 'netflix', category: 'fixed' }],
    )
    expect(result[0].category).toBe('fixed')
  })

  it('first matching rule wins', () => {
    const result = applyMapping(
      [{ 'Posting Date': '03/01/2026', Amount: '-40.00', Description: 'Netflix and Amazon' }],
      config,
      false,
      [
        { keyword: 'netflix', category: 'fixed' },
        { keyword: 'amazon', category: 'discretionary' },
      ],
    )
    expect(result[0].category).toBe('fixed')
  })

  it('omitting rules falls back to defaultCategory (backwards compatible)', () => {
    const result = applyMapping(
      [{ 'Posting Date': '03/01/2026', Amount: '-40.00', Description: 'Coffee' }],
      { ...config, defaultCategory: 'savings' },
    )
    expect(result[0].category).toBe('savings')
  })
})
```

- [ ] **Step 2: Run tests to verify the new ones fail**

```bash
cd /Users/tomgobich/code/trackmyfi && npm test -- --reporter=verbose 2>&1 | tail -20
```

Expected: the 5 new "applyMapping with category rules" tests fail. Existing tests pass.

- [ ] **Step 3: Add `CategoryRuleInput` interface and update `applyMapping` in mapping.ts**

At the top of `src/lib/csv/mapping.ts`, after the `MappingConfig` interface, add:

```typescript
export interface CategoryRuleInput {
  keyword: string
  category: string
}
```

Update the `applyMapping` signature and body:

```typescript
export function applyMapping(
  rows: Record<string, string>[],
  config: MappingConfig,
  isLiabilityAccount = false,
  rules: CategoryRuleInput[] = [],
): ParsedTransaction[] {
  return rows.map((row) => {
    const date = isoDate(row[config.dateColumn] ?? '', config.dateFormat)
    const description = row[config.descriptionColumn] ?? ''

    const descLower = description.toLowerCase()
    const matchedRule = rules.find((r) => descLower.includes(r.keyword.toLowerCase()))
    const category = matchedRule ? matchedRule.category : config.defaultCategory

    if (config.amountMode === 'split') {
      const { amount, type } = resolveSplit(row, config, isLiabilityAccount)
      return { date, amount, description, type, category }
    }

    const signed = parseAmount(row[config.amountColumn] ?? '0')
    const isExpense = config.amountSign === 'negative-is-expense' ? signed < 0 : signed > 0
    return {
      date,
      amount: Math.abs(signed),
      description,
      type: isExpense ? 'expense' : 'income',
      category,
    }
  })
}
```

- [ ] **Step 4: Run all tests**

```bash
cd /Users/tomgobich/code/trackmyfi && npm test -- --reporter=verbose 2>&1 | tail -30
```

Expected: all tests pass, including existing split-mode and single-mode tests (the new `rules` param defaults to `[]` so behaviour is unchanged).

- [ ] **Step 5: Commit**

```bash
git add src/lib/csv/mapping.ts src/lib/csv/mapping.test.ts
git commit -m "feat: applyMapping accepts optional category rules, first match wins"
```

---

## Task 6: Category rules section in Settings page

**Files:**
- Modify: `src/pages/Settings.vue`

- [ ] **Step 1: Add imports to the script block**

In `src/pages/Settings.vue`, add to the existing `import` section at the top of `<script setup>`:

```typescript
import { DateTime } from 'luxon'
import * as categoryRulesApi from '../lib/api/categoryRules'
import type { CategoryRule } from '../lib/types/CategoryRule'
import { categoryItems } from '../lib/transactions/constants'
```

- [ ] **Step 2: Add reactive state for category rules**

In `src/pages/Settings.vue`, after the existing refs (e.g. after `const showDeleteModal = ref(false)`), add:

```typescript
const categoryRules = ref<CategoryRule[]>([])
const newRuleKeyword = ref('')
const newRuleCategory = ref('discretionary')
```

- [ ] **Step 3: Load rules in onMounted**

In `src/pages/Settings.vue`, update `onMounted` to also load rules:

```typescript
onMounted(async () => {
  await store.load()
  if (store.profile) Object.assign(form, store.profile)
  categoryRules.value = await categoryRulesApi.listCategoryRules()
})
```

- [ ] **Step 4: Add rule management functions**

After `runSyncNow`, add:

```typescript
async function addCategoryRule() {
  if (!newRuleKeyword.value.trim()) return
  await categoryRulesApi.createCategoryRule(
    newRuleKeyword.value.trim().toLowerCase(),
    newRuleCategory.value,
    DateTime.now().toISO()!,
  )
  categoryRules.value = await categoryRulesApi.listCategoryRules()
  newRuleKeyword.value = ''
  newRuleCategory.value = 'discretionary'
}

async function removeCategoryRule(id: number) {
  await categoryRulesApi.deleteCategoryRule(id)
  categoryRules.value = await categoryRulesApi.listCategoryRules()
}
```

- [ ] **Step 5: Add the Category Rules section to the template**

In `src/pages/Settings.vue`, add a new `<section>` before the Danger Zone `<section>` (before `<hr class="border-default" />` that precedes Danger Zone):

```html
<hr class="border-default" />

<section class="space-y-3">
  <h2 class="text-xl font-bold">Category Rules</h2>
  <p class="text-sm text-muted">
    Keywords matched against transaction descriptions during CSV import.
    First matching rule wins; unmatched rows use the mapping's default category.
  </p>

  <table v-if="categoryRules.length" class="w-full text-sm">
    <thead class="text-left text-muted border-b border-default">
      <tr>
        <th class="pb-1">Keyword</th>
        <th class="pb-1">Category</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="rule in categoryRules" :key="rule.id" class="border-b border-default/50">
        <td class="py-1.5 font-mono text-xs">{{ rule.keyword }}</td>
        <td class="py-1.5">{{ rule.category }}</td>
        <td class="py-1.5 text-right">
          <UButton size="xs" color="error" variant="ghost" @click="removeCategoryRule(rule.id)">
            Remove
          </UButton>
        </td>
      </tr>
    </tbody>
  </table>
  <p v-else class="text-sm text-muted">No rules yet.</p>

  <div class="flex gap-2 items-center pt-1">
    <UInput
      v-model="newRuleKeyword"
      placeholder="keyword (e.g. netflix)"
      class="flex-1"
      @keydown.enter="addCategoryRule"
    />
    <USelect v-model="newRuleCategory" :items="categoryItems" class="w-44" />
    <UButton size="sm" variant="soft" :disabled="!newRuleKeyword.trim()" @click="addCategoryRule">
      Add rule
    </UButton>
  </div>
</section>
```

- [ ] **Step 6: Commit**

```bash
git add src/pages/Settings.vue
git commit -m "feat: add category rules management section to Settings page"
```

---

## Task 7: ImportWizard — auto-detect on load + default category in Step 2

**Files:**
- Modify: `src/components/ImportWizard.vue`

- [ ] **Step 1: Update script imports**

At the top of the `<script setup>` block in `src/components/ImportWizard.vue`, add the new imports:

```typescript
import { computed, onMounted, ref, watch } from 'vue'
import { DateTime } from 'luxon'
import { parseCsv } from '../lib/csv/parse'
import { applyMapping, autoDetectMapping, detectDuplicates, parseAmount, type MappingConfig } from '../lib/csv/mapping'
import { bulkCreateTransactions } from '../lib/api/transactions'
import * as mappingApi from '../lib/api/importMappings'
import * as categoryRulesApi from '../lib/api/categoryRules'
import { useAccountsStore } from '../stores/accounts'
import { useTransactionsStore } from '../stores/transactions'
import { isLiability } from '../lib/accountTypes'
import { categoryItems } from '../lib/transactions/constants'
import type { ImportMapping } from '../lib/types/ImportMapping'
import type { CategoryRule } from '../lib/types/CategoryRule'
```

- [ ] **Step 2: Add `categoryRules` ref**

After the `savedMappings` ref, add:

```typescript
const categoryRules = ref<CategoryRule[]>([])
```

- [ ] **Step 3: Load rules in onMounted**

Update `onMounted`:

```typescript
onMounted(async () => {
  await accountsStore.load()
  savedMappings.value = await mappingApi.listImportMappings()
  categoryRules.value = await categoryRulesApi.listCategoryRules()
})
```

- [ ] **Step 4: Update `parsed` computed to pass rules**

Replace the existing `parsed` computed:

```typescript
const parsed = computed(() =>
  step.value === 3
    ? applyMapping(rawRows.value, config.value, isLiabilityAccount.value, categoryRules.value)
    : [],
)
```

- [ ] **Step 5: Call `autoDetectMapping` in `onFile`**

Replace the existing `onFile` function:

```typescript
async function onFile(event: Event) {
  const file = (event.target as HTMLInputElement).files?.[0]
  if (!file) return
  const text = await file.text()
  const result = parseCsv(text)
  headers.value = result.headers
  rawRows.value = result.rows
  const detected = autoDetectMapping(result.headers, result.rows)
  config.value = { ...config.value, ...detected }
  step.value = 2
}
```

- [ ] **Step 6: Add default category picker to Step 2 template**

In the template, find the `<!-- FORMAT -->` section in Step 2. Add a new `<!-- CATEGORY DEFAULTS -->` section after `<!-- FORMAT -->` and before `<!-- SAVE MAPPING -->`:

```html
      <!-- CATEGORY DEFAULTS -->
      <div class="space-y-3">
        <p class="text-xs font-semibold uppercase tracking-wide text-muted">Category Defaults</p>
        <div>
          <p class="text-xs text-muted mb-1">Default category for unmatched rows</p>
          <USelect v-model="config.defaultCategory" :items="categoryItems" class="w-full" />
        </div>
      </div>
```

- [ ] **Step 7: Commit**

```bash
git add src/components/ImportWizard.vue
git commit -m "feat: auto-detect CSV column mapping and add default category picker in Step 2"
```

---

## Task 8: ImportWizard — per-row category override + quick-add rule in Step 3

**Files:**
- Modify: `src/components/ImportWizard.vue`

- [ ] **Step 1: Add `rowCategories` and `manuallyOverridden` refs**

After the `include` ref, add:

```typescript
const rowCategories = ref<string[]>([])
const manuallyOverridden = ref<boolean[]>([])
const newRuleKeyword = ref('')
const newRuleCategory = ref('discretionary')
```

- [ ] **Step 2: Update `goToPreview` to initialize the new refs**

Replace the existing `goToPreview` function:

```typescript
function goToPreview() {
  step.value = 3
  include.value = parsed.value.map((_, i) => !dupes.value[i])
  rowCategories.value = parsed.value.map((p) => p.category)
  manuallyOverridden.value = parsed.value.map(() => false)
}
```

- [ ] **Step 3: Add watcher to re-apply rules when `parsed` changes in Step 3**

After `goToPreview`, add:

```typescript
watch(parsed, (newParsed) => {
  if (step.value !== 3) return
  rowCategories.value = newParsed.map((p, i) =>
    manuallyOverridden.value[i] ? rowCategories.value[i] : p.category,
  )
})
```

- [ ] **Step 4: Add the quick-add rule save function**

After the `watch` block, add:

```typescript
async function saveQuickRule() {
  if (!newRuleKeyword.value.trim()) return
  await categoryRulesApi.createCategoryRule(
    newRuleKeyword.value.trim().toLowerCase(),
    newRuleCategory.value,
    DateTime.now().toISO()!,
  )
  categoryRules.value = await categoryRulesApi.listCategoryRules()
  newRuleKeyword.value = ''
  newRuleCategory.value = 'discretionary'
}
```

- [ ] **Step 5: Update `confirmImport` to use `rowCategories`**

Replace the existing `confirmImport` function. Note: preserve the original index `i` through the filter so `rowCategories[i]` refers to the correct row.

```typescript
async function confirmImport() {
  if (accountId.value == null) return
  const now = DateTime.now().toISO()!
  const toInsert = parsed.value
    .map((p, i) => ({ p, i }))
    .filter(({ i }) => include.value[i])
    .map(({ p, i }) => ({
      accountId: accountId.value!,
      transferAccountId: null,
      amount: p.amount,
      description: p.description,
      date: p.date,
      type: p.type,
      category: rowCategories.value[i] ?? p.category,
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

- [ ] **Step 6: Add Category column and per-row select to the Step 3 preview table**

In the Step 3 template, replace the `<table>` block:

```html
      <table class="w-full text-sm">
        <thead class="text-left text-muted border-b border-default">
          <tr>
            <th></th>
            <th>Date</th>
            <th>Description</th>
            <th>Type</th>
            <th>Category</th>
            <th class="text-right">Amount</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(p, i) in parsed" :key="i" class="border-b border-default/50"
            :class="{ 'opacity-50': dupes[i] }">
            <td><UCheckbox v-model="include[i]" /></td>
            <td>{{ p.date }}</td>
            <td>{{ p.description }} <span v-if="dupes[i]" class="text-xs text-amber-600">(dup)</span></td>
            <td>{{ p.type }}</td>
            <td>
              <USelect
                v-model="rowCategories[i]"
                :items="categoryItems"
                size="xs"
                class="w-36"
                @update:model-value="manuallyOverridden[i] = true"
              />
            </td>
            <td class="text-right tabular-nums">{{ p.amount }}</td>
          </tr>
        </tbody>
      </table>
```

- [ ] **Step 7: Add the quick-add rule row below the table**

After the closing `</table>` tag and before the import button `<div>`, add:

```html
      <div class="flex gap-2 items-center pt-1 border-t border-default">
        <p class="text-xs text-muted shrink-0">Add rule:</p>
        <UInput
          v-model="newRuleKeyword"
          placeholder="keyword"
          size="xs"
          class="flex-1"
          @keydown.enter="saveQuickRule"
        />
        <USelect v-model="newRuleCategory" :items="categoryItems" size="xs" class="w-36" />
        <UButton size="xs" variant="soft" :disabled="!newRuleKeyword.trim()" @click="saveQuickRule">
          Save rule
        </UButton>
      </div>
```

- [ ] **Step 8: Commit**

```bash
git add src/components/ImportWizard.vue
git commit -m "feat: per-row category override and quick-add rule in import wizard Step 3"
```

---

## Task 9: Final verification

- [ ] **Step 1: Run full test suite**

```bash
cd /Users/tomgobich/code/trackmyfi && npm test 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 2: Build Rust to confirm no compilation errors**

```bash
cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -10
```

Expected: exits 0.
