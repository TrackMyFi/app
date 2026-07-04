import { describe, it, expect } from 'vitest'
import { accessibleSplit, bridgeStatus, PENALTY_FREE_AGE } from './bridge'
import type { FireAccount, FireBalance } from './types'

const acct = (id: number, type: string, include = true): FireAccount =>
  ({ id, type, includeInFireCalculations: include })
const bal = (accountId: number, balance: number): FireBalance =>
  ({ accountId, balance, recordedAt: '2026-06-01' })

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
  })

  it('excludes accounts not counted toward FIRE', () => {
    const s = accessibleSplit([acct(1, 'brokerage', false), acct(2, '401k')], [bal(1, 100_000), bal(2, 50_000)])
    expect(s.accessible).toBe(0)
    expect(s.deferred).toBe(50_000)
  })

  it('liabilities reduce the accessible side', () => {
    const s = accessibleSplit([acct(1, 'brokerage'), acct(2, 'liability')], [bal(1, 100_000), bal(2, 30_000)])
    expect(s.accessible).toBe(70_000)
    expect(s.deferred).toBe(0)
  })
})

describe('bridgeStatus', () => {
  const split = { accessible: 200_000, deferred: 400_000 }

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
    const r = bridgeStatus({ accessible: -10_000, deferred: 100_000 }, 40, 5, 40_000, 0.07, 0.03)
    expect(r.projectedAccessibleAtFi).toBe(0)
    expect(r.coverage).toBe(0)
  })
})
