import type { FireAccount, FireBalance } from './types'
import { isLiability } from '../accountTypes'
import { isNewer } from '../balances/recency'
import { monthsToFire, realMonthlyReturn } from './projection'

export const fireNumber = (annualExpensesTarget: number) => annualExpensesTarget * 25

export function latestBalances(balances: FireBalance[]): Map<number, number> {
  const latest = new Map<number, FireBalance>()
  for (const b of balances) {
    const seen = latest.get(b.accountId)
    if (!seen || isNewer(b, seen)) latest.set(b.accountId, b)
  }
  const value = new Map<number, number>()
  for (const [accountId, b] of latest) value.set(accountId, b.balance)
  return value
}

export function currentNetWorth(accounts: FireAccount[], balances: FireBalance[]): number {
  const latest = latestBalances(balances)
  let total = 0
  for (const a of accounts) { const bal = latest.get(a.id) ?? 0; total += isLiability(a.type) ? -bal : bal }
  return total
}

export function investableNetWorth(accounts: FireAccount[], balances: FireBalance[]): number {
  const latest = latestBalances(balances)
  let total = 0
  for (const a of accounts) {
    if (!a.includeInFireCalculations) continue
    const bal = latest.get(a.id) ?? 0
    total += isLiability(a.type) ? -bal : bal
  }
  return total
}

export const fiProgress = (investable: number, fireNum: number) =>
  fireNum === 0 ? 0 : (investable / fireNum) * 100

/**
 * What % of the TIME journey from $0 to the FIRE number has elapsed.
 * Because compounding accelerates growth, this is always >= fiProgress
 * once any balance exists — e.g. $500k/$1M at 7% + $2k/mo ≈ 66%, not 50%.
 */
export function journeyProgress(
  presentValue: number,
  monthlyContribution: number,
  expectedReturnRate: number,
  inflationRate: number,
  target: number,
): number | null {
  if (target <= 0) return null
  if (presentValue >= target) return 100
  const r = realMonthlyReturn(expectedReturnRate, inflationRate)
  const total = monthsToFire(0, monthlyContribution, r, target)
  if (!total) return null
  const remaining = monthsToFire(presentValue, monthlyContribution, r, target)
  if (remaining === null) return null
  return ((total - remaining) / total) * 100
}

/** Nominal dollar growth the portfolio generates on its own each month. */
export const portfolioMonthlyEarnings = (presentValue: number, annualReturnRate: number) =>
  presentValue * (Math.pow(1 + annualReturnRate, 1 / 12) - 1)
