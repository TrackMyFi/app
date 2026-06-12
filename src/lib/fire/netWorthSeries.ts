import type { FireAccount, FireBalance } from './types'
import { isLiability } from '../accountTypes'

export interface NetWorthPoint { date: string; netWorth: number }

export function netWorthOverTime(accounts: FireAccount[], balances: FireBalance[]): NetWorthPoint[] {
  if (balances.length === 0) return []
  const typeById = new Map(accounts.map(a => [a.id, a.type]))
  const dates = [...new Set(balances.map(b => b.recordedAt))].sort()
  const sorted = [...balances].sort((a, b) => a.recordedAt.localeCompare(b.recordedAt))

  return dates.map(date => {
    const latestForAccount = new Map<number, number>()
    for (const b of sorted) { if (b.recordedAt <= date) latestForAccount.set(b.accountId, b.balance) }
    let netWorth = 0
    for (const [accountId, bal] of latestForAccount) {
      netWorth += isLiability(typeById.get(accountId) ?? '') ? -bal : bal
    }
    return { date, netWorth }
  })
}
