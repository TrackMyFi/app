import { DateTime } from 'luxon'
import type { Paycheck } from '../types/Paycheck'

export interface ContributionPreviewItem {
  label: string
  amount: number
  accountId: number
}

interface DeductionLike {
  label: string
  amount: number
  contributionAccountType?: string | null
  accountId?: number | null
}

interface MatchLike {
  label: string
  amount: number
  accountId?: number | null
}

export function contributionItems(
  deductions: DeductionLike[],
  employerMatch: MatchLike[],
): ContributionPreviewItem[] {
  const items: ContributionPreviewItem[] = []
  for (const d of deductions) {
    if (d.contributionAccountType != null && d.accountId != null) {
      items.push({ label: d.label, amount: d.amount, accountId: d.accountId })
    }
  }
  for (const m of employerMatch) {
    if (m.accountId != null) {
      items.push({ label: m.label, amount: m.amount, accountId: m.accountId })
    }
  }
  return items
}

export interface ExistingTxnRef {
  id: number
  amount: number
  date: string
  description: string
  type: string
  paycheckId: number | null
}

/** Default ± window (in days) for matching a paycheck deposit against a bank posting date. */
export const PAYCHECK_DEPOSIT_TOLERANCE_DAYS = 3

/**
 * Find an existing, non-paycheck-linked income transaction on the deposit
 * account that looks like it's already this paycheck's bank-side deposit
 * (e.g. imported from a CSV before the paycheck was entered). Matches on
 * amount + a date window, ignoring description, same approach as the CSV
 * import wizard's duplicate detection.
 *
 * Only ever matches income rows with `paycheckId == null` — a transaction
 * already linked to a paycheck (including the one being edited, and any
 * pre-tax contribution rows) can never be flagged.
 */
export function findDuplicateDeposit(
  candidate: { amount: number; date: string },
  existing: ExistingTxnRef[],
  toleranceDays = PAYCHECK_DEPOSIT_TOLERANCE_DAYS,
): ExistingTxnRef | null {
  const cDate = DateTime.fromISO(candidate.date)
  const match = existing.find(
    (t) =>
      t.type === 'income' &&
      t.paycheckId == null &&
      Math.abs(t.amount - candidate.amount) < 0.005 &&
      Math.abs(DateTime.fromISO(t.date).diff(cDate, 'days').days) <= toleranceDays,
  )
  return match ?? null
}

export function paycheckTotals(paychecks: Paycheck[]): {
  totalGross: number
  totalNet: number
  count: number
} {
  let totalGross = 0
  let totalNet = 0
  for (const p of paychecks) {
    totalGross = Math.round((totalGross + p.grossAmount) * 100) / 100
    totalNet = Math.round((totalNet + p.netAmount) * 100) / 100
  }
  return { totalGross, totalNet, count: paychecks.length }
}

const cents = (n: number) => Math.round(n * 100) / 100

export interface PaycheckBreakdown {
  totalGross: number
  totalNet: number
  /** Sum of the five withheld tax fields across every paycheck. */
  totalTaxes: number
  /** Sum of every line-item deduction (401k, health, etc.). */
  totalDeductions: number
  /** Everything not paid out as net pay: gross − net. Drives the take-home bar. */
  totalWithheld: number
  /** Share of gross that actually reached the bank, 0–1. 0 when there's no gross. */
  takeHomeRate: number
  count: number
}

/**
 * Breaks aggregate paychecks into the take-home story: of every gross dollar,
 * how much landed in your account (net) versus what was withheld, with the
 * withheld portion itemized into taxes and deductions. Accumulates in cents to
 * avoid floating-point drift, matching `paycheckTotals`.
 */
export function paycheckBreakdown(paychecks: Paycheck[]): PaycheckBreakdown {
  let totalGross = 0
  let totalNet = 0
  let totalTaxes = 0
  let totalDeductions = 0
  for (const p of paychecks) {
    totalGross = cents(totalGross + p.grossAmount)
    totalNet = cents(totalNet + p.netAmount)
    totalTaxes = cents(
      totalTaxes + p.federalTax + p.stateTax + p.localTax + p.socialSecurityTax + p.medicareTax,
    )
    for (const d of p.deductions) totalDeductions = cents(totalDeductions + d.amount)
  }
  const totalWithheld = cents(Math.max(totalGross - totalNet, 0))
  const takeHomeRate = totalGross > 0 ? totalNet / totalGross : 0
  return { totalGross, totalNet, totalTaxes, totalDeductions, totalWithheld, takeHomeRate, count: paychecks.length }
}
