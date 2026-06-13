import type { FireAccount, FireBalance } from './types'
import { isLiability } from '../accountTypes'
import { isNewer } from '../balances/recency'

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
