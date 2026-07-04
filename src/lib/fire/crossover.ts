import { DateTime } from 'luxon'
import { portfolioMonthlyEarnings } from './metrics'

const MAX_MONTHS = 1200 // 100-year cap, same as projections

export interface CrossoverStatus {
  /** True once the portfolio's monthly earnings meet or exceed the monthly contribution. */
  crossed: boolean
  /** Projected crossing date; null if already crossed or unreachable. */
  date: DateTime | null
}

/**
 * The crossover point: when the portfolio starts out-earning what its owner
 * contributes each month. Deliberately computed in NOMINAL terms (unlike FI
 * projections, which use real returns) so it agrees with the displayed
 * "portfolio earns per month" figure, which is nominal.
 */
export function crossoverStatus(
  investable: number, monthlyContribution: number, expectedReturnRate: number,
  from: DateTime = DateTime.now(),
): CrossoverStatus {
  const r = Math.pow(1 + expectedReturnRate, 1 / 12) - 1
  if (investable > 0 && portfolioMonthlyEarnings(investable, expectedReturnRate) >= monthlyContribution) {
    return { crossed: true, date: null }
  }
  if (r <= 0 || (investable <= 0 && monthlyContribution <= 0)) return { crossed: false, date: null }
  let fv = Math.max(0, investable)
  for (let m = 1; m <= MAX_MONTHS; m++) {
    fv = fv * (1 + r) + monthlyContribution
    if (fv * r >= monthlyContribution) return { crossed: false, date: from.plus({ months: m }) }
  }
  return { crossed: false, date: null }
}
