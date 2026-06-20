# Handoff: Fix B — slow Turso bulk writes (CSV import)

This is the follow-up to **Fix A** (local-first startup), which is already done. Fix A
made *reads* feel instant by serving the local replica file at boot and pulling the
cloud in the background. Fix B is about *writes*, which Fix A did not touch.

## The actual problem

On a libSQL **embedded replica** (`Builder::new_remote_replica`), reads are served
from the local file, but **every write is forwarded to the remote Turso primary over
the network**. So each individual SQL write statement is a network round-trip.

A CSV import amplifies this badly. The import goes through
`bulk_create_transactions_with_snapshots` in
[`src-tauri/src/commands/transactions.rs`](../src-tauri/src/commands/transactions.rs),
which, **per row**:

- runs several snapshot reads (`base_balance`, `balance_before`, …) — local, cheap, and
- runs multiple `INSERT`/`UPDATE`s (the txn + generated balance snapshots) — each a
  **remote round-trip**.

It also uses a manual `BEGIN`/`COMMIT` interactive transaction
(`bulk_create_transactions_with_snapshots`, ~line 563) and the plain
`bulk_create_transactions` (~line 525) uses `conn.transaction()`. Over an embedded
replica, an interactive transaction holds a stream open against the primary and **each
statement inside it is its own round-trip** — there is no local batching.

Net: importing N rows ≈ (several × N) network hops. That's the slowness. Confirmed with
the user: their imports use the **snapshot path** (`*_with_snapshots`).

## Fix direction (cheapest → most involved)

1. **Collapse inserts into one round-trip per chunk.** Build a single multi-row
   `INSERT INTO txn (...) VALUES (..),(..),(..)` (chunk under SQLite's 999 bound-variable
   limit, so ~`floor(999 / cols)` rows per statement) and send it with one `execute` /
   `execute_batch`. This alone turns hundreds of hops into a handful.

2. **Precompute snapshots in memory instead of read-after-write per row.** The snapshot
   projection is deterministic given the prior balance, so the interleaved
   `base_balance`/`balance_before` reads aren't necessary during the import — compute the
   running balances in Rust as you walk the (date-ascending) rows, then emit all the
   generated balance snapshots in the same batched insert as step 1. This removes the
   remote read/write ping-pong that dominates the snapshot path. The pure projection
   logic already exists on the frontend under `src/lib/` (`balances/`, `csv/`); mirror it
   (or reuse the existing Rust snapshot helpers `side_delta`/`snapshot_delta`/
   `reproject_account`) so the in-memory computation matches what `create_transaction`
   would have written.

3. **`read_your_writes(false)` on the import path's replica builder** if the import
   doesn't need to immediately read back its own writes mid-loop — it avoids an implicit
   pull after each write. (Default is `true`.) Lower impact than 1–2, but free once the
   reads are gone.

## Suggested approach

Add a dedicated import function that does steps 1+2: walk the date-sorted rows once,
compute txn rows **and** their balance snapshots in memory, then write them in batched
multi-row inserts inside a single transaction. Keep the existing per-row
`create_transaction` path for the interactive single-add UI (where latency doesn't
matter). Note the current code relies on `generated_balance_id` /
`generated_balance_to_id` linking each txn to the snapshot rows it created — the batched
path has to preserve that linkage (insert snapshots first, capture their ids, then insert
txns referencing them, or vice-versa with an update pass).

## Verify

There are Rust tests around snapshot propagation in
[`src-tauri/src/commands/transactions.rs`](../src-tauri/src/commands/transactions.rs)
(`inserting_past_transaction_propagates_to_later_linked_snapshots`,
`transfer_switch_writes_two_snapshots`, etc.). After the rewrite, the batched import must
produce **identical** rows to the row-by-row path — add a test that imports a fixture via
both paths and asserts the resulting `txn` + balance snapshot tables match.

## Related (out of scope but worth noting)

Writes currently only push to the cloud on the 15-min backstop timer, the manual sync
button, or app close (`do_sync` is not called after individual mutations). So an edit on
one device can take up to 15 min (or a quit) to reach another device. If you want
cross-device writes to feel live, consider a debounced `do_sync` after mutations. This is
a separate concern from import *speed* and was not part of Fix A or B.
