import { classifyFlow } from '../transactions/flow'
import { resolveVendor, type VendorRuleInput } from './merchants'
import type { Transaction } from '../types/Transaction'
import type { Account } from '../types/Account'

type AccountLookup = Map<number, Account> | Account[]

export interface RecurringCharge {
  key: string
  displayName: string
  /** Literal substring of the original descriptions — safe to use as a Transactions search filter. */
  searchTerm: string
  /** Average amount per month it appeared. */
  monthlyAmount: number
  /** How many of the trailing window's calendar months it showed up in. */
  monthsSeen: number
  /** monthlyAmount × 12 — the "if this kept up all year" framing. */
  annualized: number
}

export interface RecurringOptions {
  /** ISO date (YYYY-MM-DD) the trailing window ends at — typically "today". */
  asOf: string
  /** How many trailing calendar months to look across. */
  monthsWindow?: number
  /** Minimum number of those months a charge must appear in to count as recurring. */
  minMonths?: number
  /** Max coefficient of variation across months for a charge to read as "consistent". */
  maxVariance?: number
}

function trailingMonthKeys(asOf: string, monthsWindow: number): Set<string> {
  let [year, month] = asOf.slice(0, 7).split('-').map(Number)
  const keys = new Set<string>()
  for (let i = 0; i < monthsWindow; i++) {
    keys.add(`${year}-${String(month).padStart(2, '0')}`)
    month -= 1
    if (month === 0) { month = 12; year -= 1 }
  }
  return keys
}

function mean(values: number[]): number {
  return values.reduce((s, v) => s + v, 0) / values.length
}

function coefficientOfVariation(values: number[]): number {
  const m = mean(values)
  if (m === 0) return Infinity
  const variance = mean(values.map((v) => (v - m) ** 2))
  return Math.sqrt(variance) / m
}

/**
 * Detects charges that show up most months at a roughly consistent amount —
 * subscriptions, memberships, recurring services. Scoped to the discretionary
 * bucket only:
 *  - "fixed" (rent, mortgage, insurance) is already-known obligations, not
 *    something a "cancel this" opportunity applies to.
 *  - "uncategorized" is excluded too — in practice it's often income that
 *    wasn't logged as such (a recurring payroll/Stripe deposit, say), and
 *    recommending someone "cancel" a recurring paycheck is worse than missing
 *    an occasional uncategorized subscription.
 */
export function detectRecurring(
  transactions: Transaction[],
  accounts: AccountLookup,
  options: RecurringOptions,
  vendorRules: VendorRuleInput[] = [],
): RecurringCharge[] {
  const monthsWindow = options.monthsWindow ?? 4
  const minMonths = options.minMonths ?? 3
  const maxVariance = options.maxVariance ?? 0.15
  const windowKeys = trailingMonthKeys(options.asOf, monthsWindow)

  interface Group { displayName: string; searchTerm: string; byMonth: Map<string, number> }
  const groups = new Map<string, Group>()

  for (const t of transactions) {
    const monthKey = t.date.slice(0, 7)
    if (!windowKeys.has(monthKey)) continue

    const flow = classifyFlow(t, accounts)
    if (flow.isSavings || flow.outflow <= 0) continue
    if (flow.bucket !== 'discretionary') continue

    const resolved = resolveVendor(t.description, vendorRules)
    if (resolved.isGeneric) continue

    let g = groups.get(resolved.key)
    if (!g) {
      g = { displayName: resolved.displayName, searchTerm: resolved.searchTerm, byMonth: new Map() }
      groups.set(resolved.key, g)
    }
    g.byMonth.set(monthKey, (g.byMonth.get(monthKey) ?? 0) + flow.outflow)
  }

  const result: RecurringCharge[] = []
  for (const [key, g] of groups) {
    if (g.byMonth.size < minMonths) continue
    const amounts = [...g.byMonth.values()]
    if (coefficientOfVariation(amounts) > maxVariance) continue

    const monthlyAmount = mean(amounts)
    result.push({
      key,
      displayName: g.displayName,
      searchTerm: g.searchTerm,
      monthlyAmount,
      monthsSeen: g.byMonth.size,
      annualized: monthlyAmount * 12,
    })
  }

  return result.sort((a, b) => b.annualized - a.annualized)
}
