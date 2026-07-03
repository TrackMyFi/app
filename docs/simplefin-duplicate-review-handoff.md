# Handoff: SimpleFIN vs. manual/CSV duplicate review

> **Status: IMPLEMENTED (2026-07-03).** On-demand candidate query
> (`duplicate_candidates` / `simplefin_duplicate_candidates` in
> `src-tauri/src/simplefin.rs`, greedy one-to-one matching, ±3-day window,
> bucket classification included), keep-snapshot delete variant
> (`delete_transaction_keep_snapshot` in
> `src-tauri/src/commands/transactions.rs` — no reprojection, orphaned
> snapshot becomes a manual anchor), review modal
> (`src/components/SimpleFinDuplicateReview.vue`) opened from a
> "Review possible duplicates" button on the Bank Sync settings page.
> Bucket resolutions as designed: ordinary → delete the non-SimpleFIN row
> keeping its snapshot; net-deposit → same, but default-unchecked in the UI;
> contribution → reversed, the SimpleFIN row is deleted via the normal
> delete path. No schema change needed; the optional `dismissed_pairs`
> polish and bulk-delete command were not built.

Scoped in a planning session (no code written). This covers the problem, the design
that was rejected and why, the design that was agreed on, and the concrete pieces
needed to build it.

## The problem

SimpleFIN Bridge import (`src-tauri/src/simplefin.rs`) has no awareness of manual or
CSV-imported transactions. Its only dedup key is `txn.simplefin_id`
(`idx_txn_simplefin_id`, unique), which manual/CSV rows never have. So if you've been
tracking an account by hand (or via CSV import) and then link it to SimpleFIN, any
transaction that falls inside the backfill window gets a second row: one manual, one
`simplefin`-sourced, same real-world event — inflating totals for that account until
someone cleans it up.

## Design considered and rejected: pre-import review gate

Original idea: before committing a SimpleFIN fetch to the DB, show the user a
preview (like the CSV importer's step 3) and let them deselect rows before they're
ever inserted.

**Why we dropped it:** the background scheduler (`maybe_sync` in `simplefin.rs`,
ticks every 30 min, imports automatically once due) has no concept of "the user
already reviewed and rejected this." A transaction deselected during review is
dated within the last few days — exactly the `OVERLAP_DAYS = 3` window every
subsequent sync re-fetches (`simplefin.rs:658`). With no persisted "don't re-import
this `simplefin_id`" tombstone, a rejected transaction just reappears on the next
automatic sync. Building the tombstone table would have made this workable, but it
adds real state to maintain for something the flipped design below avoids
entirely.

## Agreed design: post-import duplicate review

Let SimpleFIN import run exactly as it does today — nothing about `run_sync`,
`maybe_sync`, or `import_account_set` changes. After the fact, surface candidate
duplicate **pairs** (one row with `simplefin_id`, one without, same account,
matching amount/type, close date) in a review UI. The user unchecks false
positives; on submit, the non-`simplefin` side of each remaining checked pair gets
deleted (details below — the resolution isn't always "delete the manual side," see
the three buckets).

This sidesteps the tombstone problem completely: nothing is ever withheld from
import, so there's nothing for a later sync to reintroduce. The review can be run
on demand, repeatedly, safely — it's just cleanup, not a gate anything else depends
on.

### Trigger / placement

An on-demand "Review possible duplicates" action — e.g. a button in
`src/pages/settings/BankSync.vue` next to the existing "Sync now" button. Not
auto-triggered after every sync; the user runs it after linking a new account,
after using the disconnect/reconnect backfill trick (see below), or whenever they
suspect overlap.

## Candidate detection

New backend query (no existing code covers this). Match within the same
`account_id`:

- `amount` equal (or within ~0.005 float tolerance)
- `type` equal (`income`/`expense`) — don't match across sign
- `date` within a small tolerance window, not exact — SimpleFIN's `posted` date can
  lag a manually-entered purchase date by a day or two for card transactions.
  Reusing `OVERLAP_DAYS` (3) as the tolerance is a reasonable starting point, but
  it's a judgment call, not derived from anything else in the code.
- One side has `simplefin_id IS NOT NULL`, the other `IS NULL`
- Exclude `type = 'transfer'` rows entirely (see Edge cases below)

**Do not filter on description.** Unlike the CSV importer's `detectDuplicates`
(`src/lib/csv/mapping.ts:192`, keyed on `date|amount|description`), a strict
description match would miss real duplicates — a hand-typed "Starbucks" vs.
SimpleFIN's "STARBUCKS #123 SEATTLE WA" won't match on that key even though it's
the same purchase. Show both descriptions side by side in the review UI so the
human eye handles matching; the query's only job is recall (surface candidates),
not precision — false positives get unchecked by the user, false negatives just
never get flagged. Note from this session: the person requesting this feature
already does bank-formatted CSV imports rather than hand-typed manual entries, so
description drift may be less of an issue in practice than in the general case —
but the query shouldn't rely on that being true for every user.

## Resolution: three buckets, not one rule

This was the most important correction that came out of the session — a blanket
"delete the non-SimpleFIN side" default is wrong for paycheck-linked rows.

### 1. Ordinary transaction (no `paycheck_id`)

Default and expected case. Delete the non-`simplefin` row, keep the `simplefin`
row (it's the more durable source going forward — future syncs dedupe against it
via `simplefin_id`).

**Important: don't reuse `delete_transaction`/`delete_transaction_cmd` as-is for
this.** The existing path (`transactions.rs:647`) calls `clear_generated()` before
deleting the row, which explicitly `DELETE`s any `account_balance` snapshot the
transaction owns (`generated_balance_id`/`generated_balance_to_id`,
`delete_snapshot` at `transactions.rs:334`). That's correct for the app's normal
"delete a transaction" UX, but wrong here: SimpleFIN never backfills historical
daily balances — it only ever writes one current `balance`/`balance-date` pair per
account per sync (`simplefin.rs:599-600`, "No generated balance snapshots for
synced transactions — the bridge's own balance snapshot is the authoritative
anchor"). So a manual/CSV transaction's generated snapshot may be the **only**
historical balance data point you have for that date. Deleting it along with the
duplicate transaction would lose real information for no reason.

**Build a variant that skips `clear_generated`:** delete only the `txn` row, leave
its `account_balance` row untouched. This is safe long-term, not just a one-off
hack — `reproject_account` (`transactions.rs:459`) determines whether a snapshot
is "generated" (recomputed from a transaction) purely by whether some live
`txn.generated_balance_id`/`generated_balance_to_id` currently points at it
(`transactions.rs:500-527`, built fresh via SQL join every call, not a stored
flag). A snapshot absent from that map is treated as a manual anchor
(`transactions.rs:538-539`, "Manual snapshot: absolute anchor that resets the
running balance"). Once the owning transaction is deleted, the snapshot
automatically becomes a manual anchor from then on — no migration, no extra
column, no flag to set. And because its stored value doesn't change, **no call to
`reproject_accounts` is needed after this kind of delete** — unlike a normal
delete, which needs it because it removes both the transaction and its balance
effect.

### 2. Paycheck net-deposit duplicate (`paycheck_id` set, row lives on `income_account_id`)

`paycheck_id`-linked rows are the paycheck feature's actual generated ledger
entries, not just a dedup pointer — confirmed by reading `auto_create_income_txn`
and `auto_create_contributions` (`paychecks.rs:98-190`). Deleting the linked
transaction does **not** delete the parent `paycheck` record (one-directional FK,
`txn.paycheck_id → paycheck.id`), but it does remove that paycheck's actual
recorded deposit transaction.

Recommend excluding these from the default bulk "submit" action — flag them
distinctly in the UI rather than auto-checking them the same as an ordinary
row. Note the app already has a forward-looking fix for new paychecks:
`NewPaycheck.create_deposit_txn` (`paychecks.rs:30-35`) lets you skip
auto-creating the deposit row at paycheck-creation time if a matching
bank-imported transaction already exists. This duplicate-review feature mainly
matters for paychecks entered before SimpleFIN was connected.

### 3. Paycheck contribution duplicate (`paycheck_id` set, row lives on a 401(k)/HSA/etc. destination account)

**This is the one where the default resolution direction must be reversed.**
Deductions and employer-match amounts are inserted directly into their
destination account by `auto_create_contributions` (`paychecks.rs:98-150`) with
`is_contribution = 1` hardcoded. If that destination account is also linked to
SimpleFIN, the same real-world deposit gets imported a second time — but
`simplefin.rs:607` hardcodes `is_contribution = 0` (and `is_withdrawal = 0`) for
**every** SimpleFIN-imported transaction, unconditionally. There is no
account-type or category-based inference that would set it — `category_rules`
only ever populate the `category` column.

So for this bucket: **keep the paycheck-generated row, delete the SimpleFIN row**
instead. Applying the bucket-1 default here (keep SimpleFIN, delete manual) would
delete the row carrying `is_contribution=1` + `paycheck_id` and keep a row that
can never carry either — silently breaking IRS contribution-limit tracking
(`src/lib/contributions/`) for that paycheck, with no error or indication
anything went wrong.

Detecting this bucket: same amount/date/account matching as bucket 1, but check
whether the non-`simplefin` side has `paycheck_id IS NOT NULL` **and** its account
is a contribution-eligible type (retirement/HSA/etc. — whatever the app already
uses to distinguish these, check `src/lib/accountTypes.ts` and how
`contribution_account_type` is used in `paychecks.rs:108`).

## New backend pieces needed

1. **Candidate query command.** Given the matching rules above, return pairs with
   enough display data for the UI: date, amount, both descriptions, both ids,
   which bucket (ordinary / net-deposit / contribution) based on `paycheck_id` +
   destination account type.
2. **Delete-transaction-keep-snapshot variant.** Skips `clear_generated`, just
   `DELETE FROM txn WHERE id = ?1`. No reprojection call needed (see bucket 1
   above for why).
3. **Bulk variant (optional, perf only).** If a flagged list can get long, mirror
   the batching pattern from `bulk_create_transactions_with_snapshots`
   (`transactions.rs:705`) — one `execute_batch` instead of N round-trips. Not
   required to ship an MVP: looping the single-delete path via `Promise.all` from
   the frontend works fine for a typical handful of duplicates, and
   bucket-1/bucket-3 resolutions can reuse whichever delete primitive applies
   (existing `delete_transaction_cmd` for the "delete SimpleFIN row" case in
   bucket 3 — that row never owns a snapshot, so the existing delete path is
   already correct for it, no keep-snapshot variant needed there).

## New frontend pieces needed

Review UI following the `ImportWizard.vue` step-3 pattern (`ImportWizard.vue:692`):
checkbox table, default-checked (uncheck = "not actually a duplicate, keep both"),
columns for date/amount/both descriptions. No tri-state "balance-only" toggle
needed here (unlike `ImportWizard`'s import/balance-only/skip) — since bucket 1's
resolution never touches balance snapshots either way, plain checked/unchecked is
enough. Bucket 2 and 3 rows should probably render with a distinct visual
treatment (warning color / separate section) rather than sitting in the same list
as ordinary duplicates, given the different and higher-stakes resolution.

## Relevant background facts (already confirmed this session, don't re-derive)

- **Initial backfill window is smaller than it looks in practice.** The 90-day
  lookback (`FIRST_SYNC_LOOKBACK_DAYS`) only applies when `last_success_at IS
  NULL` (`simplefin.rs:658-661`). `simplefin_connect` fires an automatic sync
  immediately on connect (`simplefin.rs:764-775`), before any accounts are
  linked — that sync sets `last_success_at` regardless of whether anything was
  actually imported. So the "Sync now" you click after linking accounts (per the
  UI's own instruction, `BankSync.vue:252-255`) typically only fetches the
  3-day overlap window, not 90 days. Workaround, inferred from the code (not
  documented in the UI): `simplefin_disconnect` clears sync state but
  deliberately does not clear `account.simplefin_id` links
  (`simplefin.rs:812-819`). Link accounts first, then disconnect + reconnect
  with a fresh setup token — `last_success_at` resets to `NULL`, so the next
  automatic sync gets the full 90-day window against accounts that are already
  linked. This matters for how big a duplicate-candidate batch a user is likely
  to see.
- SimpleFIN writes exactly one `balance`/`balance-date` pair per account per
  sync (current balance only) — never a historical daily series
  (`simplefin.rs:599-600`).
- Balances between manual and SimpleFIN sources are lower-risk than
  transactions: they're not summed, just picked by recency
  (`src/lib/balances/recency.ts:21-24`, latest `recordedAt` then highest `id`
  wins), so a same-date manual + SimpleFIN snapshot pair doesn't double-count,
  it just means the SimpleFIN one (inserted later, higher id) becomes "current"
  for that date. Out of scope for this feature.

## Edge cases / explicitly out of scope for v1

- **Transfers.** A transfer between two of your accounts is stored as a single
  canonical row (owns two `generated_balance_id`/`generated_balance_to_id`
  snapshots), while SimpleFIN reports each side as two independent ordinary
  transactions in two different accounts. The shapes don't line up for simple
  pair-matching — exclude `type = 'transfer'` from the candidate query rather
  than trying to solve this now.
- **No persisted "not a duplicate" dismissal.** If a candidate pair is unchecked
  as a false positive, it will resurface every time the review is run again later
  — the query has no memory of past decisions. Not dangerous (nothing gets
  deleted or reimported), just a minor annoyance on repeat visits. A
  `dismissed_pairs` table (account_id + both txn ids or a stable pair key) would
  fix it; treat as optional polish, not required for v1.

## Related but separately scoped: SimpleFIN has no transfer concept

Discovered during this same planning session, but split into its own handoff —
[`simplefin-transfer-detection-handoff.md`](./simplefin-transfer-detection-handoff.md) —
rather than folded in here. It's a different kind of problem (cross-account
income/expense miscategorization that happens by default, vs. this doc's
same-account duplicate-row cleanup that the user opts into), with a different
mutation shape (collapsing two real rows into one canonical transfer, vs.
deleting a duplicate) and a different urgency profile (no existing manual
workaround, affects every multi-account user by default — arguably higher
priority than this doc's feature, and worth building first). See that doc for
the full scoping; don't bundle the two implementations.

## Suggested build order

1. Candidate query command (bucket classification included) — no UI yet, verify
   via a script/test against real data first since matching tolerances (date
   window especially) are a judgment call worth eyeballing before committing to.
2. Delete-keep-snapshot variant + its test (mirror the existing
   `reproject_accounts` tests' style, e.g. assert a snapshot survives and stays
   correctly anchored after its owning txn is deleted).
3. Review UI, bucket 1 only (ordinary duplicates), reusing `ImportWizard`'s
   table/checkbox pattern.
4. Extend to buckets 2/3 with the distinct visual treatment and reversed
   resolution for bucket 3.
