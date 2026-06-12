# Account & Balance History Editing — Design

Date: 2026-06-12
Status: Approved

## Goal

Round out Phase 1's account management with four capabilities:

1. Specify when an account was opened via the account form (defaulting to today).
2. Edit existing accounts.
3. Edit an account's balance history items.
4. Delete an account's balance history items.

## Context

The `account` table already has a `created_at` column and the `Account` model
already exposes it; today `AccountForm.vue` hardcodes `DateTime.now()` on create,
so feature #1 is purely surfacing an existing field. Features #2–4 need new
Rust commands plus UI.

Existing conventions this design follows:

- **Rust**: testable inner async fns (`fn(conn, ...)`) in
  `src-tauri/src/commands/accounts.rs`, each with a thin `#[tauri::command]`
  `_cmd` wrapper that grabs a connection and delegates. Wrappers are registered
  in `src-tauri/src/lib.rs`.
- **Frontend**: `lib/api/accounts.ts` (`invoke` wrappers) → `stores/accounts.ts`
  (Pinia, every mutation calls `load()` to refresh) → components.
- A reusable `DateInput.vue` (ISO date string `v-model`) already exists.
- Delete confirmations use `@tauri-apps/plugin-dialog`'s async `confirm()`
  (the webview's native `window.confirm` is a no-op).
- Nuxt UI v4 (`@nuxt/ui` ^4.8.2) is available, including `UModal`.

## Backend — `src-tauri/src/commands/accounts.rs`

Add three inner fns + three `_cmd` wrappers, registered in `lib.rs`.

### `update_account`

```rust
pub async fn update_account(conn: &Connection, id: i32, a: &NewAccount) -> Result<(), String>
```

`UPDATE account SET name = ?, type = ?, institution = ?,
include_in_fire_calculations = ?, created_at = ? WHERE id = ?`.

Reuses the existing `NewAccount` payload struct plus a separate `id` argument.
Deliberately does **not** touch `is_active` — archiving/restoring remain their
own commands, so editing an account never changes its archived state.

Wrapper: `update_account_cmd(db, id: i32, account: NewAccount)`.

### `update_balance`

New payload struct:

```rust
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBalance {
    pub id: i32,
    pub balance: f64,
    pub recorded_at: String,
}
```

```rust
pub async fn update_balance(conn: &Connection, b: &UpdateBalance) -> Result<(), String>
```

`UPDATE account_balance SET balance = ?, recorded_at = ? WHERE id = ?`. The
`account_id` is immutable, so it is not part of the payload.

Wrapper: `update_balance_cmd(db, balance: UpdateBalance)`.

### `delete_balance`

```rust
pub async fn delete_balance(conn: &Connection, id: i32) -> Result<(), String>
```

`DELETE FROM account_balance WHERE id = ?`.

Wrapper: `delete_balance_cmd(db, id: i32)`.

No schema or migration changes are required.

## Frontend data layer

### `lib/api/accounts.ts`

```ts
export const updateAccount = (id: number, account: { /* same shape as createAccount */ }) =>
  invoke<void>('update_account_cmd', { id, account })
export const updateBalance = (balance: { id: number; balance: number; recordedAt: string }) =>
  invoke<void>('update_balance_cmd', { balance })
export const deleteBalance = (id: number) => invoke<void>('delete_balance_cmd', { id })
```

### `stores/accounts.ts`

Add three actions, each calling the API then `load()` (matching existing
`create`/`archive`/`remove`):

```ts
async function update(id: number, a) { await api.updateAccount(id, a); await load() }
async function updateBalanceSnapshot(b) { await api.updateBalance(b); await load() }
async function removeBalanceSnapshot(id: number) { await api.deleteBalance(id); await load() }
```

## Feature #1 + dual-mode `AccountForm.vue`

`AccountForm` becomes a single component used for both adding and editing, and
always renders **inside a modal** (no longer an always-visible card).

- New optional prop `account?: Account`.
  - **Absent → add mode**: blank form, submit button "Add Account", calls
    `store.create`.
  - **Present → edit mode**: form pre-filled from the account (incl. opened
    date), submit button "Save Changes", calls `store.update(account.id, …)`.
- Add an **"Opened" `DateInput`** bound to a `createdAt` field, defaulting to
  today in add mode and to `account.createdAt` in edit mode. This is sent as
  `createdAt` to the backend (feature #1).
- The existing `type → includeInFireCalculations` auto-default `watch` fires
  **only in add mode**. In edit mode the stored value is preserved and changing
  the type does not silently flip the FIRE toggle (the user controls it
  explicitly).
- On successful submit the form `emit('saved')` (both modes) so the host page
  can close the modal. In add mode it also resets its fields.
- The component drops its own `<UCard>` header wrapper; the modal supplies the
  title.

## Feature #2 — edit accounts (`Accounts.vue`)

- Replace the always-visible "Add Account" card at the top of the page with an
  **"Add Account" button**.
- A single `UModal` (`v-model:open`) hosts `<AccountForm :account="editingAccount" @saved="onSaved">`.
  - "Add Account" button → `editingAccount = null`, open modal; title "Add Account".
  - Each active account card gets an **"Edit" button** → `editingAccount = account`,
    open modal; title "Edit Account".
  - `onSaved` closes the modal and clears `editingAccount`.
- Because `AccountForm` is re-created per open (keyed by `editingAccount?.id ?? 'new'`),
  it initializes cleanly for each use without stale state.

## Features #3 & #4 — balance row edit/delete (new `BalanceRow.vue`)

Extract each balance-history `<tr>` from the `Accounts.vue` table into a
`BalanceRow.vue` component. This keeps the table markup in `Accounts.vue` simple
and mirrors how `BalanceForm.vue` calls the store directly.

- Props: `balance: AccountBalance`.
- Default (read) state: renders the date and currency-formatted balance, plus
  **Edit** and **Delete** ghost buttons.
- Edit state (local `isEditing` ref): the date and balance cells become a
  `DateInput` and a numeric `UInput`, with **Save** / **Cancel** buttons.
  - Save → `store.updateBalanceSnapshot({ id, balance, recordedAt })`, then exit
    edit state. Cancel → discard local edits, exit edit state.
- Delete → `@tauri-apps/plugin-dialog` `confirm()` ("Delete this balance
  snapshot from <date>? This cannot be undone."), and on confirm
  `store.removeBalanceSnapshot(id)`.

## Testing

- Extend the existing Rust `account_and_balance_roundtrip` test in
  `src-tauri/tests/roundtrip.rs`:
  - After `update_account`, assert the editable fields changed and `is_active`
    is unchanged.
  - After `update_balance`, assert `balance` and `recorded_at` changed for the
    target row.
  - After `delete_balance`, assert that row is gone and sibling rows are intact.
- No frontend component tests are added — consistent with the current repo,
  where only `src/lib/fire` has unit tests and UI components are covered
  manually.

## Out of scope

- Bulk editing balances.
- Changing a balance's `account_id` (moving a snapshot between accounts).
- Validation beyond what the existing forms already enforce.
