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
export type FlowBucket = 'savings' | 'fixed' | 'discretionary' | 'uncategorized'

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

function accountType(id: number | null, accounts: AccountLookup): string {
  if (id == null) return ''
  const acct = accounts instanceof Map
    ? accounts.get(id)
    : accounts.find((a) => a.id === id)
  return acct?.type ?? ''
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

  // Visual direction is independent of the accounting buckets below.
  let direction: FlowDirection
  if (t.type === 'income') direction = 'inflow'
  else if (t.type === 'expense') direction = 'outflow'
  else direction = transferDirection(
    accountType(t.accountId, accounts),
    accountType(t.transferAccountId, accounts),
  )

  // Contributions always count as savings, whatever their underlying type.
  if (t.isContribution) {
    return { direction, isTransfer, inflow: 0, outflow: t.amount, bucket: 'savings', isSavings: true }
  }

  if (t.type === 'income') {
    return { direction, isTransfer, inflow: t.amount, outflow: 0, bucket: null, isSavings: false }
  }

  if (t.type === 'expense') {
    const bucket = (t.category || 'uncategorized') as FlowBucket
    return { direction, isTransfer, inflow: 0, outflow: t.amount, bucket, isSavings: false }
  }

  // Transfers: only debt payments and refunds move spendable cash.
  if (direction === 'outflow') {
    return { direction, isTransfer, inflow: 0, outflow: t.amount, bucket: 'uncategorized', isSavings: false }
  }
  if (direction === 'inflow') {
    return { direction, isTransfer, inflow: t.amount, outflow: 0, bucket: null, isSavings: false }
  }
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
