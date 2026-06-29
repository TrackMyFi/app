import type { FireAccount, FireBalance } from './types'
import { isInvestment } from '../accountTypes'
import { byRecencyAsc } from '../balances/recency'

export interface InvestmentPoint {
  date: string
  total: number
  byAccount: Record<number, number>
}

export interface InvestmentsOverTime {
  points: InvestmentPoint[]
  /** Ordered list of investment account IDs that appear in the series. */
  accountIds: number[]
}

export function investmentsOverTime(
  accounts: FireAccount[],
  balances: FireBalance[],
): InvestmentsOverTime {
  const investmentIds = new Set(accounts.filter(a => isInvestment(a.type)).map(a => a.id))
  const investmentBalances = balances.filter(b => investmentIds.has(b.accountId))

  if (investmentBalances.length === 0) return { points: [], accountIds: [] }

  const dates = [...new Set(investmentBalances.map(b => b.recordedAt))].sort()
  const sorted = [...investmentBalances].sort(byRecencyAsc)

  const points: InvestmentPoint[] = dates.map(date => {
    const latestForAccount = new Map<number, number>()
    for (const b of sorted) { if (b.recordedAt <= date) latestForAccount.set(b.accountId, b.balance) }

    let total = 0
    const byAccount: Record<number, number> = {}
    for (const [accountId, bal] of latestForAccount) {
      total += bal
      byAccount[accountId] = bal
    }

    return { date, total, byAccount }
  })

  // Preserve stable ordering: accounts sorted by their first appearance in the data.
  const seen = new Set<number>()
  const accountIds: number[] = []
  for (const b of sorted) {
    if (!seen.has(b.accountId)) { seen.add(b.accountId); accountIds.push(b.accountId) }
  }

  return { points, accountIds }
}
