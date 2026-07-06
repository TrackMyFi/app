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
  /**
   * Roth IRA contribution basis: tracked lifetime contributions (net of
   * withdrawals, capped at the balance), withdrawable tax- and penalty-free at
   * any age. Kept apart from `accessible` because basis doesn't grow — the
   * growth on those dollars accrues to the still-locked earnings.
   */
  rothBasis: number
  /**
   * Roth 401k contribution basis (including the Roth share of mixed 401ks):
   * locked while inside the plan, but rolling the Roth 401k into a Roth IRA —
   * the standard move at retirement — turns these tracked contributions into
   * withdrawable basis. Still counted inside `deferred` today; the bridge
   * projection unlocks it at FI. Non-growing, like all basis.
   */
  rolloverBasis: number
  /** Locked in retirement accounts until 59½ (Roth IRA basis already carved out). */
  deferred: number
  /** Subset of `deferred` a Roth conversion ladder could unlock (pre-tax dollars). */
  ladderable: number
}

/**
 * Lifetime contribution basis per account: contributions in, netted against
 * withdrawals (the IRS ordering takes contributions out first). Transfers
 * count toward the destination account.
 */
function contributionBasisByAccount(ids: Set<number>, txns: Transaction[]): Map<number, number> {
  const basis = new Map<number, number>()
  for (const t of txns) {
    if (!t.isContribution) continue
    const dest = t.transferAccountId ?? t.accountId
    if (!ids.has(dest)) continue
    basis.set(dest, (basis.get(dest) ?? 0) + (t.isWithdrawal ? -t.amount : t.amount))
  }
  return basis
}

/** The Roth share of a deferred account's dollars: all of a Roth account, the non-traditional slice of a mixed 401k. */
function rothShare(a: FireAccount): number {
  if (a.type === 'roth_ira' || a.type === 'roth_401k') return 1
  if (a.type === 'mixed_401k') return 1 - (a.traditionalPct ?? DEFAULT_MIXED_TRADITIONAL_PCT)
  return 0
}

/**
 * Splits investable net worth (same account set as `investableNetWorth`) into
 * what an early retiree could spend before 59½ vs. what's penalty-locked.
 * Liabilities reduce the accessible side — debts get paid from spendable money.
 *
 * When `txns` (full contribution history) is supplied, each Roth IRA's tracked
 * contribution basis moves from `deferred` to `rothBasis` — those dollars are
 * withdrawable at any age. Understates true basis when history predates the
 * app's records, which errs on the safe side.
 */
export function accessibleSplit(
  accounts: FireAccount[], balances: FireBalance[], txns?: Transaction[],
): BridgeSplit {
  const latest = latestBalances(balances)
  const basisIds = new Set(accounts.filter(a => a.includeInFireCalculations && rothShare(a) > 0).map(a => a.id))
  const basisById = txns ? contributionBasisByAccount(basisIds, txns) : new Map<number, number>()
  let accessible = 0
  let rothBasis = 0
  let rolloverBasis = 0
  let deferred = 0
  let ladderable = 0
  for (const a of accounts) {
    if (!a.includeInFireCalculations) continue
    const bal = latest.get(a.id) ?? 0
    if (isLiability(a.type)) accessible -= bal
    else if (DEFERRED_TYPES.has(a.type)) {
      // Contribution basis on the account's Roth side, capped at what that
      // side actually holds today.
      const share = rothShare(a)
      const basis = Math.min(
        Math.max(0, (basisById.get(a.id) ?? 0) * share),
        Math.max(0, bal * share),
      )
      if (a.type === 'roth_ira') {
        // Withdrawable today — carve it out of the locked side.
        rothBasis += basis
        deferred += bal - basis
      } else {
        // Roth 401k side: unlocks at FI via rollover, still locked today.
        rolloverBasis += basis
        deferred += bal
      }
      if (LADDERABLE_TYPES.has(a.type)) ladderable += bal
      else if (a.type === 'mixed_401k') ladderable += bal * (a.traditionalPct ?? DEFAULT_MIXED_TRADITIONAL_PCT)
    }
    else accessible += bal
  }
  return { accessible, rothBasis, rolloverBasis, deferred, ladderable }
}

export interface BridgeContributions {
  /** Monthly contributions landing in accessible (taxable/cash) accounts. */
  accessible: number
  /** Monthly contributions into Roth IRAs — new basis, spendable at any age but non-growing. */
  rothBasis: number
  /** Monthly contributions to Roth 401k sides — basis that unlocks at FI via rollover. */
  rolloverBasis: number
  /** Monthly contributions landing in ladder-convertible pre-tax accounts. */
  ladderable: number
}

/**
 * Splits the trailing-12-month contribution flow into the same buckets as
 * `accessibleSplit`, so the bridge can project balances forward WITH ongoing
 * contributions. Mirrors `trailingMonthlyContribution`'s window and withdrawal
 * netting; Roth IRA contributions become withdrawable basis, Roth 401k-side
 * contributions become rollover basis, and HSA contributions fund no bucket
 * (medically gated and not ladder-convertible).
 *
 * A contribution is bucketed by where the money lands, not where it's recorded:
 * imported transfers sit on the funding account (often a checking account
 * excluded from FIRE) with `transferAccountId` pointing at the destination.
 */
export function bridgeContributionSplit(
  accounts: FireAccount[], txns: Transaction[], asOfIso: string,
): BridgeContributions {
  const cutoff = DateTime.fromISO(asOfIso).minus({ months: 12 }).toISODate()!
  const byId = new Map(accounts.filter(a => a.includeInFireCalculations).map(a => [a.id, a]))
  let accessible = 0
  let rothBasis = 0
  let rolloverBasis = 0
  let ladderable = 0
  for (const t of txns) {
    if (!t.isContribution || t.date <= cutoff || t.date > asOfIso) continue
    const a = byId.get(t.transferAccountId ?? t.accountId)
    if (!a || isLiability(a.type)) continue
    const amt = t.isWithdrawal ? -t.amount : t.amount
    if (DEFERRED_TYPES.has(a.type)) {
      if (a.type === 'roth_ira') rothBasis += amt
      else if (a.type === 'roth_401k') rolloverBasis += amt
      else if (LADDERABLE_TYPES.has(a.type)) ladderable += amt
      else if (a.type === 'mixed_401k') {
        const pct = a.traditionalPct ?? DEFAULT_MIXED_TRADITIONAL_PCT
        ladderable += amt * pct
        rolloverBasis += amt * (1 - pct)
      }
    } else accessible += amt
  }
  return {
    accessible: accessible / 12,
    rothBasis: rothBasis / 12,
    rolloverBasis: rolloverBasis / 12,
    ladderable: ladderable / 12,
  }
}

/** Funding source of a slice of the bridge span, for the timeline viz. */
export type BridgeYearSource = 'accessible' | 'ladder' | 'gap'

export interface BridgeTimelineSlice {
  source: BridgeYearSource
  /** Length of the slice in years (bridge years sum to the full span). */
  years: number
}

export interface LadderStatus {
  /** What accessible funds must still cover once ladder conversions take over. */
  bridgeNeeded: number
  /** Ladderable (pre-tax) funds compounded at the real return to the FI date, plus ongoing contributions when supplied. */
  projectedLadderableAtFi: number
  /** Bridge years funded by seasoned conversions in the simulation. */
  fundableYears: number
  /** Post-seasoning bridge years that need conversion funding. */
  conversionYears: number
  /** Bridge years funded by accessible money (taxable growth + basis). */
  accessibleYears: number
  /** Bridge years nothing covers. */
  gapYears: number
  /** Share of what falls on accessible funds that they actually cover; 1 = fully covered. */
  coverage: number
  /** Chronological funding timeline across the bridge span. */
  timeline: BridgeTimelineSlice[]
}

interface BridgeSimulation {
  fundedYears: number
  accessibleYears: number
  ladderYears: number
  gapYears: number
  timeline: BridgeTimelineSlice[]
}

/**
 * Walks the bridge year by year instead of pricing it statically, so money not
 * yet spent keeps earning: the taxable pool and the unconverted pre-tax pool
 * compound at the real return, while basis and seasoned conversions sit at
 * face value (their growth accrues to still-locked Roth earnings). Each year
 * converts up to a year of expenses whenever the rung would still season
 * inside the bridge, and spends flat money (matured rungs, then basis) before
 * the growing taxable pool.
 */
function simulateBridge(
  bridgeYears: number, annualExpenses: number, realReturn: number,
  accessible: number, flatBasis: number, preTax: number,
): BridgeSimulation {
  const timeline: BridgeTimelineSlice[] = []
  const push = (source: BridgeYearSource, years: number) => {
    if (years <= 1e-9) return
    const last = timeline[timeline.length - 1]
    if (last?.source === source) last.years += years
    else timeline.push({ source, years })
  }
  const rungs = new Map<number, number>()
  let taxable = Math.max(0, accessible)
  let basis = Math.max(0, flatBasis)
  let preTaxLeft = Math.max(0, preTax)
  let matured = 0
  let accessibleYears = 0
  let ladderYears = 0
  let gapYears = 0
  const yearCount = Math.ceil(bridgeYears - 1e-9)
  for (let k = 0; k < yearCount; k++) {
    const weight = Math.min(1, bridgeYears - k)
    // Convert only while the rung still matures before 59½ unlocks everything.
    if (preTaxLeft > 0 && k + LADDER_SEASONING_YEARS < bridgeYears) {
      const converted = Math.min(preTaxLeft, annualExpenses)
      preTaxLeft -= converted
      rungs.set(k + LADDER_SEASONING_YEARS, converted)
    }
    matured += rungs.get(k) ?? 0
    let need = annualExpenses * weight
    const fromLadder = Math.min(matured, need)
    matured -= fromLadder
    need -= fromLadder
    const fromBasis = Math.min(basis, need)
    basis -= fromBasis
    need -= fromBasis
    const fromTaxable = Math.min(taxable, need)
    taxable -= fromTaxable
    need -= fromTaxable
    ladderYears += fromLadder / annualExpenses
    accessibleYears += (fromBasis + fromTaxable) / annualExpenses
    gapYears += need / annualExpenses
    push('ladder', fromLadder / annualExpenses)
    push('accessible', (fromBasis + fromTaxable) / annualExpenses)
    push('gap', need / annualExpenses)
    const growth = Math.pow(1 + realReturn, weight)
    taxable *= growth
    preTaxLeft *= growth
  }
  return {
    fundedYears: accessibleYears + ladderYears,
    accessibleYears,
    ladderYears,
    gapYears,
    timeline,
  }
}

export interface BridgeStatus {
  accessible: number
  deferred: number
  ageAtFi: number
  /** Years between FI and 59½ that accessible funds must cover; 0 when FI lands past 59½. */
  bridgeYears: number
  bridgeNeeded: number
  /**
   * Spendable at FI: accessible funds compounded at the real return to the FI
   * date, plus Roth IRA basis and Roth 401k rollover basis at face value
   * (basis doesn't grow — its growth belongs to the locked earnings). Includes
   * the future value of ongoing contributions when a contribution split is
   * supplied; otherwise today's balances alone.
   */
  projectedAccessibleAtFi: number
  /**
   * Share of bridge years accessible funds cover in a taxable-only drawdown,
   * with unspent money compounding through the bridge; 1 = fully covered.
   * Null when no bridge is needed.
   */
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
 *
 * Coverage comes from a year-by-year drawdown simulation across the bridge —
 * see `simulateBridge` — not a static price of the whole span at FI.
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
  // Basis is spendable during the bridge but never compounds: today's basis at
  // face value, plus ongoing basis-building contributions without growth. The
  // Roth 401k side joins via the rollover assumed to happen at FI.
  const basisAtFi = Math.max(0, split.rothBasis) + (contributions?.rothBasis ?? 0) * months
    + Math.max(0, split.rolloverBasis) + (contributions?.rolloverBasis ?? 0) * months
  const taxableAtFi = Math.max(0, Math.max(0, split.accessible) * growth + contribFv(contributions?.accessible ?? 0))
  const projectedAccessibleAtFi = taxableAtFi + basisAtFi
  const projectedLadderableAtFi = Math.max(0, Math.max(0, split.ladderable) * growth + contribFv(contributions?.ladderable ?? 0))
  const canSimulate = needed && annualExpenses > 0
  // Taxable-only drawdown: accessible money alone carries the whole bridge.
  const taxableOnly = canSimulate
    ? simulateBridge(bridgeYears, annualExpenses, r, taxableAtFi, basisAtFi, 0)
    : null
  const conversionYears = bridgeYears - LADDER_SEASONING_YEARS
  const ladderSim = canSimulate && conversionYears > 0 && projectedLadderableAtFi > 0
    ? simulateBridge(bridgeYears, annualExpenses, r, taxableAtFi, basisAtFi, projectedLadderableAtFi)
    : null
  return {
    accessible: split.accessible,
    deferred: split.deferred,
    ageAtFi,
    bridgeYears,
    bridgeNeeded,
    projectedAccessibleAtFi,
    coverage: taxableOnly
      ? (taxableOnly.gapYears <= 1e-9 ? 1 : taxableOnly.fundedYears / bridgeYears)
      : null,
    needed,
    ladder: ladderSim ? {
      bridgeNeeded: (bridgeYears - ladderSim.ladderYears) * annualExpenses,
      projectedLadderableAtFi,
      fundableYears: ladderSim.ladderYears,
      conversionYears,
      accessibleYears: ladderSim.accessibleYears,
      gapYears: ladderSim.gapYears,
      coverage: ladderSim.gapYears <= 1e-9
        ? 1
        : ladderSim.accessibleYears / (ladderSim.accessibleYears + ladderSim.gapYears),
      timeline: ladderSim.timeline,
    } : null,
  }
}
