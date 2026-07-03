# Handoff: SimpleFIN has no transfer concept — cash-flow + contribution gap

> **Status: IMPLEMENTED (2026-07-03).** Automatic post-import pass
> (`collapse_transfer_pairs` in `src-tauri/src/simplefin.rs`), migration
> `0017_simplefin_transfers.sql`. Decisions on the open questions below:
> source (expense) row survives the collapse; deleted counterpart's SimpleFIN
> id is stored in `txn.simplefin_counterpart_id` so the import's NOT EXISTS
> guard blocks re-import from the overlap window; amount matching is exact
> (±0.005) — no fee tolerance; runs automatically after every import (full
> table scan, so history and late-arriving counterparts both collapse);
> Gap B included — `is_contribution = 1` when the destination is an
> investment-type account and the source is not (investment→investment
> moves are rollovers, not new principal).

Scoped in a planning session (no code written). Discovered while scoping
contribution-flagging for a taxable brokerage account, but this is a correctness
bug on its own, independent of that use case — treat as a pre-release blocker,
separate from and higher-priority than the SimpleFIN duplicate-review feature (see
[`simplefin-duplicate-review-handoff.md`](./simplefin-duplicate-review-handoff.md)).
That doc covers the *cleanup* problem (same event recorded twice in one account,
opt-in review, manual workaround already exists today). This doc covers a
*miscategorization* problem that happens silently by default for every user with
2+ SimpleFIN-linked accounts, with no existing workaround. Different mutation
shape, different blast radius, different urgency — don't bundle the implementation.

## Root cause

SimpleFIN import never produces a `type = 'transfer'` row. `import_account_set`
hardcodes `transfer_account_id = NULL` and derives `type` purely from the sign of
the amount:

```rust
let (ty, amount) = if signed < 0.0 { ("expense", -signed) } else { ("income", signed) };
```

(`src-tauri/src/simplefin.rs:587-588`, insert at `simplefin.rs:607`.)

SimpleFIN's protocol itself has no cross-account linkage — each linked account's
transaction feed is fetched and imported independently. So a real transfer
between two of your own SimpleFIN-linked accounts (checking → brokerage, checking
→ credit card payment, asset → liability, whatever) always arrives as **two
completely separate, unlinked ordinary rows**: an expense on the source account,
an income on the destination account. There's nothing in the SimpleFIN response
itself to key a transfer off of at fetch time — this has to be detected after the
fact by comparing rows across accounts.

## Gap A: cash-flow totals are inflated by internal transfers

`classifyFlow` (`src/lib/transactions/flow.ts:62`) only treats a transaction as a
neutral transfer when `t.type === 'transfer'`. Since SimpleFIN rows are never
typed that way, every internal transfer between two linked accounts counts as
full expense on one side and full income on the other in cash-flow totals.

- **Net worth stays correct** — each account's balance is independently anchored
  by SimpleFIN's own balance snapshot (`simplefin.rs:536-573`), unaffected by how
  the transaction that produced it is categorized.
- **Income/spending totals do not.** Monthly spending and income figures shown on
  the transactions page, budget breakdown, and annual chart (`flow.ts` is
  explicitly shared by all three, per its own doc comment) are overstated by the
  size of every internal transfer, for every user who links more than one
  account. This isn't an edge case — moving money between your own accounts
  (checking → savings, checking → brokerage, paying a credit card from checking)
  is a normal, common pattern.
- **No existing workaround.** Unlike the duplicate-review problem, there's
  currently no UI path for a user to notice and fix this themselves — recognizing
  "these two independent transactions across two accounts are actually one
  transfer" and manually converting them (edit one to `type = 'transfer'` +
  `transfer_account_id`, delete the other) isn't something the existing
  transaction-edit UI is built for.

## Gap B: can't auto-flag brokerage (or other investment) transfers as contributions without this

Raised originally as: "would transfers into my taxable brokerage get flagged as
contributions?" Short answer is no today, and it can't be done safely until Gap A
is solved.

`brokerage` is already an `isInvestment` account type and already appears in the
contributions breakdown (`src/lib/contributions/index.ts:41`, labeled "Brokerage",
no IRS `limitFor` — contributions in this app aren't IRS-limit-only, brokerage is
tracked the same way retirement/HSA accounts are). So flagging a transfer-in as
`is_contribution = 1` is consistent with how the app already models contributions.

But you can't safely set that flag on a SimpleFIN row without first knowing it's
actually a transfer-in from one of the user's own accounts, as opposed to organic
income (e.g. a coincidental deposit, interest, a rebate). That determination is
exactly Gap A's unsolved detection problem. Build order matters: Gap A has to be
solved (or at least its detection logic built) before Gap B is meaningful.

## Suggested direction

A post-sync pass, reusing the technique `detectTransferCounterparts` already uses
for CSV import (`src/lib/csv/mapping.ts:226`):

- Match by amount (± float tolerance) and a date tolerance window (the CSV
  version defaults to `TRANSFER_DATE_TOLERANCE_DAYS = 3`,
  `src/lib/csv/mapping.ts:208` — reasonable starting point here too).
- **Deliberately ignore description.** The two sides of a real transfer almost
  never share wording (bank-reported description on the source side vs.
  brokerage/destination-reported description on the other), same reasoning as
  the duplicate-review doc's candidate query.

Key difference from the CSV version: `detectTransferCounterparts` runs at
CSV-import time, matching *unimported* rows against *already-existing* DB
transfers filtered by `type = 'transfer'`. This needs to run **after** SimpleFIN
import, matching two **already-inserted**, independent `income`/`expense` rows
across two of the user's own linked accounts against each other — a different
data shape, not a direct call-site reuse, even though the matching math is the
same idea.

On a match, **collapse the pair into the app's canonical single-row transfer
model** rather than leaving two independent rows: pick one row to keep, set its
`type = 'transfer'` and `transfer_account_id` to point at the other account,
delete the other row. Only after that collapse is it safe to also set
`is_contribution = 1` on the kept row when the destination side is an
investment-type account (Gap B).

Open questions to resolve during implementation, not answered in this scoping
session:

- Which side's row survives the collapse (source or destination), and does it
  matter which account's `simplefin_id`/description gets kept for provenance?
- What if only one side of a real transfer is on a SimpleFIN-linked account (the
  other is untracked, or manual/CSV) — that's not a detectable pair at all under
  this design, and would need to stay as an ordinary income/expense row. Worth
  being explicit in the UI/docs that this only fixes transfers between two
  *linked* accounts.
- Rounding/fee differences: does a wire transfer or ACH sometimes report slightly
  different amounts on each side (fees deducted)? If so the exact-amount match
  needs a small tolerance too, not just the date window.
- Trigger/placement: most naturally another on-demand post-sync pass, similar in
  spirit and possibly similar UI treatment to the duplicate-review action — but
  worth deciding whether this one should run automatically (since it's a
  default-behavior correctness bug, not opt-in cleanup) rather than requiring the
  user to remember to trigger it.

## Explicitly not solved here: dividend reinvestment

Recommend against auto-flagging reinvested dividends as contributions, even once
transfer detection exists for genuine transfers-in. This is a FIRE-methodology
concern, not just a technical one: `trailingMonthlyContribution`
(`src/lib/fire/contributionRate.ts:12`) sums `isContribution` transactions
directly into the monthly contribution rate that feeds the FIRE forecast and
coast-FIRE math (`src/lib/fire/`). Reinvested dividends are market growth, not
new principal — counting them as contributions would double-count that growth:
once implicitly via whatever return assumption the forecast applies to net worth,
and again as if it were fresh money actively added, inflating the apparent
savings rate and distorting the forward projection.

There's also no reliable protocol-level signal to detect it — it would mean
pattern-matching bank/brokerage-specific description text ("REINVESTED DIVIDEND"
or similar), which varies by institution and isn't something SimpleFIN
standardizes. If wanted later, treat as an opt-in keyword rule the user curates
themselves (similar shape to `category_rules`), not a default behavior.
