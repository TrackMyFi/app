import { describe, it, expect } from 'vitest'
import { fireNumber, latestBalances, currentNetWorth, investableNetWorth, fiProgress, journeyProgress, portfolioMonthlyEarnings } from './metrics'
import type { FireAccount, FireBalance } from './types'

const accounts: FireAccount[] = [
  { id: 1, type: 'brokerage', includeInFireCalculations: true },
  { id: 2, type: 'checking', includeInFireCalculations: false },
  { id: 3, type: 'liability', includeInFireCalculations: false },
]
const balances: FireBalance[] = [
  { accountId: 1, balance: 100, recordedAt: '2026-01-01' },
  { accountId: 1, balance: 200, recordedAt: '2026-02-01' },
  { accountId: 2, balance: 50, recordedAt: '2026-01-15' },
  { accountId: 3, balance: 30, recordedAt: '2026-01-10' },
]

describe('fire metrics', () => {
  it('fireNumber is 25x expenses at the default 4% rate', () => { expect(fireNumber(40000)).toBe(1_000_000) })
  it('fireNumber scales with a custom withdrawal rate', () => {
    expect(fireNumber(40000, 0.035)).toBeCloseTo(40000 / 0.035, 6)
    expect(fireNumber(40000, 0)).toBe(0) // degenerate rate never divides by zero
  })
  it('latestBalances picks most recent per account', () => {
    const m = latestBalances(balances)
    expect(m.get(1)).toBe(200); expect(m.get(2)).toBe(50)
  })
  it('currentNetWorth subtracts liabilities', () => { expect(currentNetWorth(accounts, balances)).toBe(220) })
  it('investableNetWorth counts only included accounts', () => { expect(investableNetWorth(accounts, balances)).toBe(200) })
  it('fiProgress is investable / fireNumber * 100', () => { expect(fiProgress(200_000, 1_000_000)).toBe(20) })

  it('journeyProgress returns null when target is 0', () => {
    expect(journeyProgress(0, 2000, 0.07, 0.03, 0)).toBeNull()
  })
  it('journeyProgress returns 100 when already at target', () => {
    expect(journeyProgress(1_000_000, 2000, 0.07, 0.03, 1_000_000)).toBe(100)
  })
  it('journeyProgress is greater than fiProgress due to compounding', () => {
    const jp = journeyProgress(500_000, 2000, 0.07, 0.03, 1_000_000)
    expect(jp).not.toBeNull()
    expect(jp!).toBeGreaterThan(50) // compounding means you're further than 50% in time
    expect(jp!).toBeLessThan(100)
  })
  it('journeyProgress returns null when contributions can never reach target', () => {
    expect(journeyProgress(0, 0, 0, 0, 1_000_000)).toBeNull()
  })

  it('portfolioMonthlyEarnings is zero when balance is zero', () => {
    expect(portfolioMonthlyEarnings(0, 0.07)).toBe(0)
  })
  it('portfolioMonthlyEarnings compounds correctly at 7% annual', () => {
    const monthly = portfolioMonthlyEarnings(500_000, 0.07)
    expect(monthly).toBeCloseTo(500_000 * (Math.pow(1.07, 1 / 12) - 1), 2)
    expect(monthly).toBeGreaterThan(2800) // sanity: ~$2,834/mo at 7% on $500k
  })
})
