# CSV Import — Snapshot Batch Performance & Loading Feedback

**Date:** 2026-06-17
**Status:** Approved

## Problem

The "generate balance snapshots" path in `confirmImport` calls `createTransaction` individually per row in a JS `for` loop. Each call is a separate Tauri invoke → separate HTTP round-trip to Turso. A CSV with 50 rows = 50 network requests; imports take minutes or time out silently. The user has no feedback that anything is happening.

## Solution

Move the snapshot import path to a new batch Rust command that runs all inserts inside a single database transaction (one Turso round-trip total). Add a loading state to the frontend so the UI communicates progress while the batch runs.

## Rust — `commands/transactions.rs`

### New inner function

```rust
pub async fn bulk_create_transactions_with_snapshots(
    conn: &Connection,
    rows: &[NewTransaction],
) -> Result<i64, String>
```

- Opens a manual `BEGIN` on the connection.
- Calls the existing `create_transaction(conn, t)` inner function for each row in order.
- On any error, issues `ROLLBACK` and returns the error.
- On success, issues `COMMIT` and returns the count.
- Reuses all existing logic (`materialize_snapshots`, `is_liability`, `base_balance`, `insert_snapshot`) — nothing duplicated. Because all calls share the same `Connection` inside the `BEGIN` block, each row's `base_balance` query correctly sees snapshots written by earlier rows in the same transaction.

### New command wrapper

```rust
#[tauri::command]
pub async fn bulk_create_transactions_with_snapshots_cmd(
    db: State<'_, Db>,
    transactions: Vec<NewTransaction>,
) -> Result<i64, String>
```

Registered in `lib.rs` alongside the existing transaction commands.

## TypeScript API — `src/lib/api/transactions.ts`

```ts
export const bulkCreateTransactionsWithSnapshots = (transactions: NewTransaction[]) =>
  invoke<number>('bulk_create_transactions_with_snapshots_cmd', { transactions })
```

Reuses the existing `NewTransaction` interface — no new types needed.

## Frontend — `ImportWizard.vue`

### State

```ts
const importing = ref(false)
```

### `confirmImport` snapshot branch

1. Set `importing.value = true` at the top of the branch (before any awaits).
2. Call `addBalance` for seed if `needsSeed` — unchanged, stays as a pre-flight call so the batch has a base balance to start from.
3. Sort selected rows by date and apply the liability transfer swap — unchanged.
4. Call `await bulkCreateTransactionsWithSnapshots(sortedRows)` — replaces the entire `for` loop.
5. Wrap steps 2–4 in a `try/catch/finally`: `finally` sets `importing.value = false` so it always clears. `catch` adds a persistent error toast (see below) and returns early.

### UI

- `Import selected` button: add `:loading="importing"` — NuxtUI renders a spinner automatically.
- `← Back to settings` button: add `:disabled="importing"` — prevents navigation mid-import.
- Above the action row, show while `importing` is true:
  ```
  Saving {{ include.filter(Boolean).length }} transactions…
  ```
  Hidden once import completes or errors.

### Error toast

```ts
toast.add({
  title: 'Import failed',
  description: e.message,
  color: 'error',
  duration: 0,   // no auto-dismiss — user must close manually
})
```

Auto-dismiss is disabled because the import can run for several seconds; a toast that disappears before the user looks up would be silent data loss.

## What does NOT change

- The `bulkCreateTransactions` path (no snapshots) — already fixed in the prior session with a transaction wrapper.
- The `addBalance` seed call — stays separate; it must complete before the batch so the first row has a base balance.
- The liability transfer account swap — stays in the frontend, same as today.
- All existing Rust helpers (`materialize_snapshots`, `base_balance`, `is_liability`, `insert_snapshot`, `create_transaction`) — untouched.

## Testing

- Existing `transactions.rs` unit tests cover `create_transaction` and snapshot logic — no changes needed there.
- Add a new integration test `bulk_create_with_snapshots_sequential` in `tests/transactions.rs`:
  - Insert 3 transactions in date order with `update_balance: true`.
  - Assert that `account_balance` rows accumulate correctly (each row's balance reflects all prior inserts in the batch).
  - Assert all 3 `txn` rows exist with correct `generated_balance_id` values.
- No new Vitest tests — the frontend change is a pure control-flow refactor on the existing branch.
