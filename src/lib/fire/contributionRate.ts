import { DateTime } from 'luxon'
import type { Transaction } from '../types/Transaction'

function cutoffIso(asOfIso: string): string {
  return DateTime.fromISO(asOfIso).minus({ months: 12 }).toISODate()!
}

/**
 * Average monthly contribution over the trailing 12 months ending at `asOfIso`.
 * Sums `isContribution` txns with `cutoff < date <= asOf`, divided by 12.
 */
export function trailingMonthlyContribution(txns: Transaction[], asOfIso: string): number {
  const cutoff = cutoffIso(asOfIso)
  let total = 0
  for (const t of txns) {
    if (!t.isContribution) continue
    // Withdrawals net the trailing contribution rate down.
    if (t.date > cutoff && t.date <= asOfIso) total += t.isWithdrawal ? -t.amount : t.amount
  }
  return total / 12
}

/**
 * True when at least one contribution exists at or before the 12-month cutoff,
 * i.e. the trailing-12-month window captures a full year of contribution history.
 */
export function hasFullYearOfContributions(txns: Transaction[], asOfIso: string): boolean {
  const cutoff = cutoffIso(asOfIso)
  return txns.some(t => t.isContribution && t.date <= cutoff)
}

export interface DerivedContribution {
  monthly: number
  estimated: boolean
}

/**
 * Derived monthly contribution baseline. Uses the trailing-12-month actual
 * average when a full year of contribution history exists; otherwise returns the
 * caller-supplied `estimateMonthly` (the Phase 1 savings-rate approximation) and
 * flags `estimated: true`.
 */
export function derivedMonthlyContribution(
  txns: Transaction[], asOfIso: string, estimateMonthly: number,
): DerivedContribution {
  if (hasFullYearOfContributions(txns, asOfIso)) {
    return { monthly: trailingMonthlyContribution(txns, asOfIso), estimated: false }
  }
  return { monthly: estimateMonthly, estimated: true }
}
