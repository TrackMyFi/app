import { DateTime } from 'luxon'
import { realMonthlyReturn, monthsToFire } from './projection'

/** Annual real return implied by the monthly real return basis used in projections. */
export function realAnnualReturn(expectedReturnRate: number, inflationRate: number): number {
  return Math.pow(1 + realMonthlyReturn(expectedReturnRate, inflationRate), 12) - 1
}

/**
 * Investable net worth needed today so it compounds to `fireNumber` by retirement
 * with zero further contributions. When already at/past retirement age, the coast
 * number is the full FIRE number (no time left to grow).
 */
export function coastFireNumber(
  fireNumber: number, currentAge: number, targetRetirementAge: number,
  expectedReturnRate: number, inflationRate: number,
): number {
  const years = targetRetirementAge - currentAge
  if (years <= 0) return fireNumber
  const r = realAnnualReturn(expectedReturnRate, inflationRate)
  return fireNumber / Math.pow(1 + r, years)
}

export interface CoastStatus {
  coasting: boolean
  coastNumber: number
  /** Date the current trajectory crosses the coast number; null if coasting or unreachable. */
  crossingDate: DateTime | null
}

export function coastStatus(
  investable: number, monthlyContribution: number, fireNumber: number,
  currentAge: number, targetRetirementAge: number,
  expectedReturnRate: number, inflationRate: number,
  from: DateTime = DateTime.now(),
): CoastStatus {
  const coastNumber = coastFireNumber(fireNumber, currentAge, targetRetirementAge, expectedReturnRate, inflationRate)
  if (investable >= coastNumber) return { coasting: true, coastNumber, crossingDate: null }
  const months = monthsToFire(
    investable, monthlyContribution,
    realMonthlyReturn(expectedReturnRate, inflationRate), coastNumber,
  )
  return { coasting: false, coastNumber, crossingDate: months === null ? null : from.plus({ months }) }
}
