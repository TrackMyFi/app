import type { FireAccount, FireBalance } from './types'
import { isLiability, isEquity } from '../accountTypes'
import { byRecencyAsc } from '../balances/recency'

export interface NetWorthPoint {
  date: string
  netWorth: number
  /** Net worth excluding real estate equity (real estate assets + mortgage). Null when no real estate balances exist. */
  lessEquity: number | null
  /** Sum of checking + savings balances. */
  liquid: number
  /** netWorth minus liquid. */
  illiquid: number
}

const LIQUID_TYPES = new Set(['checking', 'savings', 'brokerage', 'crypto'])

export interface DrawdownStatus {
  high: number
  highDate: string
  /** Fraction below the all-time high; 0 when at it. */
  drawdown: number
  atHigh: boolean
}

/** Where current net worth sits relative to its all-time high. Null without a positive high. */
export function drawdownStatus(points: NetWorthPoint[]): DrawdownStatus | null {
  if (points.length === 0) return null
  let high = -Infinity
  let highDate = ''
  for (const p of points) {
    if (p.netWorth > high) { high = p.netWorth; highDate = p.date }
  }
  if (high <= 0) return null
  const current = points[points.length - 1].netWorth
  return {
    high,
    highDate,
    drawdown: Math.max(0, (high - current) / high),
    atHigh: current >= high,
  }
}

export function netWorthOverTime(accounts: FireAccount[], balances: FireBalance[]): NetWorthPoint[] {
  if (balances.length === 0) return []
  const typeById = new Map(accounts.map(a => [a.id, a.type]))
  const dates = [...new Set(balances.map(b => b.recordedAt))].sort()
  const sorted = [...balances].sort(byRecencyAsc)

  const hasRealEstateBalance = balances.some(b => typeById.get(b.accountId) === 'real_estate')

  return dates.map(date => {
    const latestForAccount = new Map<number, number>()
    for (const b of sorted) { if (b.recordedAt <= date) latestForAccount.set(b.accountId, b.balance) }

    let netWorth = 0
    let lessEquity = 0
    let liquid = 0

    for (const [accountId, bal] of latestForAccount) {
      const type = typeById.get(accountId) ?? ''
      const signed = isLiability(type) ? -bal : bal
      netWorth += signed
      if (!isEquity(type)) lessEquity += signed
      if (LIQUID_TYPES.has(type)) liquid += bal
    }

    return {
      date,
      netWorth,
      lessEquity: hasRealEstateBalance ? lessEquity : null,
      liquid,
      illiquid: netWorth - liquid,
    }
  })
}
