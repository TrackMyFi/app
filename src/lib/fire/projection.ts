import { DateTime } from 'luxon'
import type { FireAccount, FireBalance } from './types'
import { isInvestment } from '../accountTypes'
import { isNewer } from '../balances/recency'

const MAX_MONTHS = 1200 // 100-year cap

export function realMonthlyReturn(expectedReturnRate: number, inflationRate: number): number {
  return Math.pow((1 + expectedReturnRate) / (1 + inflationRate), 1 / 12) - 1
}

export function monthsToFire(
  presentValue: number, monthlyContribution: number, monthlyReturn: number, target: number,
): number | null {
  if (presentValue >= target) return 0
  let fv = presentValue
  for (let m = 1; m <= MAX_MONTHS; m++) {
    fv = fv * (1 + monthlyReturn) + monthlyContribution
    if (fv >= target) return m
  }
  return null
}

export function projectedFiDate(
  presentValue: number, monthlyContribution: number,
  expectedReturnRate: number, inflationRate: number, target: number,
  from: DateTime = DateTime.now(),
): DateTime | null {
  const months = monthsToFire(presentValue, monthlyContribution, realMonthlyReturn(expectedReturnRate, inflationRate), target)
  return months === null ? null : from.plus({ months })
}

function investmentBalanceAt(accounts: FireAccount[], balances: FireBalance[], isoDate: string): number {
  const invest = new Set(accounts.filter(a => isInvestment(a.type)).map(a => a.id))
  const latest = new Map<number, FireBalance>()
  for (const b of balances) {
    if (!invest.has(b.accountId) || b.recordedAt > isoDate) continue
    const seen = latest.get(b.accountId)
    if (!seen || isNewer(b, seen)) latest.set(b.accountId, b)
  }
  let total = 0
  for (const b of latest.values()) total += b.balance
  return total
}

export function savingsRate(
  accounts: FireAccount[], balances: FireBalance[], annualIncome: number, asOfIso: string,
): number {
  if (annualIncome === 0) return 0
  const now = investmentBalanceAt(accounts, balances, asOfIso)
  const yearAgoIso = DateTime.fromISO(asOfIso).minus({ years: 1 }).toISODate()!
  const prior = investmentBalanceAt(accounts, balances, yearAgoIso)
  return (now - prior) / annualIncome
}

export interface ProjectionPoint { date: string; value: number }

/**
 * Month-by-month projected investable value, `months + 1` points (index 0 = the
 * present value at `from`). Each subsequent month compounds at the monthly real
 * return then adds the monthly contribution.
 */
export function projectionSeries(
  presentValue: number, monthlyContribution: number,
  expectedReturnRate: number, inflationRate: number,
  months: number, from: DateTime = DateTime.now(),
): ProjectionPoint[] {
  const mr = realMonthlyReturn(expectedReturnRate, inflationRate)
  const pts: ProjectionPoint[] = []
  let fv = presentValue
  for (let m = 0; m <= months; m++) {
    if (m > 0) fv = fv * (1 + mr) + monthlyContribution
    pts.push({ date: from.plus({ months: m }).toISODate()!, value: fv })
  }
  return pts
}
