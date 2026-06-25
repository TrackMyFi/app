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
