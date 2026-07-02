import type { DateTime } from 'luxon'
import { latestSnapshot } from './recency'

export interface BalanceChange {
  amount: number
  /** Fractional change vs. the prior balance's magnitude; null when the prior balance was 0. */
  percent: number | null
}

/** Minimal shape needed to compute historical changes; matches recency.ts's RecencyKey convention. */
export interface ChangeBalance {
  accountId: number
  balance: number
  recordedAt: string
  id?: number
}

function balanceAsOf(balances: ChangeBalance[], accountId: number, asOfDate: string): number | null {
  const candidates = balances.filter(b => b.accountId === accountId && b.recordedAt <= asOfDate)
  return latestSnapshot(candidates)?.balance ?? null
}

function changeSince(current: number, prior: number | null): BalanceChange | null {
  if (prior == null) return null
  const amount = current - prior
  const percent = prior !== 0 ? amount / Math.abs(prior) : null
  return { amount, percent }
}

/** Change vs. the account's latest balance recorded on/before the last day of the prior month. */
export function monthChange(
  balances: ChangeBalance[], accountId: number, current: number, now: DateTime,
): BalanceChange | null {
  const asOf = now.startOf('month').minus({ days: 1 }).toISODate()!
  return changeSince(current, balanceAsOf(balances, accountId, asOf))
}

/** Change vs. the account's latest balance recorded on/before the last day of the prior year. */
export function yearToDateChange(
  balances: ChangeBalance[], accountId: number, current: number, now: DateTime,
): BalanceChange | null {
  const asOf = now.startOf('year').minus({ days: 1 }).toISODate()!
  return changeSince(current, balanceAsOf(balances, accountId, asOf))
}
