import { describe, it, expect } from 'vitest'
import {
  accessibleSplit, bridgeStatus, bridgeContributionSplit,
  PENALTY_FREE_AGE, LADDER_SEASONING_YEARS, DEFAULT_MIXED_TRADITIONAL_PCT,
} from './bridge'
import type { FireAccount, FireBalance } from './types'
import type { Transaction } from '../types/Transaction'

const acct = (id: number, type: string, include = true): FireAccount =>
  ({ id, type, includeInFireCalculations: include })
const bal = (accountId: number, balance: number): FireBalance =>
  ({ accountId, balance, recordedAt: '2026-06-01' })

// Minimal Transaction factory — only fields bridgeContributionSplit reads matter.
function txn(p: Partial<Transaction>): Transaction {
  return {
    id: 1, accountId: 1, transferAccountId: null, amount: 0, description: '',
    date: '2026-01-01', type: 'transfer', category: '', isContribution: true, isWithdrawal: false,
    importSource: 'manual', generatedBalanceId: null, generatedBalanceToId: null,
    paycheckId: null, vendorCategory: null, simplefinId: null, suppressedAs: null, rawDescription: null, createdAt: '', updatedAt: '', ...p,
  }
}

// Real monthly return matching realMonthlyReturn(0.07, 0.03).
const REAL_MONTHLY = Math.pow(1.07 / 1.03, 1 / 12) - 1
const annuityFv = (monthly: number, months: number) =>
  monthly * ((Math.pow(1 + REAL_MONTHLY, months) - 1) / REAL_MONTHLY)

describe('accessibleSplit', () => {
  it('splits taxable/cash from penalty-locked retirement accounts', () => {
    const accounts = [
      acct(1, 'brokerage'), acct(2, '401k'), acct(3, 'roth_ira'),
      acct(4, 'savings'), acct(5, 'crypto'), acct(6, 'hsa'),
    ]
    const balances = [bal(1, 100_000), bal(2, 200_000), bal(3, 50_000), bal(4, 20_000), bal(5, 5_000), bal(6, 15_000)]
    const s = accessibleSplit(accounts, balances)
    expect(s.accessible).toBe(125_000) // brokerage + savings + crypto
    expect(s.deferred).toBe(265_000) // 401k + roth ira + hsa
    expect(s.ladderable).toBe(200_000) // only the traditional 401k converts
  })

  it('counts the traditional share of a mixed 401k as ladderable', () => {
    const mixed: FireAccount = { id: 1, type: 'mixed_401k', includeInFireCalculations: true, traditionalPct: 0.7 }
    const s = accessibleSplit([mixed, acct(2, 'traditional_ira')], [bal(1, 100_000), bal(2, 50_000)])
    expect(s.deferred).toBe(150_000)
    expect(s.ladderable).toBeCloseTo(0.7 * 100_000 + 50_000, 6)
  })

  it('assumes a default traditional share when a mixed 401k has none set', () => {
    const s = accessibleSplit([acct(1, 'mixed_401k')], [bal(1, 100_000)])
    expect(s.ladderable).toBeCloseTo(DEFAULT_MIXED_TRADITIONAL_PCT * 100_000, 6)
  })

  it('excludes accounts not counted toward FIRE', () => {
    const s = accessibleSplit([acct(1, 'brokerage', false), acct(2, '401k')], [bal(1, 100_000), bal(2, 50_000)])
    expect(s.accessible).toBe(0)
    expect(s.deferred).toBe(50_000)
    expect(s.ladderable).toBe(50_000)
  })

  it('carves tracked Roth IRA contribution basis out of the locked side', () => {
    const accounts = [acct(1, 'roth_ira'), acct(2, '401k')]
    const s = accessibleSplit(accounts, [bal(1, 50_000), bal(2, 100_000)], [
      txn({ accountId: 1, amount: 20_000, date: '2020-05-01' }),
      txn({ accountId: 9, transferAccountId: 1, amount: 5_000, date: '2024-01-15' }), // transfer → destination
      txn({ accountId: 1, amount: 3_000, date: '2025-02-01', isWithdrawal: true }), // withdrawals eat basis first
      txn({ accountId: 2, amount: 10_000, date: '2023-01-01' }), // pre-tax: never basis
    ])
    expect(s.rothBasis).toBe(22_000) // 20k + 5k − 3k
    expect(s.deferred).toBe(50_000 - 22_000 + 100_000)
    expect(s.ladderable).toBe(100_000)
  })

  it('caps Roth basis at the account balance and floors it at zero', () => {
    const capped = accessibleSplit([acct(1, 'roth_ira')], [bal(1, 10_000)], [
      txn({ accountId: 1, amount: 25_000, date: '2020-05-01' }), // account lost value
    ])
    expect(capped.rothBasis).toBe(10_000)
    expect(capped.deferred).toBe(0)
    const drained = accessibleSplit([acct(1, 'roth_ira')], [bal(1, 10_000)], [
      txn({ accountId: 1, amount: 5_000, date: '2020-05-01' }),
      txn({ accountId: 1, amount: 8_000, date: '2024-05-01', isWithdrawal: true }),
    ])
    expect(drained.rothBasis).toBe(0)
    expect(drained.deferred).toBe(10_000)
  })

  it('reports zero Roth basis without contribution history', () => {
    const s = accessibleSplit([acct(1, 'roth_ira')], [bal(1, 50_000)])
    expect(s.rothBasis).toBe(0)
    expect(s.deferred).toBe(50_000)
  })

  it('liabilities reduce the accessible side', () => {
    const s = accessibleSplit([acct(1, 'brokerage'), acct(2, 'liability')], [bal(1, 100_000), bal(2, 30_000)])
    expect(s.accessible).toBe(70_000)
    expect(s.deferred).toBe(0)
  })
})

describe('bridgeStatus', () => {
  const split = { accessible: 200_000, rothBasis: 0, deferred: 400_000, ladderable: 400_000 }

  it('needs no bridge when FI lands at or past 59½', () => {
    const r = bridgeStatus(split, 50, 10, 40_000, 0.07, 0.03)
    expect(r.needed).toBe(false)
    expect(r.bridgeYears).toBe(0)
    expect(r.coverage).toBeNull()
  })

  it('sizes the bridge from FI age to 59½ at annual expenses', () => {
    // FI at 45 → 14.5 bridge years
    const r = bridgeStatus(split, 40, 5, 40_000, 0.07, 0.03)
    expect(r.needed).toBe(true)
    expect(r.ageAtFi).toBe(45)
    expect(r.bridgeYears).toBeCloseTo(PENALTY_FREE_AGE - 45, 6)
    expect(r.bridgeNeeded).toBeCloseTo(14.5 * 40_000, 6)
  })

  it('projects accessible funds to FI at the real return, without contributions', () => {
    const r = bridgeStatus(split, 40, 5, 40_000, 0.07, 0.03)
    const real = (1.07 / 1.03) - 1
    expect(r.projectedAccessibleAtFi).toBeCloseTo(200_000 * Math.pow(1 + real, 5), 2)
    expect(r.coverage).toBeCloseTo(r.projectedAccessibleAtFi / r.bridgeNeeded, 6)
  })

  it('already-FI (yearsToFi 0) still reports the remaining bridge', () => {
    const r = bridgeStatus(split, 45, 0, 40_000, 0.07, 0.03)
    expect(r.needed).toBe(true)
    expect(r.projectedAccessibleAtFi).toBe(200_000)
    expect(r.bridgeYears).toBeCloseTo(14.5, 6)
  })

  it('an underwater accessible balance projects to zero, not negative', () => {
    const r = bridgeStatus({ accessible: -10_000, rothBasis: 0, deferred: 100_000, ladderable: 0 }, 40, 5, 40_000, 0.07, 0.03)
    expect(r.projectedAccessibleAtFi).toBe(0)
    expect(r.coverage).toBe(0)
  })
})

describe('bridgeStatus ladder', () => {
  it('shrinks the bridge to the seasoning window when pre-tax funds cover all conversion years', () => {
    // FI at 45 → 14.5 bridge years, 9.5 conversion years needing 380k; 400k grows past that.
    const split = { accessible: 200_000, rothBasis: 0, deferred: 400_000, ladderable: 400_000 }
    const r = bridgeStatus(split, 40, 5, 40_000, 0.07, 0.03)
    const l = r.ladder!
    expect(l.conversionYears).toBeCloseTo(r.bridgeYears - LADDER_SEASONING_YEARS, 6)
    expect(l.fundableYears).toBeCloseTo(l.conversionYears, 6)
    expect(l.bridgeNeeded).toBeCloseTo(LADDER_SEASONING_YEARS * 40_000, 6)
    expect(l.coverage).toBeCloseTo(r.projectedAccessibleAtFi / l.bridgeNeeded, 6)
    expect(l.coverage).toBeGreaterThan(1)
  })

  it('grows the ladder bridge when pre-tax funds cannot fund every conversion year', () => {
    // 100k pre-tax at FI funds only 2.5 of 9.5 conversion years — 12 years fall on accessible money.
    const split = { accessible: 200_000, rothBasis: 0, deferred: 100_000, ladderable: 100_000 }
    const r = bridgeStatus(split, 40, 0, 40_000, 0.07, 0.03)
    const l = r.ladder!
    expect(l.projectedLadderableAtFi).toBe(100_000)
    expect(l.fundableYears).toBeCloseTo(2.5, 6)
    expect(l.bridgeNeeded).toBeCloseTo((r.bridgeYears - 2.5) * 40_000, 6)
  })

  it('reports no ladder when nothing is convertible', () => {
    const r = bridgeStatus({ accessible: 200_000, rothBasis: 0, deferred: 400_000, ladderable: 0 }, 40, 5, 40_000, 0.07, 0.03)
    expect(r.ladder).toBeNull()
  })

  it('reports no ladder when the bridge fits inside the seasoning window', () => {
    // FI at 56 → 3.5 bridge years; conversions would never season in time to help.
    const r = bridgeStatus({ accessible: 200_000, rothBasis: 0, deferred: 400_000, ladderable: 400_000 }, 50, 6, 40_000, 0.07, 0.03)
    expect(r.needed).toBe(true)
    expect(r.ladder).toBeNull()
  })

  it('reports no ladder when no bridge is needed', () => {
    const r = bridgeStatus({ accessible: 200_000, rothBasis: 0, deferred: 400_000, ladderable: 400_000 }, 50, 10, 40_000, 0.07, 0.03)
    expect(r.needed).toBe(false)
    expect(r.ladder).toBeNull()
  })
})

describe('bridgeContributionSplit', () => {
  const asOf = '2026-06-30'
  const accounts = [acct(1, 'brokerage'), acct(2, '401k'), acct(3, 'roth_ira'),
    { ...acct(4, 'mixed_401k'), traditionalPct: 0.6 }]

  it('buckets trailing-12-month contributions by account type', () => {
    const c = bridgeContributionSplit(accounts, [
      txn({ accountId: 1, amount: 1_200, date: '2026-06-01' }),
      txn({ accountId: 2, amount: 2_400, date: '2026-03-01' }),
      txn({ accountId: 3, amount: 600, date: '2026-02-01' }), // roth: basis bucket
      txn({ accountId: 4, amount: 1_000, date: '2026-01-01' }), // mixed: 60% traditional
    ], asOf)
    expect(c.accessible).toBeCloseTo(1_200 / 12, 6)
    expect(c.rothBasis).toBeCloseTo(600 / 12, 6)
    expect(c.ladderable).toBeCloseTo((2_400 + 0.6 * 1_000) / 12, 6)
  })

  it('ignores contributions outside the trailing window, withdrawals net down', () => {
    const c = bridgeContributionSplit(accounts, [
      txn({ accountId: 1, amount: 1_200, date: '2025-06-30' }), // at cutoff — excluded
      txn({ accountId: 1, amount: 1_200, date: '2026-06-01' }),
      txn({ accountId: 1, amount: 300, date: '2026-06-15', isWithdrawal: true }),
    ], asOf)
    expect(c.accessible).toBeCloseTo(900 / 12, 6)
  })

  it('buckets transfers by destination account, not the funding account', () => {
    const withChecking = [...accounts, acct(5, 'checking', false)]
    const c = bridgeContributionSplit(withChecking, [
      // Imported transfers recorded on excluded checking, landing elsewhere.
      txn({ accountId: 5, transferAccountId: 1, amount: 1_200, date: '2026-06-01' }), // → brokerage
      txn({ accountId: 5, transferAccountId: 2, amount: 2_400, date: '2026-05-01' }), // → 401k
      txn({ accountId: 5, transferAccountId: 3, amount: 600, date: '2026-04-01' }), // → roth: basis bucket
    ], asOf)
    expect(c.accessible).toBeCloseTo(1_200 / 12, 6)
    expect(c.rothBasis).toBeCloseTo(600 / 12, 6)
    expect(c.ladderable).toBeCloseTo(2_400 / 12, 6)
  })

  it('skips accounts excluded from FIRE and unknown accounts', () => {
    const c = bridgeContributionSplit([acct(1, 'brokerage', false)], [
      txn({ accountId: 1, amount: 1_200, date: '2026-06-01' }),
      txn({ accountId: 99, amount: 1_200, date: '2026-06-01' }),
    ], asOf)
    expect(c.accessible).toBe(0)
    expect(c.ladderable).toBe(0)
  })
})

describe('bridgeStatus with contributions', () => {
  const split = { accessible: 200_000, rothBasis: 0, deferred: 400_000, ladderable: 400_000 }

  it('adds the future value of ongoing contributions to both projections', () => {
    const base = bridgeStatus(split, 40, 5, 40_000, 0.07, 0.03)
    const r = bridgeStatus(split, 40, 5, 40_000, 0.07, 0.03, { accessible: 1_000, rothBasis: 0, ladderable: 2_000 })
    expect(r.projectedAccessibleAtFi).toBeCloseTo(base.projectedAccessibleAtFi + annuityFv(1_000, 60), 2)
    expect(r.ladder!.projectedLadderableAtFi).toBeCloseTo(base.ladder!.projectedLadderableAtFi + annuityFv(2_000, 60), 2)
  })

  it('taxable contributions alone can turn a thin bridge into a covered one', () => {
    const thin = { accessible: 50_000, rothBasis: 0, deferred: 400_000, ladderable: 400_000 }
    const without = bridgeStatus(thin, 40, 5, 40_000, 0.07, 0.03)
    const withContrib = bridgeStatus(thin, 40, 5, 40_000, 0.07, 0.03, { accessible: 2_500, rothBasis: 0, ladderable: 0 })
    expect(without.ladder!.coverage).toBeLessThan(1)
    expect(withContrib.ladder!.coverage).toBeGreaterThan(without.ladder!.coverage)
  })

  it('a ladder can appear on contributions alone (zero pre-tax balance today)', () => {
    const noPreTax = { accessible: 100_000, rothBasis: 0, deferred: 0, ladderable: 0 }
    const without = bridgeStatus(noPreTax, 40, 5, 40_000, 0.07, 0.03)
    const withContrib = bridgeStatus(noPreTax, 40, 5, 40_000, 0.07, 0.03, { accessible: 0, rothBasis: 0, ladderable: 1_500 })
    expect(without.ladder).toBeNull()
    expect(withContrib.ladder).not.toBeNull()
    expect(withContrib.ladder!.projectedLadderableAtFi).toBeCloseTo(annuityFv(1_500, 60), 2)
  })

  it('adds Roth basis at face value — no compounding on basis dollars', () => {
    const withBasis = { accessible: 200_000, rothBasis: 30_000, deferred: 370_000, ladderable: 400_000 }
    const base = bridgeStatus(split, 40, 5, 40_000, 0.07, 0.03)
    const r = bridgeStatus(withBasis, 40, 5, 40_000, 0.07, 0.03)
    expect(r.projectedAccessibleAtFi).toBeCloseTo(base.projectedAccessibleAtFi + 30_000, 2)
  })

  it('accumulates ongoing Roth contributions without growth', () => {
    const base = bridgeStatus(split, 40, 5, 40_000, 0.07, 0.03)
    const r = bridgeStatus(split, 40, 5, 40_000, 0.07, 0.03, { accessible: 0, rothBasis: 500, ladderable: 0 })
    expect(r.projectedAccessibleAtFi).toBeCloseTo(base.projectedAccessibleAtFi + 500 * 60, 2)
  })

  it('net-withdrawal flows cannot push projections below zero', () => {
    const r = bridgeStatus({ accessible: 1_000, rothBasis: 0, deferred: 0, ladderable: 0 }, 40, 5, 40_000, 0.07, 0.03, { accessible: -500, rothBasis: 0, ladderable: 0 })
    expect(r.projectedAccessibleAtFi).toBe(0)
  })
})
