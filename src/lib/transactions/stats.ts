import type { CashFlowTotals } from './flow'
import type { PeriodStats } from '../types/PeriodStats'

export type { PeriodStats }

export function median(arr: number[]): number {
  if (arr.length === 0) return 0
  const sorted = [...arr].sort((a, b) => a - b)
  const mid = Math.floor(sorted.length / 2)
  return sorted.length % 2 !== 0 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2
}

export interface PeriodMedians {
  /** Number of reference periods used (already excludes the current one). */
  periodCount: number
  totals: CashFlowTotals
  breakdown: {
    income: number
    byCategory: Map<string, number>
  }
}

/**
 * Compute median cash-flow stats from an array of pre-aggregated period rows.
 *
 * Each row is one calendar period (month or year) returned by `period_stats_cmd`.
 * The current period has already been excluded by the Rust command so a period
 * is never compared against itself.
 */
export function computeMedian(periods: PeriodStats[]): PeriodMedians | null {
  if (periods.length === 0) return null

  const medianIncome = median(periods.map((p) => p.income))

  const byCategory = new Map<string, number>()
  byCategory.set('savings', median(periods.map((p) => p.savings)))
  byCategory.set('fixed', median(periods.map((p) => p.catFixed)))
  byCategory.set('discretionary', median(periods.map((p) => p.catDiscretionary)))
  byCategory.set('uncategorized', median(periods.map((p) => p.catUncategorized)))

  return {
    periodCount: periods.length,
    totals: {
      income: medianIncome,
      expense: median(periods.map((p) => p.expense)),
      savings: median(periods.map((p) => p.savings)),
      net: median(periods.map((p) => p.net)),
    },
    breakdown: {
      income: medianIncome,
      byCategory,
    },
  }
}
