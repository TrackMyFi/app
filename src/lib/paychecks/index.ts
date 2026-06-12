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
