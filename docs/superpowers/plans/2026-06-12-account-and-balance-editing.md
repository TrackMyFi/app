# Account & Balance History Editing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let users set an account's opened date, edit accounts, and edit/delete individual balance history snapshots.

**Architecture:** Add three Rust commands (`update_account`, `update_balance`, `delete_balance`) following the existing testable-inner-fn + thin-`_cmd`-wrapper pattern, expose them through the `lib/api` + Pinia store layers, then surface them in the UI: a dual-mode `AccountForm` (add/edit) hosted in a single `UModal`, and a new `BalanceRow` component with inline edit/delete.

**Tech Stack:** Rust + libsql + Tauri commands; Vue 3 `<script setup>`, Pinia, Nuxt UI v4 (`UModal`, `UForm`, `UInput`, `USelect`, `USwitch`), `@tauri-apps/plugin-dialog`, Luxon, existing `DateInput.vue`.

---

## File Structure

- `src-tauri/src/commands/accounts.rs` — add `UpdateBalance` struct, `update_account` / `update_balance` / `delete_balance` inner fns + `_cmd` wrappers.
- `src-tauri/src/lib.rs` — register the three new commands.
- `src-tauri/tests/roundtrip.rs` — extend `account_and_balance_roundtrip` with update/delete assertions.
- `src/lib/api/accounts.ts` — add `updateAccount`, `updateBalance`, `deleteBalance`.
- `src/stores/accounts.ts` — add `update`, `updateBalanceSnapshot`, `removeBalanceSnapshot`.
- `src/components/AccountForm.vue` — dual-mode (add/edit) + opened-date field, emits `saved`.
- `src/components/BalanceRow.vue` — **new**; inline edit/delete for one balance row.
- `src/pages/Accounts.vue` — "Add Account" button + edit button open a shared `UModal`; table uses `BalanceRow`.

---

## Task 1: Backend — `update_account`, `update_balance`, `delete_balance`

**Files:**
- Modify: `src-tauri/src/commands/accounts.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/tests/roundtrip.rs`

- [ ] **Step 1: Extend the roundtrip test with update/delete assertions**

In `src-tauri/tests/roundtrip.rs`, update the import line to include `UpdateBalance`:

```rust
use trackmyfi_app_lib::commands::accounts::{self, NewAccount, NewBalance, UpdateBalance};
```

Then, in `account_and_balance_roundtrip`, insert the following **after** the existing
`unarchive_account` block (the lines that assert `restored[0].is_active == true`) and
**before** the `// permanent delete` block:

```rust
    // edit account: editable fields change, is_active is preserved
    accounts::update_account(
        &conn,
        id,
        &NewAccount {
            name: "Brokerage (edited)".into(),
            r#type: "roth_ira".into(),
            institution: None,
            include_in_fire_calculations: false,
            created_at: "2025-12-15".into(),
        },
    )
    .await
    .unwrap();
    let edited = accounts::list_accounts(&conn).await.unwrap();
    assert_eq!(edited[0].name, "Brokerage (edited)");
    assert_eq!(edited[0].r#type, "roth_ira");
    assert_eq!(edited[0].institution, None);
    assert_eq!(edited[0].include_in_fire_calculations, false);
    assert_eq!(edited[0].created_at, "2025-12-15");
    assert_eq!(edited[0].is_active, true); // unchanged by edit

    // edit a single balance snapshot
    let bals_before = accounts::list_account_balances(&conn, id).await.unwrap();
    let target = bals_before[0].id; // the 2026-01-01 / 12345.67 row
    accounts::update_balance(
        &conn,
        &UpdateBalance {
            id: target,
            balance: 99999.99,
            recorded_at: "2026-01-15".into(),
        },
    )
    .await
    .unwrap();
    let bals_after = accounts::list_account_balances(&conn, id).await.unwrap();
    let edited_bal = bals_after.iter().find(|b| b.id == target).unwrap();
    assert_eq!(edited_bal.balance, 99999.99);
    assert_eq!(edited_bal.recorded_at, "2026-01-15");
    assert_eq!(bals_after.len(), 2); // still two rows

    // delete one snapshot: target gone, sibling intact
    accounts::delete_balance(&conn, target).await.unwrap();
    let bals_final = accounts::list_account_balances(&conn, id).await.unwrap();
    assert_eq!(bals_final.len(), 1);
    assert!(bals_final.iter().all(|b| b.id != target));
```

- [ ] **Step 2: Run the test to verify it fails to compile**

Run: `cd src-tauri && cargo test --test roundtrip account_and_balance_roundtrip`
Expected: FAIL — compile errors (`UpdateBalance` not found, no `update_account` / `update_balance` / `delete_balance`).

- [ ] **Step 3: Add the `UpdateBalance` struct**

In `src-tauri/src/commands/accounts.rs`, add after the `NewBalance` struct (around line 24):

```rust
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBalance {
    pub id: i32,
    pub balance: f64,
    pub recorded_at: String,
}
```

- [ ] **Step 4: Add the three inner fns**

In `src-tauri/src/commands/accounts.rs`, add to the `// ---- testable inner fns ----`
section. Put `update_account` after `create_account`:

```rust
pub async fn update_account(conn: &Connection, id: i32, a: &NewAccount) -> Result<(), String> {
    conn.execute(
        "UPDATE account SET name = ?1, type = ?2, institution = ?3, \
         include_in_fire_calculations = ?4, created_at = ?5 WHERE id = ?6",
        libsql::params![
            a.name.clone(),
            a.r#type.clone(),
            a.institution.clone(),
            a.include_in_fire_calculations,
            a.created_at.clone(),
            id
        ],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}
```

Put `update_balance` and `delete_balance` after `add_balance`:

```rust
pub async fn update_balance(conn: &Connection, b: &UpdateBalance) -> Result<(), String> {
    conn.execute(
        "UPDATE account_balance SET balance = ?1, recorded_at = ?2 WHERE id = ?3",
        libsql::params![b.balance, b.recorded_at.clone(), b.id],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn delete_balance(conn: &Connection, id: i32) -> Result<(), String> {
    conn.execute(
        "DELETE FROM account_balance WHERE id = ?1",
        libsql::params![id],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}
```

- [ ] **Step 5: Add the three `_cmd` wrappers**

In `src-tauri/src/commands/accounts.rs`, add to the `// ---- thin command wrappers ----`
section:

```rust
#[tauri::command]
pub async fn update_account_cmd(
    db: State<'_, Db>,
    id: i32,
    account: NewAccount,
) -> Result<(), String> {
    let conn = db.conn().await?;
    update_account(&conn, id, &account).await
}

#[tauri::command]
pub async fn update_balance_cmd(db: State<'_, Db>, balance: UpdateBalance) -> Result<(), String> {
    let conn = db.conn().await?;
    update_balance(&conn, &balance).await
}

#[tauri::command]
pub async fn delete_balance_cmd(db: State<'_, Db>, id: i32) -> Result<(), String> {
    let conn = db.conn().await?;
    delete_balance(&conn, id).await
}
```

- [ ] **Step 6: Register the commands**

In `src-tauri/src/lib.rs`, add to the `invoke_handler` list after
`commands::accounts::add_balance_cmd,`:

```rust
            commands::accounts::update_account_cmd,
            commands::accounts::update_balance_cmd,
            commands::accounts::delete_balance_cmd,
```

- [ ] **Step 7: Run the test to verify it passes**

Run: `cd src-tauri && cargo test --test roundtrip account_and_balance_roundtrip`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/commands/accounts.rs src-tauri/src/lib.rs src-tauri/tests/roundtrip.rs
git commit -m "feat: backend commands to update accounts and update/delete balances"
```

---

## Task 2: Frontend API + store wiring

**Files:**
- Modify: `src/lib/api/accounts.ts`
- Modify: `src/stores/accounts.ts`

- [ ] **Step 1: Add API wrappers**

In `src/lib/api/accounts.ts`, add after the existing `addBalance` export. The
`updateAccount` payload matches `createAccount`'s shape:

```ts
export const updateAccount = (id: number, account: {
  name: string; type: string; institution: string | null;
  includeInFireCalculations: boolean; createdAt: string
}) => invoke<void>('update_account_cmd', { id, account })
export const updateBalance = (balance: { id: number; balance: number; recordedAt: string }) =>
  invoke<void>('update_balance_cmd', { balance })
export const deleteBalance = (id: number) => invoke<void>('delete_balance_cmd', { id })
```

- [ ] **Step 2: Add store actions**

In `src/stores/accounts.ts`, add these three actions after the existing
`addBalanceSnapshot` line:

```ts
  async function update(id: number, a: Parameters<typeof api.updateAccount>[1]) { await api.updateAccount(id, a); await load() }
  async function updateBalanceSnapshot(b: Parameters<typeof api.updateBalance>[0]) { await api.updateBalance(b); await load() }
  async function removeBalanceSnapshot(id: number) { await api.deleteBalance(id); await load() }
```

Then add them to the store's returned object:

```ts
  return { accounts, allBalances, load, create, update, archive, unarchive, remove, addBalanceSnapshot, updateBalanceSnapshot, removeBalanceSnapshot }
```

- [ ] **Step 3: Verify typecheck passes**

Run: `npm run build`
Expected: PASS (no `vue-tsc` errors). This also compiles the rest of the app; an
unrelated pre-existing error elsewhere is not introduced by this task.

- [ ] **Step 4: Commit**

```bash
git add src/lib/api/accounts.ts src/stores/accounts.ts
git commit -m "feat: api + store actions for account/balance editing"
```

---

## Task 3: Dual-mode `AccountForm` with opened date

**Files:**
- Modify: `src/components/AccountForm.vue`

This rewrites `AccountForm.vue` so it serves both add and edit, adds an "Opened"
date field, and emits `saved` so a host modal can close. It no longer renders its
own `<UCard>` wrapper (the modal supplies the title/chrome).

- [ ] **Step 1: Replace the component**

Replace the entire contents of `src/components/AccountForm.vue` with:

```vue
<script setup lang="ts">
import { reactive, watch } from 'vue'
import { DateTime } from 'luxon'
import { useAccountsStore } from '../stores/accounts'
import { ACCOUNT_TYPES, defaultIncludeInFire, type AccountType } from '../lib/accountTypes'
import type { Account } from '../lib/types/Account'
import DateInput from './DateInput.vue'

const props = defineProps<{ account?: Account }>()
const emit = defineEmits<{ saved: [] }>()

const store = useAccountsStore()
const isEdit = !!props.account

const form = reactive({
  name: props.account?.name ?? '',
  type: (props.account?.type ?? 'checking') as AccountType,
  institution: props.account?.institution ?? '',
  includeInFireCalculations: props.account?.includeInFireCalculations ?? false,
  createdAt: props.account?.createdAt ?? DateTime.now().toISODate()!,
})

// Auto-default the FIRE toggle from the account type ONLY when adding, so editing
// an existing account never silently flips a user's stored choice.
if (!isEdit) {
  watch(
    () => form.type,
    (newType) => {
      form.includeInFireCalculations = defaultIncludeInFire(newType)
    },
    { immediate: true },
  )
}

const accountTypeItems = ACCOUNT_TYPES.map((t) => ({ label: t, value: t }))

async function onSubmit() {
  const payload = {
    name: form.name,
    type: form.type,
    institution: form.institution.trim() || null,
    includeInFireCalculations: form.includeInFireCalculations,
    createdAt: form.createdAt,
  }
  if (isEdit) {
    await store.update(props.account!.id, payload)
  } else {
    await store.create(payload)
  }
  emit('saved')
}
</script>

<template>
  <UForm :state="form" @submit="onSubmit" class="space-y-3">
    <UFormField label="Name" required>
      <UInput v-model="form.name" placeholder="e.g. Fidelity Brokerage" />
    </UFormField>
    <UFormField label="Type" required>
      <USelect
        v-model="form.type"
        :items="accountTypeItems"
        value-key="value"
        placeholder="Select account type"
      />
    </UFormField>
    <UFormField label="Institution (optional)">
      <UInput v-model="form.institution" placeholder="e.g. Fidelity" />
    </UFormField>
    <UFormField label="Opened">
      <DateInput v-model="form.createdAt" />
    </UFormField>
    <UFormField label="Include in FIRE calculations">
      <USwitch v-model="form.includeInFireCalculations" />
    </UFormField>
    <UButton type="submit" :disabled="!form.name">
      {{ isEdit ? 'Save Changes' : 'Add Account' }}
    </UButton>
  </UForm>
</template>
```

- [ ] **Step 2: Verify typecheck passes**

Run: `npm run build`
Expected: PASS.

> Note: `Accounts.vue` still renders `<AccountForm />` standalone at this point —
> that's fine; it now shows without the card header. Task 4 moves it into the modal.

- [ ] **Step 3: Commit**

```bash
git add src/components/AccountForm.vue
git commit -m "feat: dual-mode AccountForm with opened date and saved event"
```

---

## Task 4: `Accounts.vue` — Add/Edit modal

**Files:**
- Modify: `src/pages/Accounts.vue`

Replace the always-visible Add Account card with an "Add Account" button, add an
"Edit" button per active account, and host `AccountForm` in a single `UModal`.

- [ ] **Step 1: Add modal state + handlers to the script**

In `src/pages/Accounts.vue`, add `ref` to the existing `vue` import so it reads:

```ts
import { computed, onMounted, ref } from 'vue'
```

Then add this state and these handlers after the `store` declaration (after
`const store = useAccountsStore()`):

```ts
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

function onAccountSaved() {
  isAccountModalOpen.value = false
  editingAccount.value = null
}
```

- [ ] **Step 2: Replace the inline form with a button + modal in the template**

In `src/pages/Accounts.vue`, replace this block:

```vue
    <AccountForm />
```

with:

```vue
    <div class="mb-6">
      <UButton icon="i-lucide-plus" @click="openAdd">Add Account</UButton>
    </div>

    <UModal
      v-model:open="isAccountModalOpen"
      :title="editingAccount ? 'Edit Account' : 'Add Account'"
    >
      <template #body>
        <AccountForm
          :key="editingAccount?.id ?? 'new'"
          :account="editingAccount ?? undefined"
          @saved="onAccountSaved"
        />
      </template>
    </UModal>
```

- [ ] **Step 3: Add an Edit button to each active account card header**

In `src/pages/Accounts.vue`, in the active-accounts card header, replace the
Archive button block:

```vue
              <UButton
                size="sm"
                color="error"
                variant="ghost"
                @click="archive(account.id)"
              >
                Archive
              </UButton>
```

with an Edit button followed by the Archive button:

```vue
              <UButton
                size="sm"
                variant="ghost"
                @click="openEdit(account)"
              >
                Edit
              </UButton>
              <UButton
                size="sm"
                color="error"
                variant="ghost"
                @click="archive(account.id)"
              >
                Archive
              </UButton>
```

- [ ] **Step 4: Verify typecheck passes**

Run: `npm run build`
Expected: PASS.

- [ ] **Step 5: Manually verify add + edit**

Run: `npm run tauri dev`
- Click "Add Account" → modal opens titled "Add Account"; fill name, pick a type,
  set Opened date, submit → modal closes, account appears.
- Click "Edit" on that account → modal opens titled "Edit Account" pre-filled
  (including Opened date); change the name and Opened date, click "Save Changes"
  → modal closes, card reflects the change.

- [ ] **Step 6: Commit**

```bash
git add src/pages/Accounts.vue
git commit -m "feat: add/edit accounts via shared modal"
```

---

## Task 5: `BalanceRow` — inline edit/delete

**Files:**
- Create: `src/components/BalanceRow.vue`
- Modify: `src/pages/Accounts.vue`

- [ ] **Step 1: Create `BalanceRow.vue`**

Create `src/components/BalanceRow.vue` with:

```vue
<script setup lang="ts">
import { ref } from 'vue'
import { confirm } from '@tauri-apps/plugin-dialog'
import { useAccountsStore } from '../stores/accounts'
import type { AccountBalance } from '../lib/types/AccountBalance'
import DateInput from './DateInput.vue'

const props = defineProps<{ balance: AccountBalance }>()

const store = useAccountsStore()

const isEditing = ref(false)
const draftBalance = ref<number>(props.balance.balance)
const draftDate = ref<string>(props.balance.recordedAt)

function startEdit() {
  draftBalance.value = props.balance.balance
  draftDate.value = props.balance.recordedAt
  isEditing.value = true
}

function cancelEdit() {
  isEditing.value = false
}

async function save() {
  await store.updateBalanceSnapshot({
    id: props.balance.id,
    balance: draftBalance.value,
    recordedAt: draftDate.value,
  })
  isEditing.value = false
}

async function remove() {
  const ok = await confirm(
    `Delete this balance snapshot from ${props.balance.recordedAt}? This cannot be undone.`,
    { title: 'Delete Snapshot?', kind: 'warning' },
  )
  if (ok) await store.removeBalanceSnapshot(props.balance.id)
}

const formatted = (n: number) =>
  n.toLocaleString('en-US', { style: 'currency', currency: 'USD' })
</script>

<template>
  <tr class="border-b border-gray-100 last:border-0">
    <template v-if="isEditing">
      <td class="py-1 pr-6">
        <DateInput v-model="draftDate" />
      </td>
      <td class="py-1 text-right">
        <UInput
          v-model.number="draftBalance"
          type="number"
          step="0.01"
          class="w-32"
        />
      </td>
      <td class="py-1 text-right">
        <div class="flex justify-end gap-1">
          <UButton size="xs" variant="ghost" @click="save">Save</UButton>
          <UButton size="xs" variant="ghost" color="neutral" @click="cancelEdit">Cancel</UButton>
        </div>
      </td>
    </template>
    <template v-else>
      <td class="py-1 pr-6 text-gray-600">{{ balance.recordedAt }}</td>
      <td class="py-1 text-right font-mono">{{ formatted(balance.balance) }}</td>
      <td class="py-1 text-right">
        <div class="flex justify-end gap-1">
          <UButton size="xs" variant="ghost" @click="startEdit">Edit</UButton>
          <UButton size="xs" variant="ghost" color="error" @click="remove">Delete</UButton>
        </div>
      </td>
    </template>
  </tr>
</template>
```

- [ ] **Step 2: Use `BalanceRow` in the table and add the actions column header**

In `src/pages/Accounts.vue`, replace the `<thead>` block:

```vue
                <thead>
                  <tr class="text-left text-gray-500 border-b">
                    <th class="pb-1 pr-6 font-medium">Date</th>
                    <th class="pb-1 font-medium text-right">Balance</th>
                  </tr>
                </thead>
```

with (adds an empty actions header column):

```vue
                <thead>
                  <tr class="text-left text-gray-500 border-b">
                    <th class="pb-1 pr-6 font-medium">Date</th>
                    <th class="pb-1 font-medium text-right">Balance</th>
                    <th class="pb-1 font-medium text-right"></th>
                  </tr>
                </thead>
```

Then replace the `<tbody>` block:

```vue
                <tbody>
                  <tr
                    v-for="b in accountBalances(account.id)"
                    :key="b.id"
                    class="border-b border-gray-100 last:border-0"
                  >
                    <td class="py-1 pr-6 text-gray-600">{{ b.recordedAt }}</td>
                    <td class="py-1 text-right font-mono">
                      {{ b.balance.toLocaleString('en-US', { style: 'currency', currency: 'USD' }) }}
                    </td>
                  </tr>
                </tbody>
```

with:

```vue
                <tbody>
                  <BalanceRow
                    v-for="b in accountBalances(account.id)"
                    :key="b.id"
                    :balance="b"
                  />
                </tbody>
```

- [ ] **Step 3: Import `BalanceRow`**

In `src/pages/Accounts.vue`, add the import after the `BalanceForm` import:

```ts
import BalanceRow from '../components/BalanceRow.vue'
```

> Note: Nuxt UI components (`UButton`, `UInput`, etc.) are auto-imported, but
> local `.vue` components in this project are imported explicitly (see the existing
> `AccountForm` / `BalanceForm` imports), so `BalanceRow` needs this line.

- [ ] **Step 4: Verify typecheck passes**

Run: `npm run build`
Expected: PASS.

- [ ] **Step 5: Manually verify edit + delete**

Run: `npm run tauri dev`
- On an account with at least two snapshots, click "Edit" on a row → date and
  balance become inputs; change both, click "Save" → row shows updated values,
  and the latest-balance/dashboard reflect it.
- Click "Edit" then "Cancel" → row reverts with no change.
- Click "Delete" on a row → Tauri confirm dialog appears; confirm → row
  disappears, sibling rows remain.

- [ ] **Step 6: Commit**

```bash
git add src/components/BalanceRow.vue src/pages/Accounts.vue
git commit -m "feat: inline edit and delete for balance history rows"
```

---

## Self-Review Notes

- **Spec coverage:** #1 opened date → Task 3 (Opened `DateInput`, defaults today)
  + Task 1 (`update_account` persists `created_at`). #2 edit accounts → Tasks 1–4.
  #3 edit balances → Tasks 1, 2, 5. #4 delete balances → Tasks 1, 2, 5.
- **`is_active` preserved on edit** → asserted in Task 1 Step 1; `update_account`
  SQL omits `is_active`.
- **Type-default-on-add-only** → Task 3 guards the `watch` behind `if (!isEdit)`.
- **Immutable `account_id`** → `UpdateBalance` has no `account_id`; `BalanceRow`
  never sends one.
- **Type consistency:** store `update(id, payload)` matches
  `api.updateAccount(id, account)`; `updateBalanceSnapshot`/`removeBalanceSnapshot`
  match `BalanceRow` calls; `UpdateBalance { id, balance, recorded_at }` matches the
  TS `{ id, balance, recordedAt }` via `camelCase` serde rename.
