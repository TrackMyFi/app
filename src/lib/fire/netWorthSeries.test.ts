import { describe, it, expect } from 'vitest'
import { netWorthOverTime, drawdownStatus, type NetWorthPoint } from './netWorthSeries'
import type { FireAccount, FireBalance } from './types'

const accounts: FireAccount[] = [
  { id: 1, type: 'brokerage', includeInFireCalculations: true },
  { id: 2, type: 'liability', includeInFireCalculations: false },
]
const balances: FireBalance[] = [
  { accountId: 1, balance: 100, recordedAt: '2026-01-01' },
  { accountId: 2, balance: 40, recordedAt: '2026-01-15' },
  { accountId: 1, balance: 300, recordedAt: '2026-02-01' },
]

describe('netWorthOverTime', () => {
  it('computes net worth at each distinct date using carry-forward', () => {
    // brokerage is liquid; liability is excluded from liquid (it's a signed liability, not an asset)
    expect(netWorthOverTime(accounts, balances)).toEqual([
      { date: '2026-01-01', netWorth: 100, lessEquity: null, liquid: 100, illiquid: 0 },
      { date: '2026-01-15', netWorth: 60,  lessEquity: null, liquid: 100, illiquid: -40 },
      { date: '2026-02-01', netWorth: 260, lessEquity: null, liquid: 300, illiquid: -40 },
    ])
  })

  it('returns empty for no balances', () => { expect(netWorthOverTime(accounts, [])).toEqual([]) })

  it('splits liquid and illiquid', () => {
    const accs: FireAccount[] = [
      { id: 1, type: 'checking',   includeInFireCalculations: false },
      { id: 2, type: 'brokerage',  includeInFireCalculations: true },
      { id: 3, type: 'traditional_ira', includeInFireCalculations: true },
    ]
    const bals: FireBalance[] = [
      { accountId: 1, balance: 5_000,  recordedAt: '2026-01-01' },
      { accountId: 2, balance: 80_000, recordedAt: '2026-01-01' },
      { accountId: 3, balance: 15_000, recordedAt: '2026-01-01' },
    ]
    // checking + brokerage = liquid; traditional_ira = illiquid
    expect(netWorthOverTime(accs, bals)).toEqual([
      { date: '2026-01-01', netWorth: 100_000, lessEquity: null, liquid: 85_000, illiquid: 15_000 },
    ])
  })

  it('sets lessEquity when real estate balance exists', () => {
    const accs: FireAccount[] = [
      { id: 1, type: 'real_estate', includeInFireCalculations: false },
      { id: 2, type: 'mortgage',    includeInFireCalculations: false },
      { id: 3, type: 'brokerage',   includeInFireCalculations: true },
    ]
    const bals: FireBalance[] = [
      { accountId: 1, balance: 400_000, recordedAt: '2026-01-01' },
      { accountId: 2, balance: 300_000, recordedAt: '2026-01-01' },
      { accountId: 3, balance: 100_000, recordedAt: '2026-01-01' },
    ]
    // netWorth = 400k - 300k + 100k = 200k
    // lessEquity = just brokerage = 100k (real_estate + mortgage excluded)
    // liquid = brokerage = 100k; illiquid = 200k - 100k = 100k
    expect(netWorthOverTime(accs, bals)).toEqual([
      { date: '2026-01-01', netWorth: 200_000, lessEquity: 100_000, liquid: 100_000, illiquid: 100_000 },
    ])
  })
})

describe('drawdownStatus', () => {
  const pt = (date: string, netWorth: number): NetWorthPoint =>
    ({ date, netWorth, lessEquity: null, liquid: 0, illiquid: netWorth })

  it('reports at-high when the latest point is the peak', () => {
    const r = drawdownStatus([pt('2026-01-01', 100_000), pt('2026-02-01', 120_000)])!
    expect(r.atHigh).toBe(true)
    expect(r.drawdown).toBe(0)
    expect(r.high).toBe(120_000)
    expect(r.highDate).toBe('2026-02-01')
  })

  it('measures the drop from a past peak', () => {
    const r = drawdownStatus([pt('2026-01-01', 100_000), pt('2026-02-01', 150_000), pt('2026-03-01', 135_000)])!
    expect(r.atHigh).toBe(false)
    expect(r.drawdown).toBeCloseTo(0.1, 6)
    expect(r.high).toBe(150_000)
    expect(r.highDate).toBe('2026-02-01')
  })

  it('returns null for empty or never-positive series', () => {
    expect(drawdownStatus([])).toBeNull()
    expect(drawdownStatus([pt('2026-01-01', -5_000)])).toBeNull()
  })
})
