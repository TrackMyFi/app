# Accounts Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the monolithic Accounts page (loads all snapshots upfront) with a grouped list page + per-account detail page that lazy-loads snapshot history by month from the backend.

**Architecture:** Three new Rust commands handle efficient data fetching (latest balance per account, month summaries per account, snapshots for a single month). The frontend gets a new `/accounts/:id` route backed by `AccountDetail.vue`, while `Accounts.vue` is rewritten as a clean grouped table with stats. The store gains a `loadList()` fast path for the list page that avoids loading all historical balances.

**Tech Stack:** Rust/libsql (backend commands), Tauri invoke, Pinia (accounts store), Vue 3 Composition API, Nuxt UI v4 (UDropdownMenu, UModal, UAccordion), @unovis/vue (charts), vue-router, luxon

**Spec:** `docs/superpowers/specs/2026-06-19-accounts-redesign.md`

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `src-tauri/src/models.rs` | Modify | Add `BalanceMonthSummary` struct |
| `src-tauri/src/commands/accounts.rs` | Modify | Add 3 new inner fns + command wrappers |
| `src-tauri/src/lib.rs` | Modify | Register 3 new Tauri commands |
| `src/lib/types/BalanceMonthSummary.ts` | Create | TS type for month summary |
| `src/lib/api/accounts.ts` | Modify | Add 3 new API invoke functions |
| `src/stores/accounts.ts` | Modify | Add `latestBalances` ref + `loadList()` action |
| `src/router.ts` | Modify | Add `/accounts/:id` route |
| `src/pages/Accounts.vue` | Rewrite | Stats + grouped tables + ⋯ context menus |
| `src/components/AccountBalanceChart.vue` | Create | Context-aware line chart (monthly / intra-month) |
| `src/pages/AccountDetail.vue` | Create | Detail page: header, chart, lazy accordion |

---

## Task 1: Add `BalanceMonthSummary` Rust model

**Files:**
- Modify: `src-tauri/src/models.rs`

- [ ] **Step 1: Add the struct after `AccountBalance` in models.rs**

Open `src-tauri/src/models.rs` and add this block immediately after the `AccountBalance` struct (around line 53):

```rust
#[derive(Serialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct BalanceMonthSummary {
    pub month: String,        // "YYYY-MM"
    pub count: i64,
    pub latest_balance: f64,
}
```

Note: no `Deserialize` needed — this type is only ever returned from Rust, never sent in.

- [ ] **Step 2: Verify it compiles**

```bash
cd src-tauri && cargo check 2>&1 | grep -E "error|warning.*unused"
```

Expected: no errors. Warnings about unused struct are fine at this stage.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/models.rs
git commit -m "feat: add BalanceMonthSummary Rust model"
```

---

## Task 2: Add `list_latest_balances` Rust command

**Files:**
- Modify: `src-tauri/src/commands/accounts.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add the inner function to accounts.rs**

Open `src-tauri/src/commands/accounts.rs`. Add this import at the top if not already present — `models.rs` already exports `AccountBalance`, so the existing `use crate::models::{Account, AccountBalance};` line just needs to be left as-is.

Add this function after `list_all_balances` (around line 197):

```rust
pub async fn list_latest_balances(conn: &Connection) -> Result<Vec<AccountBalance>, String> {
    let mut rows = conn
        .query(
            "SELECT b.id, b.account_id, b.balance, b.recorded_at, t.id \
             FROM account_balance b \
             INNER JOIN ( \
               SELECT account_id, MAX(recorded_at) AS max_date \
               FROM account_balance \
               GROUP BY account_id \
             ) latest ON b.account_id = latest.account_id AND b.recorded_at = latest.max_date \
             LEFT JOIN txn t \
               ON t.generated_balance_id = b.id OR t.generated_balance_to_id = b.id",
            (),
        )
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(row_to_balance(&row)?);
    }
    Ok(out)
}
```

- [ ] **Step 2: Add the Tauri command wrapper**

At the bottom of `src-tauri/src/commands/accounts.rs`, add:

```rust
#[tauri::command]
pub async fn list_latest_balances_cmd(db: State<'_, Db>) -> Result<Vec<AccountBalance>, String> {
    let conn = db.conn().await?;
    list_latest_balances(&conn).await
}
```

- [ ] **Step 3: Register the command in lib.rs**

Open `src-tauri/src/lib.rs`. Inside the `invoke_handler!` macro list (after `commands::accounts::list_all_balances_cmd,`), add:

```rust
commands::accounts::list_latest_balances_cmd,
```

- [ ] **Step 4: Verify it compiles**

```bash
cd src-tauri && cargo check 2>&1 | grep "error"
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/accounts.rs src-tauri/src/lib.rs
git commit -m "feat: add list_latest_balances_cmd Rust command"
```

---

## Task 3: Add `list_balance_month_summaries` Rust command

**Files:**
- Modify: `src-tauri/src/commands/accounts.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add the import for BalanceMonthSummary**

In `src-tauri/src/commands/accounts.rs`, update the models import at the top:

```rust
use crate::models::{Account, AccountBalance, BalanceMonthSummary};
```

- [ ] **Step 2: Add a row mapper helper**

Add this helper function after `row_to_balance` (around line 54):

```rust
fn row_to_month_summary(row: &libsql::Row) -> Result<BalanceMonthSummary, String> {
    Ok(BalanceMonthSummary {
        month: row.get(0).map_err(|e| e.to_string())?,
        count: row.get(1).map_err(|e| e.to_string())?,
        latest_balance: row.get(2).map_err(|e| e.to_string())?,
    })
}
```

- [ ] **Step 3: Add the inner function**

Add after `list_latest_balances`:

```rust
pub async fn list_balance_month_summaries(
    conn: &Connection,
    account_id: i32,
) -> Result<Vec<BalanceMonthSummary>, String> {
    let mut rows = conn
        .query(
            "SELECT \
               strftime('%Y-%m', recorded_at) AS month, \
               COUNT(*) AS count, \
               (SELECT balance FROM account_balance b2 \
                WHERE b2.account_id = ?1 \
                  AND strftime('%Y-%m', b2.recorded_at) = strftime('%Y-%m', account_balance.recorded_at) \
                ORDER BY b2.recorded_at DESC LIMIT 1) AS latest_balance \
             FROM account_balance \
             WHERE account_id = ?1 \
             GROUP BY month \
             ORDER BY month DESC",
            libsql::params![account_id],
        )
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(row_to_month_summary(&row)?);
    }
    Ok(out)
}
```

- [ ] **Step 4: Add the command wrapper**

```rust
#[tauri::command]
pub async fn list_balance_month_summaries_cmd(
    db: State<'_, Db>,
    account_id: i32,
) -> Result<Vec<BalanceMonthSummary>, String> {
    let conn = db.conn().await?;
    list_balance_month_summaries(&conn, account_id).await
}
```

- [ ] **Step 5: Register in lib.rs**

In the `invoke_handler!` list, add after `list_latest_balances_cmd,`:

```rust
commands::accounts::list_balance_month_summaries_cmd,
```

- [ ] **Step 6: Verify**

```bash
cd src-tauri && cargo check 2>&1 | grep "error"
```

Expected: no errors.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands/accounts.rs src-tauri/src/lib.rs
git commit -m "feat: add list_balance_month_summaries_cmd Rust command"
```

---

## Task 4: Add `list_balances_for_month` Rust command

**Files:**
- Modify: `src-tauri/src/commands/accounts.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add the inner function**

Add after `list_balance_month_summaries`:

```rust
pub async fn list_balances_for_month(
    conn: &Connection,
    account_id: i32,
    month: &str,
) -> Result<Vec<AccountBalance>, String> {
    let mut rows = conn
        .query(
            "SELECT b.id, b.account_id, b.balance, b.recorded_at, t.id \
             FROM account_balance b \
             LEFT JOIN txn t \
               ON t.generated_balance_id = b.id OR t.generated_balance_to_id = b.id \
             WHERE b.account_id = ?1 \
               AND strftime('%Y-%m', b.recorded_at) = ?2 \
             ORDER BY b.recorded_at DESC",
            libsql::params![account_id, month.to_string()],
        )
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(row_to_balance(&row)?);
    }
    Ok(out)
}
```

- [ ] **Step 2: Add the command wrapper**

```rust
#[tauri::command]
pub async fn list_balances_for_month_cmd(
    db: State<'_, Db>,
    account_id: i32,
    month: String,
) -> Result<Vec<AccountBalance>, String> {
    let conn = db.conn().await?;
    list_balances_for_month(&conn, account_id, &month).await
}
```

- [ ] **Step 3: Register in lib.rs**

In the `invoke_handler!` list, add after `list_balance_month_summaries_cmd,`:

```rust
commands::accounts::list_balances_for_month_cmd,
```

- [ ] **Step 4: Verify**

```bash
cd src-tauri && cargo check 2>&1 | grep "error"
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/accounts.rs src-tauri/src/lib.rs
git commit -m "feat: add list_balances_for_month_cmd Rust command"
```

---

## Task 5: Add TS type and API functions

**Files:**
- Create: `src/lib/types/BalanceMonthSummary.ts`
- Modify: `src/lib/api/accounts.ts`

- [ ] **Step 1: Create the TS type**

Create `src/lib/types/BalanceMonthSummary.ts`:

```typescript
export type BalanceMonthSummary = {
  month: string         // "YYYY-MM"
  count: number
  latestBalance: number
}
```

Note: `ts-rs` would normally generate this, but since `BalanceMonthSummary` uses only primitive fields and `ts-rs` generates during the Rust build step, we write it manually here to avoid a full Tauri build dependency. Keep the field names matching the `#[serde(rename_all = "camelCase")]` output.

- [ ] **Step 2: Add the import and three API functions to accounts.ts**

Open `src/lib/api/accounts.ts`. Add the type import with the other imports at the top of the file:

```typescript
import type { BalanceMonthSummary } from '../types/BalanceMonthSummary'
```

Then add the three export functions at the bottom of the file:

```typescript
export const listLatestBalances = () =>
  invoke<AccountBalance[]>('list_latest_balances_cmd')

export const listBalanceMonthSummaries = (accountId: number) =>
  invoke<BalanceMonthSummary[]>('list_balance_month_summaries_cmd', { accountId })

export const listBalancesForMonth = (accountId: number, month: string) =>
  invoke<AccountBalance[]>('list_balances_for_month_cmd', { accountId, month })
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/types/BalanceMonthSummary.ts src/lib/api/accounts.ts
git commit -m "feat: add BalanceMonthSummary TS type and API functions"
```

---

## Task 6: Update the accounts store

**Files:**
- Modify: `src/stores/accounts.ts`

The store needs a fast `loadList()` path (accounts + latest balances only) for the list page, while keeping `load()` (accounts + all balances) intact for the Dashboard.

- [ ] **Step 1: Rewrite stores/accounts.ts**

Replace the entire file with:

```typescript
import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Account } from '../lib/types/Account'
import type { AccountBalance } from '../lib/types/AccountBalance'
import * as api from '../lib/api/accounts'

export const useAccountsStore = defineStore('accounts', () => {
  const accounts = ref<Account[]>([])
  const allBalances = ref<AccountBalance[]>([])
  const latestBalances = ref<AccountBalance[]>([])

  async function load() {
    accounts.value = await api.listAccounts()
    allBalances.value = await api.listAllBalances()
  }

  async function loadList() {
    accounts.value = await api.listAccounts()
    latestBalances.value = await api.listLatestBalances()
  }

  async function create(a: Parameters<typeof api.createAccount>[0]) { await api.createAccount(a); await loadList() }
  async function archive(id: number) { await api.archiveAccount(id); await loadList() }
  async function unarchive(id: number) { await api.unarchiveAccount(id); await loadList() }
  async function remove(id: number) { await api.deleteAccount(id); await loadList() }
  async function update(id: number, a: Parameters<typeof api.updateAccount>[1]) { await api.updateAccount(id, a); await loadList() }
  async function addBalanceSnapshot(b: Parameters<typeof api.addBalance>[0]) { await api.addBalance(b); await load() }
  async function updateBalanceSnapshot(b: Parameters<typeof api.updateBalance>[0]) { await api.updateBalance(b); await load() }
  async function removeBalanceSnapshot(id: number) { await api.deleteBalance(id); await load() }

  return {
    accounts, allBalances, latestBalances,
    load, loadList,
    create, update, archive, unarchive, remove,
    addBalanceSnapshot, updateBalanceSnapshot, removeBalanceSnapshot,
  }
})
```

Note: `addBalanceSnapshot`, `updateBalanceSnapshot`, and `removeBalanceSnapshot` still call `load()` (fetching `allBalances`) because Dashboard's net-worth-over-time series requires the full history. These actions are only called from `AccountDetail.vue` indirectly — but the detail page will call the API directly and refresh its own local state instead (see Task 8), so this is a no-op in practice for the detail page.

- [ ] **Step 2: Commit**

```bash
git add src/stores/accounts.ts
git commit -m "feat: add loadList() fast path and latestBalances to accounts store"
```

---

## Task 7: Add the account detail route

**Files:**
- Modify: `src/router.ts`

- [ ] **Step 1: Add the route**

Open `src/router.ts`. After the `accounts` route, add:

```typescript
{ path: '/accounts/:id', name: 'account-detail', component: () => import('./pages/AccountDetail.vue') },
```

The full routes array should look like:

```typescript
const routes = [
  { path: '/onboarding', name: 'onboarding', component: () => import('./pages/Onboarding.vue') },
  { path: '/', name: 'dashboard', component: () => import('./pages/Dashboard.vue') },
  { path: '/accounts', name: 'accounts', component: () => import('./pages/Accounts.vue') },
  { path: '/accounts/:id', name: 'account-detail', component: () => import('./pages/AccountDetail.vue') },
  { path: '/transactions', name: 'transactions', component: () => import('./pages/Transactions.vue') },
  { path: '/paychecks', name: 'paychecks', component: () => import('./pages/Paychecks.vue') },
  { path: '/contributions', name: 'contributions', component: () => import('./pages/Contributions.vue') },
  { path: '/budget', name: 'budget', component: () => import('./pages/Budget.vue') },
  { path: '/forecast', name: 'forecast', component: () => import('./pages/Forecast.vue') },
  { path: '/settings', name: 'settings', component: () => import('./pages/Settings.vue') },
]
```

- [ ] **Step 2: Commit**

```bash
git add src/router.ts
git commit -m "feat: add /accounts/:id route"
```

---

## Task 8: Rewrite Accounts.vue (list page)

**Files:**
- Rewrite: `src/pages/Accounts.vue`

The new list page: 3 stat cards, FIRE/non-FIRE account tables, ⋯ dropdown menus, collapsed archived section.

- [ ] **Step 1: Replace Accounts.vue**

Replace the entire contents of `src/pages/Accounts.vue` with:

```vue
<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useAccountsStore } from '../stores/accounts'
import { labelForAccountType } from '../lib/accountTypes'
import AccountForm from '../components/AccountForm.vue'
import StatCard from '../components/StatCard.vue'
import type { Account } from '../lib/types/Account'
import { confirm } from '@tauri-apps/plugin-dialog'

const store = useAccountsStore()
const router = useRouter()

onMounted(() => store.loadList())

const fmt = (n: number) =>
  n.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })

const latestBalanceMap = computed(() =>
  new Map(store.latestBalances.map(b => [b.accountId, b.balance]))
)

const activeAccounts = computed(() => store.accounts.filter(a => a.isActive))
const archivedAccounts = computed(() => store.accounts.filter(a => !a.isActive))
const fireAccounts = computed(() => activeAccounts.value.filter(a => a.includeInFireCalculations))
const nonFireAccounts = computed(() => activeAccounts.value.filter(a => !a.includeInFireCalculations))

const netWorth = computed(() =>
  activeAccounts.value.reduce((s, a) => s + (latestBalanceMap.value.get(a.id) ?? 0), 0)
)
const fireTotal = computed(() =>
  fireAccounts.value.reduce((s, a) => s + (latestBalanceMap.value.get(a.id) ?? 0), 0)
)
const nonFireTotal = computed(() =>
  nonFireAccounts.value.reduce((s, a) => s + (latestBalanceMap.value.get(a.id) ?? 0), 0)
)

function latestBalance(accountId: number) {
  const b = latestBalanceMap.value.get(accountId)
  return b != null ? b.toLocaleString('en-US', { style: 'currency', currency: 'USD' }) : '—'
}

function navigate(account: Account) {
  router.push({ name: 'account-detail', params: { id: account.id } })
}

// Add / Edit account modal
const isAccountModalOpen = ref(false)
const editingAccount = ref<Account | null>(null)

function openAdd() {
  editingAccount.value = null
  isAccountModalOpen.value = true
}

function openEdit(account: Account) {
  editingAccount.value = account
  isAccountModalOpen.value = true
}

watch(isAccountModalOpen, open => { if (!open) editingAccount.value = null })

function onAccountSaved() {
  isAccountModalOpen.value = false
}

// Archive / Unarchive
async function archive(id: number) {
  await store.archive(id)
}

async function unarchive(id: number) {
  await store.unarchive(id)
}

// Delete (archived accounts only)
async function remove(account: Account) {
  const ok = await confirm(
    `Permanently delete "${account.name}" and all of its balance snapshots? This cannot be undone.`,
    { title: 'Delete Account?', kind: 'warning' },
  )
  if (ok) await store.remove(account.id)
}

// Archived section toggle
const showArchived = ref(false)

// Dropdown menu items per account
function activeMenuItems(account: Account) {
  return [[
    { label: 'Edit', icon: 'i-ph-pencil', onSelect: () => openEdit(account) },
    { label: 'Archive', icon: 'i-ph-archive', onSelect: () => archive(account.id) },
  ]]
}

function archivedMenuItems(account: Account) {
  return [[
    { label: 'Restore', icon: 'i-ph-arrow-counter-clockwise', onSelect: () => unarchive(account.id) },
    { label: 'Delete', icon: 'i-ph-trash', color: 'error' as const, onSelect: () => remove(account) },
  ]]
}
</script>

<template>
  <div class="p-6 max-w-4xl">
    <!-- Header -->
    <div class="flex items-center justify-between mb-6">
      <h1 class="text-2xl font-bold">Accounts</h1>
      <UButton icon="i-ph-plus" @click="openAdd">Add Account</UButton>
    </div>

    <!-- Stats -->
    <div class="grid grid-cols-3 gap-4 mb-8">
      <StatCard label="Net Worth" :value="fmt(netWorth)" />
      <StatCard label="FIRE Accounts" :value="fmt(fireTotal)" />
      <StatCard label="Non-FIRE Accounts" :value="fmt(nonFireTotal)" />
    </div>

    <!-- FIRE Accounts -->
    <div v-if="fireAccounts.length > 0" class="mb-8">
      <p class="text-xs font-semibold uppercase tracking-widest text-muted mb-2">FIRE Accounts</p>
      <div class="border border-default rounded-lg overflow-hidden">
        <div class="grid grid-cols-[1fr_160px_140px_36px] bg-elevated px-4 py-2 border-b border-default">
          <span class="text-xs font-semibold uppercase tracking-wider text-muted">Account</span>
          <span class="text-xs font-semibold uppercase tracking-wider text-muted">Type</span>
          <span class="text-xs font-semibold uppercase tracking-wider text-muted text-right">Balance</span>
          <span />
        </div>
        <div
          v-for="account in fireAccounts"
          :key="account.id"
          class="grid grid-cols-[1fr_160px_140px_36px] items-center px-4 py-3 border-b border-default last:border-b-0 cursor-pointer hover:bg-elevated/50 transition-colors"
          @click="navigate(account)"
        >
          <div>
            <p class="text-sm font-medium">{{ account.name }}</p>
            <p v-if="account.institution" class="text-xs text-muted">{{ account.institution }}</p>
          </div>
          <span class="text-sm text-muted">{{ labelForAccountType(account.type) }}</span>
          <span class="text-sm font-semibold text-right font-mono">{{ latestBalance(account.id) }}</span>
          <div class="flex justify-center" @click.stop>
            <UDropdownMenu :items="activeMenuItems(account)">
              <UButton size="xs" variant="ghost" icon="i-ph-dots-three" color="neutral" />
            </UDropdownMenu>
          </div>
        </div>
      </div>
    </div>

    <!-- Non-FIRE Accounts -->
    <div v-if="nonFireAccounts.length > 0" class="mb-8">
      <p class="text-xs font-semibold uppercase tracking-widest text-muted mb-2">Non-FIRE Accounts</p>
      <div class="border border-default rounded-lg overflow-hidden">
        <div class="grid grid-cols-[1fr_160px_140px_36px] bg-elevated px-4 py-2 border-b border-default">
          <span class="text-xs font-semibold uppercase tracking-wider text-muted">Account</span>
          <span class="text-xs font-semibold uppercase tracking-wider text-muted">Type</span>
          <span class="text-xs font-semibold uppercase tracking-wider text-muted text-right">Balance</span>
          <span />
        </div>
        <div
          v-for="account in nonFireAccounts"
          :key="account.id"
          class="grid grid-cols-[1fr_160px_140px_36px] items-center px-4 py-3 border-b border-default last:border-b-0 cursor-pointer hover:bg-elevated/50 transition-colors"
          @click="navigate(account)"
        >
          <div>
            <p class="text-sm font-medium">{{ account.name }}</p>
            <p v-if="account.institution" class="text-xs text-muted">{{ account.institution }}</p>
          </div>
          <span class="text-sm text-muted">{{ labelForAccountType(account.type) }}</span>
          <span class="text-sm font-semibold text-right font-mono">{{ latestBalance(account.id) }}</span>
          <div class="flex justify-center" @click.stop>
            <UDropdownMenu :items="activeMenuItems(account)">
              <UButton size="xs" variant="ghost" icon="i-ph-dots-three" color="neutral" />
            </UDropdownMenu>
          </div>
        </div>
      </div>
    </div>

    <!-- Archived -->
    <div v-if="archivedAccounts.length > 0">
      <button
        class="flex items-center gap-1.5 text-xs font-semibold uppercase tracking-widest text-muted mb-2"
        @click="showArchived = !showArchived"
      >
        <span>{{ showArchived ? '▼' : '▶' }}</span>
        <span>Archived ({{ archivedAccounts.length }})</span>
      </button>
      <div v-if="showArchived" class="border border-default rounded-lg overflow-hidden">
        <div class="grid grid-cols-[1fr_160px_140px_36px] bg-elevated px-4 py-2 border-b border-default">
          <span class="text-xs font-semibold uppercase tracking-wider text-muted">Account</span>
          <span class="text-xs font-semibold uppercase tracking-wider text-muted">Type</span>
          <span class="text-xs font-semibold uppercase tracking-wider text-muted text-right">Balance</span>
          <span />
        </div>
        <div
          v-for="account in archivedAccounts"
          :key="account.id"
          class="grid grid-cols-[1fr_160px_140px_36px] items-center px-4 py-3 border-b border-default last:border-b-0"
        >
          <div>
            <p class="text-sm font-medium text-muted">{{ account.name }}</p>
            <p v-if="account.institution" class="text-xs text-muted">{{ account.institution }}</p>
          </div>
          <span class="text-sm text-muted">{{ labelForAccountType(account.type) }}</span>
          <span class="text-sm text-muted text-right font-mono">{{ latestBalance(account.id) }}</span>
          <div class="flex justify-center">
            <UDropdownMenu :items="archivedMenuItems(account)">
              <UButton size="xs" variant="ghost" icon="i-ph-dots-three" color="neutral" />
            </UDropdownMenu>
          </div>
        </div>
      </div>
    </div>

    <!-- Add / Edit account modal -->
    <UModal
      v-model:open="isAccountModalOpen"
      :title="editingAccount ? 'Edit Account' : 'Add Account'"
      class="w-112"
    >
      <template #body>
        <AccountForm
          :key="editingAccount?.id ?? 'new'"
          :account="editingAccount ?? undefined"
          @saved="onAccountSaved"
        />
      </template>
    </UModal>
  </div>
</template>
```

- [ ] **Step 2: Commit**

```bash
git add src/pages/Accounts.vue
git commit -m "feat: rewrite Accounts.vue with stats, grouped tables, and context menus"
```

---

## Task 9: Create AccountBalanceChart component

**Files:**
- Create: `src/components/AccountBalanceChart.vue`

A thin wrapper around unovis that accepts normalized `{ date, balance }` points and a mode to format tick labels.

- [ ] **Step 1: Create the component**

Create `src/components/AccountBalanceChart.vue`:

```vue
<script setup lang="ts">
import { VisXYContainer, VisLine, VisAxis } from '@unovis/vue'
import { DateTime } from 'luxon'

export type ChartPoint = { date: string; balance: number }

const props = defineProps<{
  points: ChartPoint[]
  mode: 'monthly' | 'intramonth'
}>()

type D = { t: number; v: number }

const data = (): D[] =>
  props.points.map(p => ({ t: DateTime.fromISO(p.date).toMillis(), v: p.balance }))

const x = (d: D) => d.t
const y = (d: D) => d.v

const tickFormatX = (t: number | Date) => {
  const ms = typeof t === 'number' ? t : t.getTime()
  return props.mode === 'monthly'
    ? DateTime.fromMillis(ms).toFormat('LLL yyyy')
    : DateTime.fromMillis(ms).toFormat('MMM d')
}
</script>

<template>
  <VisXYContainer :data="data()" :height="200">
    <VisLine :x="x" :y="y" />
    <VisAxis type="x" :tick-format="tickFormatX" />
    <VisAxis type="y" />
  </VisXYContainer>
</template>
```

- [ ] **Step 2: Commit**

```bash
git add src/components/AccountBalanceChart.vue
git commit -m "feat: add AccountBalanceChart component with monthly/intramonth modes"
```

---

## Task 10: Create AccountDetail.vue

**Files:**
- Create: `src/pages/AccountDetail.vue`

The detail page: account header with Edit/Archive actions, context-aware chart, lazy-loaded month accordion, inline snapshot editing, delete account.

- [ ] **Step 1: Create the page**

Create `src/pages/AccountDetail.vue`:

```vue
<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useToast } from '@nuxt/ui/composables'
import { useAccountsStore } from '../stores/accounts'
import { labelForAccountType } from '../lib/accountTypes'
import AccountForm from '../components/AccountForm.vue'
import AccountBalanceChart from '../components/AccountBalanceChart.vue'
import TransactionDetail from '../components/TransactionDetail.vue'
import DateInput from '../components/DateInput.vue'
import CurrencyInput from '../components/CurrencyInput.vue'
import type { AccountBalance } from '../lib/types/AccountBalance'
import type { BalanceMonthSummary } from '../lib/types/BalanceMonthSummary'
import type { Transaction } from '../lib/types/Transaction'
import {
  listBalanceMonthSummaries,
  listBalancesForMonth,
  addBalance,
  updateBalance,
  deleteBalance,
} from '../lib/api/accounts'
import { getTransaction } from '../lib/api/transactions'
import { confirm } from '@tauri-apps/plugin-dialog'
import { DateTime } from 'luxon'

const route = useRoute()
const router = useRouter()
const store = useAccountsStore()
const toast = useToast()

const accountId = computed(() => Number(route.params.id))
const account = computed(() => store.accounts.find(a => a.id === accountId.value) ?? null)

// Month summaries (loaded on mount, refreshed after mutations)
const monthSummaries = ref<BalanceMonthSummary[]>([])

// Accordion: only one month open at a time
const openMonth = ref<string | null>(null)

// Cache of snapshot rows per month string (e.g. "2025-06")
const monthCache = ref(new Map<string, AccountBalance[]>())

async function toggleMonth(month: string) {
  if (openMonth.value === month) {
    openMonth.value = null
    return
  }
  openMonth.value = month
  if (!monthCache.value.has(month)) {
    const rows = await listBalancesForMonth(accountId.value, month)
    monthCache.value.set(month, rows)
  }
}

// Chart data — derived from accordion state
const chartMode = computed(() =>
  openMonth.value ? 'intramonth' as const : 'monthly' as const
)

const chartPoints = computed(() => {
  if (openMonth.value) {
    const rows = monthCache.value.get(openMonth.value) ?? []
    return rows.map(b => ({ date: b.recordedAt, balance: b.balance })).reverse()
  }
  return monthSummaries.value
    .map(s => ({ date: s.month + '-01', balance: s.latestBalance }))
    .reverse()
})

const chartTitle = computed(() =>
  openMonth.value
    ? DateTime.fromISO(openMonth.value + '-01').toFormat('MMMM yyyy')
    : 'All Time'
)

// Inline snapshot editing
const editingSnapshotId = ref<number | null>(null)
const draftBalance = ref(0)
const draftDate = ref('')

function startEdit(b: AccountBalance) {
  editingSnapshotId.value = b.id
  draftBalance.value = b.balance
  draftDate.value = b.recordedAt
}

function cancelEdit() {
  editingSnapshotId.value = null
}

async function saveEdit(b: AccountBalance) {
  await updateBalance({ id: b.id, balance: draftBalance.value, recordedAt: draftDate.value })
  toast.add({ title: 'Balance updated', color: 'success' })
  editingSnapshotId.value = null
  await refreshMonth(b.recordedAt.slice(0, 7))
}

async function removeSnapshot(b: AccountBalance) {
  const ok = await confirm(
    `Delete this snapshot from ${b.recordedAt}? This cannot be undone.`,
    { title: 'Delete Snapshot?', kind: 'warning' },
  )
  if (!ok) return
  await deleteBalance(b.id)
  toast.add({ title: 'Snapshot deleted', color: 'success' })
  await refreshMonth(b.recordedAt.slice(0, 7))
}

async function refreshMonth(month: string) {
  monthSummaries.value = await listBalanceMonthSummaries(accountId.value)
  if (monthCache.value.has(month)) {
    const rows = await listBalancesForMonth(accountId.value, month)
    monthCache.value.set(month, rows)
  }
}

// Add snapshot modal
const isAddModalOpen = ref(false)
const newBalance = ref(0)
const newDate = ref(DateTime.now().toISODate()!)

async function submitAddSnapshot() {
  const savedMonth = newDate.value.slice(0, 7) // capture before reset
  await addBalance({ accountId: accountId.value, balance: newBalance.value, recordedAt: newDate.value })
  toast.add({ title: 'Balance recorded', color: 'success' })
  isAddModalOpen.value = false
  newBalance.value = 0
  newDate.value = DateTime.now().toISODate()!
  monthSummaries.value = await listBalanceMonthSummaries(accountId.value)
  if (monthCache.value.has(savedMonth)) {
    const rows = await listBalancesForMonth(accountId.value, savedMonth)
    monthCache.value.set(savedMonth, rows)
  }
  // Also refresh store's latestBalances so the list page shows updated balance
  await store.loadList()
}

watch(isAddModalOpen, open => {
  if (!open) {
    newBalance.value = 0
    newDate.value = DateTime.now().toISODate()!
  }
})

// Edit / Archive account modal
const isEditModalOpen = ref(false)

function onAccountSaved() {
  isEditModalOpen.value = false
}

async function archiveAccount() {
  const ok = await confirm(
    `Archive "${account.value?.name}"? It will be excluded from dashboard totals.`,
    { title: 'Archive Account?', kind: 'warning' },
  )
  if (!ok) return
  await store.archive(accountId.value)
  router.push({ name: 'accounts' })
}

async function deleteAccount() {
  const ok = await confirm(
    `Permanently delete "${account.value?.name}" and all of its balance snapshots? This cannot be undone.`,
    { title: 'Delete Account?', kind: 'warning' },
  )
  if (!ok) return
  await store.remove(accountId.value)
  router.push({ name: 'accounts' })
}

// Linked transaction modal
const isTransactionModalOpen = ref(false)
const viewingTransaction = ref<Transaction | null>(null)

async function openTransaction(id: number) {
  viewingTransaction.value = await getTransaction(id)
  isTransactionModalOpen.value = true
}

watch(isTransactionModalOpen, open => { if (!open) viewingTransaction.value = null })

// Format helpers
const fmt = (n: number) => n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })

function formatMonth(month: string) {
  return DateTime.fromISO(month + '-01').toFormat('MMMM yyyy')
}

onMounted(async () => {
  if (!store.accounts.length) await store.loadList()
  monthSummaries.value = await listBalanceMonthSummaries(accountId.value)
})
</script>

<template>
  <div class="p-6 max-w-4xl">
    <!-- Breadcrumb -->
    <button
      class="text-sm text-primary mb-4 hover:underline"
      @click="router.push({ name: 'accounts' })"
    >
      ← Accounts
    </button>

    <!-- Account header -->
    <div v-if="account" class="flex items-start justify-between mb-6">
      <div>
        <h1 class="text-2xl font-bold">{{ account.name }}</h1>
        <p class="text-sm text-muted mt-1">
          {{ labelForAccountType(account.type) }}
          <template v-if="account.institution"> · {{ account.institution }}</template>
          <template v-if="account.includeInFireCalculations"> · <span class="text-green-600 font-medium">FIRE ✓</span></template>
        </p>
      </div>
      <div class="flex items-center gap-2">
        <UButton size="sm" variant="ghost" @click="isEditModalOpen = true">Edit</UButton>
        <UButton size="sm" variant="ghost" color="neutral" @click="archiveAccount">Archive</UButton>
      </div>
    </div>

    <!-- Chart -->
    <div class="border border-default rounded-lg p-4 mb-6">
      <p class="text-xs font-semibold uppercase tracking-wider text-muted mb-3">
        Balance History
        <span class="ml-1 font-normal normal-case">— {{ chartTitle }}</span>
      </p>
      <AccountBalanceChart
        v-if="chartPoints.length > 0"
        :points="chartPoints"
        :mode="chartMode"
      />
      <p v-else class="text-sm text-muted py-8 text-center">No snapshots yet</p>
    </div>

    <!-- Snapshot section header -->
    <div class="flex items-center justify-between mb-3">
      <h2 class="font-semibold">Balance Snapshots</h2>
      <UButton size="sm" icon="i-ph-plus" @click="isAddModalOpen = true">Add Snapshot</UButton>
    </div>

    <!-- Month accordion -->
    <div v-if="monthSummaries.length > 0" class="border border-default rounded-lg overflow-hidden">
      <div
        v-for="(summary, i) in monthSummaries"
        :key="summary.month"
        class="border-b border-default last:border-b-0"
      >
        <!-- Month header row -->
        <button
          class="w-full flex items-center justify-between px-4 py-3 hover:bg-elevated/50 transition-colors text-left"
          @click="toggleMonth(summary.month)"
        >
          <span class="text-sm font-medium">
            {{ openMonth === summary.month ? '▼' : '▶' }}
            {{ formatMonth(summary.month) }}
          </span>
          <span class="text-sm text-muted">
            {{ summary.count }} snapshot{{ summary.count !== 1 ? 's' : '' }} · {{ fmt(summary.latestBalance) }}
          </span>
        </button>

        <!-- Expanded snapshot rows -->
        <template v-if="openMonth === summary.month">
          <div
            v-if="!monthCache.has(summary.month)"
            class="px-4 py-3 text-sm text-muted bg-elevated/30"
          >
            Loading…
          </div>
          <template v-else>
            <div
              v-for="b in monthCache.get(summary.month)"
              :key="b.id"
              class="grid grid-cols-[130px_1fr_auto] items-center px-6 py-2.5 bg-elevated/20 border-t border-default"
            >
              <!-- Date (or inline edit) -->
              <div>
                <DateInput v-if="editingSnapshotId === b.id" v-model="draftDate" />
                <span v-else class="text-sm text-muted">{{ b.recordedAt }}</span>
              </div>

              <!-- Balance (or inline edit) -->
              <div>
                <CurrencyInput v-if="editingSnapshotId === b.id" v-model="draftBalance" class="w-32" />
                <span v-else class="text-sm font-semibold font-mono">{{ fmt(b.balance) }}</span>
              </div>

              <!-- Actions -->
              <div class="flex items-center gap-1 justify-end">
                <template v-if="editingSnapshotId === b.id">
                  <UButton size="xs" variant="ghost" @click="saveEdit(b)">Save</UButton>
                  <UButton size="xs" variant="ghost" color="neutral" @click="cancelEdit">Cancel</UButton>
                </template>
                <template v-else>
                  <UButton
                    v-if="b.linkedTransactionId != null"
                    size="xs"
                    variant="ghost"
                    icon="i-ph-receipt"
                    title="View linked transaction"
                    @click="openTransaction(b.linkedTransactionId!)"
                  />
                  <UButton size="xs" variant="ghost" icon="i-ph-pencil" @click="startEdit(b)" />
                  <UButton size="xs" variant="ghost" color="error" icon="i-ph-trash" @click="removeSnapshot(b)" />
                </template>
              </div>
            </div>
          </template>
        </template>
      </div>
    </div>

    <div v-else class="text-sm text-muted text-center py-8">
      No snapshots yet. Add one to start tracking this account.
    </div>

    <!-- Delete account danger zone -->
    <div class="mt-10 pt-6 border-t border-default">
      <h3 class="text-sm font-semibold text-error mb-1">Danger Zone</h3>
      <p class="text-xs text-muted mb-3">Permanently remove this account and all of its balance history.</p>
      <UButton size="sm" color="error" variant="soft" @click="deleteAccount">Delete Account</UButton>
    </div>

    <!-- Add snapshot modal -->
    <UModal v-model:open="isAddModalOpen" title="Add Balance Snapshot">
      <template #body>
        <UForm :state="{ newBalance, newDate }" class="space-y-4" @submit="submitAddSnapshot">
          <UFormField label="Balance">
            <CurrencyInput v-model="newBalance" class="w-full" />
          </UFormField>
          <UFormField label="Date">
            <DateInput v-model="newDate" class="w-full" />
          </UFormField>
          <div class="flex justify-end pt-2">
            <UButton type="submit">Save Snapshot</UButton>
          </div>
        </UForm>
      </template>
    </UModal>

    <!-- Edit account modal -->
    <UModal v-model:open="isEditModalOpen" title="Edit Account" class="w-112">
      <template #body>
        <AccountForm
          v-if="account"
          :key="account.id"
          :account="account"
          @saved="onAccountSaved"
        />
      </template>
    </UModal>

    <!-- Linked transaction modal -->
    <UModal v-model:open="isTransactionModalOpen" title="Transaction details">
      <template #body>
        <TransactionDetail v-if="viewingTransaction" :transaction="viewingTransaction" />
      </template>
    </UModal>
  </div>
</template>
```

- [ ] **Step 2: Commit**

```bash
git add src/pages/AccountDetail.vue
git commit -m "feat: add AccountDetail page with lazy month accordion and context-aware chart"
```

---

## Task 11: End-to-end verification

- [ ] **Step 1: Build the Tauri app**

```bash
cd /Users/tomgobich/code/trackmyfi
npm run tauri dev
```

Watch for Rust compile errors in the terminal. Expected: app starts with no errors.

- [ ] **Step 2: Verify list page**

Navigate to Accounts. Confirm:
- Three stat cards show correct Net Worth, FIRE total, Non-FIRE total
- Accounts are split into FIRE / Non-FIRE groups
- Each row shows name, institution, type, latest balance
- ⋯ button opens a dropdown with Edit and Archive options (does NOT navigate to detail page)
- Clicking anywhere else on a row navigates to `/accounts/:id`
- Archived section is collapsed; expanding it shows archived accounts with Restore/Delete options
- Adding a new account via "+ Add Account" works

- [ ] **Step 3: Verify detail page**

Click into any account. Confirm:
- Breadcrumb "← Accounts" navigates back
- Account name, type, institution, FIRE status shown
- Chart shows monthly history (one point per month)
- "Balance Snapshots" accordion shows month rows with count and latest balance
- Expanding a month loads and shows individual snapshots
- Chart switches to intra-month view when a month is expanded
- Chart reverts to monthly view when the month is collapsed
- Snapshot row shows edit/delete buttons; edit goes inline; save updates the row and refreshes month summary
- Linked transaction icon (🧾) appears and opens transaction modal when present
- "+ Add Snapshot" opens modal, saves, and refreshes the accordion

- [ ] **Step 4: Verify Dashboard is unaffected**

Navigate to Dashboard. Confirm net worth, investable net worth, and the net worth chart still load correctly (they still use `store.load()` via `allBalances`).

- [ ] **Step 5: Final commit if any fixups were needed**

```bash
git add -p
git commit -m "fix: post-integration fixups for accounts redesign"
```
