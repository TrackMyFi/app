import { isLiability } from '../accountTypes'
import type { Transaction } from '../types/Transaction'
import type { Account } from '../types/Account'

/**
 * Cash-flow classification for a transaction, shared by the transactions page,
 * the monthly breakdown, and the annual chart so all three agree on what counts
 * as income, spending, and savings.
 *
 * Two concerns are kept separate:
 *  - `direction` is purely visual (which arrow/colour to show next to the amount).
 *  - `inflow`/`outflow`/`bucket`/`isSavings` drive the totals and breakdown.
 *
 * A contribution (`isContribution`) is always treated as savings — an outflow of
 * spendable cash into the savings bucket — regardless of its underlying type.
 * This matches how the budget module treats contributions.
 */

export type FlowDirection = 'inflow' | 'outflow' | 'neutral'
export type FlowBucket = 'savings' | 'fixed' | 'discretionary' | 'irregular' | 'uncategorized'

export interface TransactionFlow {
  /** Visual direction for the amount prefix icon/colour. */
  direction: FlowDirection
  /** True when the underlying transaction is a transfer (drives the ⇄ glyph). */
  isTransfer: boolean
  /** Amount counted as money coming in. */
  inflow: number
  /** Amount counted as money going out (spending or savings). */
  outflow: number
  /** Bucket an outflow belongs to; null for inflows and neutral transfers. */
  bucket: FlowBucket | null
  /** True when this represents a savings/investment contribution. */
  isSavings: boolean
}

type AccountLookup = Map<number, Account> | Account[]

function lookupAccount(id: number | null, accounts: AccountLookup): Account | undefined {
  if (id == null) return undefined
  return accounts instanceof Map ? accounts.get(id) : accounts.find((a) => a.id === id)
}

function accountType(id: number | null, accounts: AccountLookup): string {
  return lookupAccount(id, accounts)?.type ?? ''
}

/**
 * Direction of a transfer from the source account's perspective, based on the
 * asset/liability nature of each side:
 *  - asset → asset (or liability → liability) = neutral (moving money around)
 *  - asset → liability                        = outflow (paying down debt)
 *  - liability → asset                        = inflow  (e.g. a refund/credit)
 */
export function transferDirection(srcType: string, dstType: string): FlowDirection {
  const srcLiab = isLiability(srcType)
  const dstLiab = isLiability(dstType)
  if (srcLiab === dstLiab) return 'neutral'
  return dstLiab ? 'outflow' : 'inflow'
}

export function classifyFlow(t: Transaction, accounts: AccountLookup): TransactionFlow {
  const isTransfer = t.type === 'transfer'

  // Rule-suppressed noise (investment activity, fees inside a 401k, …) is real
  // money movement within the account but not income or spending — fully
  // neutral here; balance math still sees the row.
  if (t.suppressedAs) {
    return { direction: 'neutral', isTransfer, inflow: 0, outflow: 0, bucket: null, isSavings: false }
  }

  // Visual direction is independent of the accounting buckets below.
  let direction: FlowDirection
  if (t.type === 'income') direction = 'inflow'
  else if (t.type === 'expense') direction = 'outflow'
  else direction = transferDirection(
    accountType(t.accountId, accounts),
    accountType(t.transferAccountId, accounts),
  )

  // Contributions always count as savings, whatever their underlying type. A
  // withdrawal is dis-saving — it nets the savings bucket down via a negative
  // outflow, so savings rate reflects money pulled back out of investments.
  //
  // An income-type contribution is money that arrived in the account without
  // passing through spendable cash — a pre-tax paycheck deduction (401k, HSA)
  // or employer match. It's counted as income AND savings, otherwise the
  // breakdown would show gross-funded savings consuming net income.
  if (t.isContribution) {
    const outflow = t.isWithdrawal ? -t.amount : t.amount
    const inflow = t.type === 'income' && !t.isWithdrawal ? t.amount : 0
    return { direction, isTransfer, inflow, outflow, bucket: 'savings', isSavings: true }
  }

  // A refund reverses an earlier expense. The money genuinely arrived (income
  // type keeps balance math correct), but counting it as income would inflate
  // earnings while the original expense still inflates spending. Instead it's
  // a negative outflow in its category bucket, netting the expense back out —
  // the same trick the withdrawal branch above uses for the savings bucket.
  if (t.type === 'income' && t.isRefund) {
    const bucket = (t.category || 'uncategorized') as FlowBucket
    return { direction, isTransfer, inflow: 0, outflow: -t.amount, bucket, isSavings: false }
  }

  if (t.type === 'income') {
    return { direction, isTransfer, inflow: t.amount, outflow: 0, bucket: null, isSavings: false }
  }

  if (t.type === 'expense') {
    const bucket = (t.category || 'uncategorized') as FlowBucket
    return { direction, isTransfer, inflow: 0, outflow: t.amount, bucket, isSavings: false }
  }

  // Transfers are normally cash-flow neutral — the economic event (income or expense)
  // is captured on the individual transactions themselves. Counting a credit card
  // payment as an expense would double-count every purchase already recorded against
  // the card. Exception: a destination account flagged countPaymentsAsExpense (a
  // mortgage, a car loan) records no purchases of its own, so the payment IS the
  // expense — count the full amount as spending, defaulting to the fixed bucket
  // (a loan payment is a fixed obligation).
  const dest = lookupAccount(t.transferAccountId, accounts)
  if (dest?.countPaymentsAsExpense) {
    const bucket: FlowBucket = t.category === 'discretionary' ? 'discretionary' : 'fixed'
    return { direction, isTransfer, inflow: 0, outflow: t.amount, bucket, isSavings: false }
  }

  // The visual `direction` arrow in the table is still driven by the
  // asset/liability classification above; only the accounting values are zeroed.
  return { direction, isTransfer, inflow: 0, outflow: 0, bucket: null, isSavings: false }
}

export interface CashFlowTotals {
  income: number
  /** Spending only — excludes savings/contributions. */
  expense: number
  savings: number
  /** Income minus spending. Savings does not reduce net (the money is still yours). */
  net: number
}

export function cashFlowTotals(txns: Transaction[], accounts: AccountLookup): CashFlowTotals {
  let income = 0
  let expense = 0
  let savings = 0
  for (const t of txns) {
    const f = classifyFlow(t, accounts)
    income += f.inflow
    if (f.isSavings) savings += f.outflow
    else expense += f.outflow
  }
  return { income, expense, savings, net: income - expense }
}

/** Savings rate as a fraction of income (Savings ÷ Income), or null when undefined. */
export function savingsRate(totals: CashFlowTotals): number | null {
  if (totals.income <= 0) return null
  return totals.savings / totals.income
}
