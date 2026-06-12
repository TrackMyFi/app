import type { FireAccount, FireBalance } from './types'
import { isLiability } from '../accountTypes'

export const fireNumber = (annualExpensesTarget: number) => annualExpensesTarget * 25

export function latestBalances(balances: FireBalance[]): Map<number, number> {
  const latestAt = new Map<number, string>()
  const value = new Map<number, number>()
  for (const b of balances) {
    const seen = latestAt.get(b.accountId)
    if (!seen || b.recordedAt > seen) { latestAt.set(b.accountId, b.recordedAt); value.set(b.accountId, b.balance) }
  }
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
