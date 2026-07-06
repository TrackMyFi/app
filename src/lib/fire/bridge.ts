import { DateTime } from 'luxon'
import type { FireAccount, FireBalance } from './types'
import type { Transaction } from '../types/Transaction'
import { latestBalances } from './metrics'
import { isLiability } from '../accountTypes'
import { realAnnualReturn } from './coast'
import { realMonthlyReturn } from './projection'

/** Age when retirement-account withdrawals stop carrying the early-withdrawal penalty. */
export const PENALTY_FREE_AGE = 59.5

/**
 * Account types locked behind the 59½ penalty wall. Roth IRA contributions are
 * technically accessible early, but the app doesn't track basis vs. earnings,
 * so the whole account is treated as deferred — the conservative reading.
 */
const DEFERRED_TYPES = new Set(['401k', 'roth_401k', 'mixed_401k', 'traditional_ira', 'roth_ira', 'hsa'])

/**
 * Deferred types whose dollars a Roth conversion ladder can unlock: pre-tax
 * money that can be converted to Roth and withdrawn penalty-free after the
 * seasoning period. Roth accounts are already converted (earnings stay locked)
 * and HSA money is gated on medical expenses, so neither can be laddered.
 */
const LADDERABLE_TYPES = new Set(['401k', 'traditional_ira'])

/** Assumed traditional share of a mixed 401k when the user hasn't set one. */
export const DEFAULT_MIXED_TRADITIONAL_PCT = 0.5

/** Years a Roth conversion must season before the converted amount is withdrawable. */
export const LADDER_SEASONING_YEARS = 5

export interface BridgeSplit {
  /** Spendable before 59½: taxable, cash, crypto, real estate — minus liabilities. */
  accessible: number
  /** Locked in retirement accounts until 59½. */
  deferred: number
  /** Subset of `deferred` a Roth conversion ladder could unlock (pre-tax dollars). */
  ladderable: number
}

/**
 * Splits investable net worth (same account set as `investableNetWorth`) into
 * what an early retiree could spend before 59½ vs. what's penalty-locked.
 * Liabilities reduce the accessible side — debts get paid from spendable money.
 */
export function accessibleSplit(accounts: FireAccount[], balances: FireBalance[]): BridgeSplit {
  const latest = latestBalances(balances)
  let accessible = 0
  let deferred = 0
  let ladderable = 0
  for (const a of accounts) {
    if (!a.includeInFireCalculations) continue
    const bal = latest.get(a.id) ?? 0
    if (isLiability(a.type)) accessible -= bal
    else if (DEFERRED_TYPES.has(a.type)) {
      deferred += bal
      if (LADDERABLE_TYPES.has(a.type)) ladderable += bal
      else if (a.type === 'mixed_401k') ladderable += bal * (a.traditionalPct ?? DEFAULT_MIXED_TRADITIONAL_PCT)
    }
    else accessible += bal
  }
  return { accessible, deferred, ladderable }
}

export interface BridgeContributions {
  /** Monthly contributions landing in accessible (taxable/cash) accounts. */
  accessible: number
  /** Monthly contributions landing in ladder-convertible pre-tax accounts. */
  ladderable: number
}

/**
 * Splits the trailing-12-month contribution flow into the same buckets as
 * `accessibleSplit`, so the bridge can project balances forward WITH ongoing
 * contributions. Mirrors `trailingMonthlyContribution`'s window and withdrawal
 * netting; contributions to Roth/HSA accounts fund neither bucket (locked and
 * not ladder-convertible).
 */
export function bridgeContributionSplit(
  accounts: FireAccount[], txns: Transaction[], asOfIso: string,
): BridgeContributions {
  const cutoff = DateTime.fromISO(asOfIso).minus({ months: 12 }).toISODate()!
  const byId = new Map(accounts.filter(a => a.includeInFireCalculations).map(a => [a.id, a]))
  let accessible = 0
  let ladderable = 0
  for (const t of txns) {
    if (!t.isContribution || t.date <= cutoff || t.date > asOfIso) continue
    const a = byId.get(t.accountId)
    if (!a || isLiability(a.type)) continue
    const amt = t.isWithdrawal ? -t.amount : t.amount
    if (DEFERRED_TYPES.has(a.type)) {
      if (LADDERABLE_TYPES.has(a.type)) ladderable += amt
      else if (a.type === 'mixed_401k') ladderable += amt * (a.traditionalPct ?? DEFAULT_MIXED_TRADITIONAL_PCT)
    } else accessible += amt
  }
  return { accessible: accessible / 12, ladderable: ladderable / 12 }
}

export interface LadderStatus {
  /** What accessible funds must still cover once ladder conversions take over. */
  bridgeNeeded: number
  /** Ladderable (pre-tax) funds compounded at the real return to the FI date, plus ongoing contributions when supplied. */
  projectedLadderableAtFi: number
  /** Post-seasoning bridge years the projected pre-tax balance can fund via conversions. */
  fundableYears: number
  /** Post-seasoning bridge years that need conversion funding. */
  conversionYears: number
  /** projectedAccessibleAtFi / ladder bridgeNeeded. */
  coverage: number
}

export interface BridgeStatus {
  accessible: number
  deferred: number
  ageAtFi: number
  /** Years between FI and 59½ that accessible funds must cover; 0 when FI lands past 59½. */
  bridgeYears: number
  bridgeNeeded: number
  /**
   * Accessible funds compounded at the real return to the FI date. Includes the
   * future value of ongoing contributions when a contribution split is supplied;
   * otherwise today's balance alone.
   */
  projectedAccessibleAtFi: number
  /** projectedAccessibleAtFi / bridgeNeeded; null when no bridge is needed. */
  coverage: number | null
  needed: boolean
  /**
   * The bridge re-assessed assuming a Roth conversion ladder: accessible funds
   * only carry the 5-year seasoning window; annual conversions from pre-tax
   * accounts fund the rest. Null when no bridge is needed, when the bridge is
   * within the seasoning window anyway, or when there's nothing to convert.
   */
  ladder: LadderStatus | null
}

/**
 * Assesses the early-retirement bridge: an FI date before 59½ must be funded
 * entirely from accessible accounts until the penalty wall lifts. Projects
 * today's balances at the real return to the FI date; when `contributions` is
 * supplied, the current monthly flow into each bucket is assumed to continue
 * until FI (and stop there). Without it the projection is balances-only and
 * understates a saver who keeps contributing — a flag to check, not a verdict.
 */
export function bridgeStatus(
  split: BridgeSplit, currentAge: number, yearsToFi: number, annualExpenses: number,
  expectedReturnRate: number, inflationRate: number,
  contributions?: BridgeContributions,
): BridgeStatus {
  const ageAtFi = currentAge + Math.max(0, yearsToFi)
  const bridgeYears = Math.max(0, PENALTY_FREE_AGE - ageAtFi)
  const needed = bridgeYears > 0
  const bridgeNeeded = bridgeYears * annualExpenses
  const r = realAnnualReturn(expectedReturnRate, inflationRate)
  const growth = Math.pow(1 + r, Math.max(0, yearsToFi))
  // Future value of a monthly flow continued until FI, at the real return.
  const i = realMonthlyReturn(expectedReturnRate, inflationRate)
  const months = Math.max(0, yearsToFi) * 12
  const contribFv = (monthly: number) => i > 0 ? monthly * ((Math.pow(1 + i, months) - 1) / i) : monthly * months
  const projectedAccessibleAtFi = Math.max(0, Math.max(0, split.accessible) * growth + contribFv(contributions?.accessible ?? 0))
  const projectedLadderableAtFi = Math.max(0, Math.max(0, split.ladderable) * growth + contribFv(contributions?.ladderable ?? 0))
  return {
    accessible: split.accessible,
    deferred: split.deferred,
    ageAtFi,
    bridgeYears,
    bridgeNeeded,
    projectedAccessibleAtFi,
    coverage: needed && bridgeNeeded > 0 ? projectedAccessibleAtFi / bridgeNeeded : null,
    needed,
    ladder: ladderStatus(bridgeYears, annualExpenses, projectedAccessibleAtFi, projectedLadderableAtFi),
  }
}

/**
 * Re-assesses the bridge assuming a Roth conversion ladder: conversions start
 * in year one of FI and each becomes withdrawable five years later, so
 * accessible funds only have to carry the seasoning window — the rest of the
 * bridge draws from converted pre-tax dollars. When the projected pre-tax
 * balance can't fund every post-seasoning year, the unfunded years fall back
 * on accessible money and the ladder's bridge need grows accordingly.
 */
function ladderStatus(
  bridgeYears: number, annualExpenses: number,
  projectedAccessibleAtFi: number, projectedLadderableAtFi: number,
): LadderStatus | null {
  const conversionYears = bridgeYears - LADDER_SEASONING_YEARS
  if (conversionYears <= 0 || projectedLadderableAtFi <= 0 || annualExpenses <= 0) return null
  const fundableYears = Math.min(conversionYears, projectedLadderableAtFi / annualExpenses)
  const bridgeNeeded = (bridgeYears - fundableYears) * annualExpenses
  return {
    bridgeNeeded,
    projectedLadderableAtFi,
    fundableYears,
    conversionYears,
    coverage: projectedAccessibleAtFi / bridgeNeeded,
  }
}
