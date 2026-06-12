import { DateTime } from 'luxon'
import type { FireAccount, FireBalance } from './types'
import { isInvestment } from '../accountTypes'

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
  const latest = new Map<number, { at: string; bal: number }>()
  for (const b of balances) {
    if (!invest.has(b.accountId) || b.recordedAt > isoDate) continue
    const seen = latest.get(b.accountId)
    if (!seen || b.recordedAt > seen.at) latest.set(b.accountId, { at: b.recordedAt, bal: b.balance })
  }
  let total = 0
  for (const { bal } of latest.values()) total += bal
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
