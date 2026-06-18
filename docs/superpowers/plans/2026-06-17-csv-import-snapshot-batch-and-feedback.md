# CSV Import Snapshot Batch & Feedback Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the one-at-a-time `createTransaction` loop in the snapshot import path with a single batched Rust command and add a loading state to the UI.

**Architecture:** A new Rust inner function wraps the existing `create_transaction` logic inside a manual `BEGIN`/`COMMIT` block so all row inserts (including snapshot materialization) execute in one database transaction. The frontend replaces the JS `for` loop with a single `invoke` call and tracks an `importing` ref to drive the loading UI.

**Tech Stack:** Rust · libSQL 0.9 · Tauri 2 · Vue 3 · NuxtUI 4

---

## File Map

| File | Change |
|------|--------|
| `src-tauri/tests/transactions.rs` | Add integration test |
| `src-tauri/src/commands/transactions.rs` | Add inner fn + command wrapper |
| `src-tauri/src/lib.rs` | Register new command |
| `src/lib/api/transactions.ts` | Add TS API fn |
| `src/components/ImportWizard.vue` | Loading state + batch call |

---

### Task 1: Write the failing integration test

**Files:**
- Modify: `src-tauri/tests/transactions.rs`

- [ ] **Step 1: Add the test** — append to `src-tauri/tests/transactions.rs`:

```rust
#[tokio::test]
async fn bulk_create_with_snapshots_sequential_balances() {
    let conn = setup().await;
    let acct = accounts::create_account(&conn, &NewAccount {
        name: "Checking".into(), r#type: "checking".into(), institution: None,
        include_in_fire_calculations: false, created_at: "2026-01-01".into(),
    }).await.unwrap();

    // Seed balance: $1,000 on Jan 1 — gives base_balance something to start from.
    accounts::add_balance(&conn, &accounts::NewBalance {
        account_id: acct, balance: 1000.0, recorded_at: "2026-01-01".into(),
    }).await.unwrap();

    // Three rows in ascending date order, as the frontend sorts before calling.
    let rows = vec![
        NewTransaction {
            account_id: acct, transfer_account_id: None,
            amount: 100.0, description: "expense 1".into(),
            date: "2026-01-02".into(), r#type: "expense".into(),
            category: "uncategorized".into(), is_contribution: false,
            import_source: "csv".into(), update_balance: true,
            created_at: "2026-01-02".into(),
        },
        NewTransaction {
            account_id: acct, transfer_account_id: None,
            amount: 200.0, description: "expense 2".into(),
            date: "2026-01-03".into(), r#type: "expense".into(),
            category: "uncategorized".into(), is_contribution: false,
            import_source: "csv".into(), update_balance: true,
            created_at: "2026-01-03".into(),
        },
        NewTransaction {
            account_id: acct, transfer_account_id: None,
            amount: 500.0, description: "income".into(),
            date: "2026-01-04".into(), r#type: "income".into(),
            category: "uncategorized".into(), is_contribution: false,
            import_source: "csv".into(), update_balance: true,
            created_at: "2026-01-04".into(),
        },
    ];

    let n = transactions::bulk_create_transactions_with_snapshots(&conn, &rows).await.unwrap();
    assert_eq!(n, 3);

    // All three transactions were inserted.
    let page = transactions::list_transactions(&conn, &TransactionFilter::default()).await.unwrap();
    assert_eq!(page.rows.len(), 3);

    // Every transaction must have a generated_balance_id — snapshot was materialized.
    assert!(page.rows.iter().all(|r| r.generated_balance_id.is_some()));

    // Running balances chain correctly:
    //   seed 1000 → -100 → 900 → -200 → 700 → +500 → 1200
    assert_eq!(latest_balance(&conn, acct).await, 1200.0);

    // 1 seed + 3 generated = 4 balance rows total.
    assert_eq!(balance_count(&conn).await, 4);
}
```

- [ ] **Step 2: Run test — expect compile error** (function not yet defined)

```bash
cargo test --manifest-path src-tauri/Cargo.toml bulk_create_with_snapshots_sequential_balances 2>&1 | tail -20
```

Expected: compile error — `no function 'bulk_create_transactions_with_snapshots' found`

---

### Task 2: Implement the Rust inner function

**Files:**
- Modify: `src-tauri/src/commands/transactions.rs`

- [ ] **Step 1: Add the inner function** — insert after `bulk_create_transactions` (around line 439):

```rust
/// Import many transactions with balance snapshot generation in a single database
/// transaction. Rows must arrive sorted by date (ascending) so each base_balance
/// query sees the snapshots written by prior rows in the same transaction.
///
/// Uses manual BEGIN/COMMIT (not conn.transaction()) so we can reuse the existing
/// create_transaction inner function and all its snapshot helpers unchanged.
pub async fn bulk_create_transactions_with_snapshots(
    conn: &Connection,
    rows: &[NewTransaction],
) -> Result<i64, String> {
    conn.execute("BEGIN", ()).await.map_err(|e| e.to_string())?;
    let mut count = 0i64;
    for t in rows {
        if let Err(e) = create_transaction(conn, t).await {
            conn.execute("ROLLBACK", ()).await.ok();
            return Err(e);
        }
        count += 1;
    }
    conn.execute("COMMIT", ()).await.map_err(|e| e.to_string())?;
    Ok(count)
}
```

- [ ] **Step 2: Run the test — expect pass**

```bash
cargo test --manifest-path src-tauri/Cargo.toml bulk_create_with_snapshots_sequential_balances 2>&1 | tail -10
```

Expected: `test bulk_create_with_snapshots_sequential_balances ... ok`

- [ ] **Step 3: Run the full test suite — confirm no regressions**

```bash
cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | tail -20
```

Expected: all tests pass, none failing.

---

### Task 3: Add the command wrapper and register it

**Files:**
- Modify: `src-tauri/src/commands/transactions.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add the command wrapper** — append after `bulk_create_transactions_cmd` in `commands/transactions.rs`:

```rust
#[tauri::command]
pub async fn bulk_create_transactions_with_snapshots_cmd(
    db: State<'_, Db>,
    transactions: Vec<NewTransaction>,
) -> Result<i64, String> {
    let conn = db.conn().await?;
    bulk_create_transactions_with_snapshots(&conn, &transactions).await
}
```

- [ ] **Step 2: Register in `lib.rs`** — add the new command to the `invoke_handler` list in `src-tauri/src/lib.rs`, after `commands::transactions::bulk_create_transactions_cmd`:

```rust
commands::transactions::bulk_create_transactions_with_snapshots_cmd,
```

- [ ] **Step 3: Build — confirm it compiles**

```bash
cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5
```

Expected: `Finished` with no errors.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/transactions.rs src-tauri/src/lib.rs src-tauri/tests/transactions.rs
git commit -m "feat: add bulk_create_transactions_with_snapshots Rust command"
```

---

### Task 4: Add the TypeScript API function

**Files:**
- Modify: `src/lib/api/transactions.ts`

- [ ] **Step 1: Add the export** — append to `src/lib/api/transactions.ts`:

```ts
export const bulkCreateTransactionsWithSnapshots = (transactions: NewTransaction[]) =>
  invoke<number>('bulk_create_transactions_with_snapshots_cmd', { transactions })
```

- [ ] **Step 2: Type-check**

```bash
npx vue-tsc --noEmit 2>&1 | head -20
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/api/transactions.ts
git commit -m "feat: add bulkCreateTransactionsWithSnapshots TS API"
```

---

### Task 5: Update ImportWizard.vue

**Files:**
- Modify: `src/components/ImportWizard.vue`

- [ ] **Step 1: Update the import line** — in `<script setup>`, replace:

```ts
import { bulkCreateTransactions, createTransaction } from '../lib/api/transactions'
```

with:

```ts
import { bulkCreateTransactions, bulkCreateTransactionsWithSnapshots } from '../lib/api/transactions'
```

(`createTransaction` is no longer used — the batch command replaces the for loop.)

- [ ] **Step 2: Add the `importing` ref** — after the existing `const generateSnapshots = ref(false)` line, add:

```ts
const importing = ref(false)
```

- [ ] **Step 3: Replace the snapshot branch in `confirmImport`** — replace the entire `else` block (from `} else {` through the closing `}` before `await txnStore.load()`) with:

```ts
  } else {
    importing.value = true
    try {
      if (needsSeed.value) {
        await addBalance({
          accountId: accountId.value!,
          balance: seedBalance.value,
          recordedAt: earliestDate.value,
        })
      }
      const sorted = [...selectedRows]
        .sort((a, b) => a.date.localeCompare(b.date))
        .map((row) => {
          const isLiabTransfer =
            isLiabilityAccount.value && row.type === 'transfer' && row.transferAccountId != null
          return {
            ...row,
            accountId: isLiabTransfer ? row.transferAccountId! : row.accountId,
            transferAccountId: isLiabTransfer ? row.accountId : row.transferAccountId,
            updateBalance: true,
          }
        })
      await bulkCreateTransactionsWithSnapshots(sorted)
      await accountsStore.load()
    } catch (e: any) {
      toast.add({
        title: 'Import failed',
        description: e?.message ?? String(e),
        color: 'error',
        duration: 0,
      })
      return
    } finally {
      importing.value = false
    }
  }
```

- [ ] **Step 4: Update the action row in the template** — replace the existing `<div class="flex justify-between">` action row at the bottom of step 3 (the one with "← Back to settings" and "Import selected"):

```html
      <div class="flex justify-between">
        <UButton variant="ghost" :disabled="importing" @click="step = 2">← Back to settings</UButton>
        <div class="flex items-center gap-3">
          <p v-if="importing" class="text-sm text-muted">
            Saving {{ include.filter(Boolean).length }} transactions…
          </p>
          <UButton :disabled="!include.some(Boolean)" :loading="importing" @click="confirmImport">
            Import selected
          </UButton>
        </div>
      </div>
```

- [ ] **Step 5: Type-check**

```bash
npx vue-tsc --noEmit 2>&1 | head -20
```

Expected: no errors.

- [ ] **Step 6: Run Vitest**

```bash
npm run test -- --run 2>&1 | tail -15
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/components/ImportWizard.vue
git commit -m "feat: loading state and batch snapshot import in ImportWizard"
```
