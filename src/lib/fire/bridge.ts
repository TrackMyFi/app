import type { FireAccount, FireBalance } from './types'
import { latestBalances } from './metrics'
import { isLiability } from '../accountTypes'
import { realAnnualReturn } from './coast'

/** Age when retirement-account withdrawals stop carrying the early-withdrawal penalty. */
export const PENALTY_FREE_AGE = 59.5

/**
 * Account types locked behind the 59½ penalty wall. Roth IRA contributions are
 * technically accessible early, but the app doesn't track basis vs. earnings,
 * so the whole account is treated as deferred — the conservative reading.
 */
const DEFERRED_TYPES = new Set(['401k', 'roth_401k', 'mixed_401k', 'traditional_ira', 'roth_ira', 'hsa'])

export interface BridgeSplit {
  /** Spendable before 59½: taxable, cash, crypto, real estate — minus liabilities. */
  accessible: number
  /** Locked in retirement accounts until 59½. */
  deferred: number
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
  for (const a of accounts) {
    if (!a.includeInFireCalculations) continue
    const bal = latest.get(a.id) ?? 0
    if (isLiability(a.type)) accessible -= bal
    else if (DEFERRED_TYPES.has(a.type)) deferred += bal
    else accessible += bal
  }
  return { accessible, deferred }
}

export interface BridgeStatus {
  accessible: number
  deferred: number
  ageAtFi: number
  /** Years between FI and 59½ that accessible funds must cover; 0 when FI lands past 59½. */
  bridgeYears: number
  bridgeNeeded: number
  /** Today's accessible funds compounded at the real return to the FI date — excludes future contributions. */
  projectedAccessibleAtFi: number
  /** projectedAccessibleAtFi / bridgeNeeded; null when no bridge is needed. */
  coverage: number | null
  needed: boolean
}

/**
 * Assesses the early-retirement bridge: an FI date before 59½ must be funded
 * entirely from accessible accounts until the penalty wall lifts. The projection
 * compounds only today's accessible balance (real return, no new contributions),
 * so it understates a saver who keeps funding taxable accounts — a shortfall
 * here is a flag to check, not a verdict.
 */
export function bridgeStatus(
  split: BridgeSplit, currentAge: number, yearsToFi: number, annualExpenses: number,
  expectedReturnRate: number, inflationRate: number,
): BridgeStatus {
  const ageAtFi = currentAge + Math.max(0, yearsToFi)
  const bridgeYears = Math.max(0, PENALTY_FREE_AGE - ageAtFi)
  const needed = bridgeYears > 0
  const bridgeNeeded = bridgeYears * annualExpenses
  const r = realAnnualReturn(expectedReturnRate, inflationRate)
  const projectedAccessibleAtFi = Math.max(0, split.accessible) * Math.pow(1 + r, Math.max(0, yearsToFi))
  return {
    accessible: split.accessible,
    deferred: split.deferred,
    ageAtFi,
    bridgeYears,
    bridgeNeeded,
    projectedAccessibleAtFi,
    coverage: needed && bridgeNeeded > 0 ? projectedAccessibleAtFi / bridgeNeeded : null,
    needed,
  }
}
