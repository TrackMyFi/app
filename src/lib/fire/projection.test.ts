import { describe, it, expect } from 'vitest'
import { realMonthlyReturn, monthsToFire, savingsRate } from './projection'
import type { FireAccount, FireBalance } from './types'

describe('projection', () => {
  it('realMonthlyReturn deflates nominal by inflation', () => {
    expect(realMonthlyReturn(0.07, 0.03)).toBeCloseTo(0.003180, 5)
  })
  it('monthsToFire returns 0 when already at the number', () => {
    expect(monthsToFire(1_000_000, 1000, 0.003, 1_000_000)).toBe(0)
  })
  it('monthsToFire grows the portfolio until it reaches the target', () => {
    expect(monthsToFire(0, 100, 0, 10_000)).toBe(100)
  })
  it('monthsToFire returns null when unreachable within cap', () => {
    expect(monthsToFire(0, 0, 0, 10_000)).toBeNull()
  })
  it('savingsRate divides trailing-12mo investment increase by income', () => {
    const accounts: FireAccount[] = [
      { id: 1, type: 'brokerage', includeInFireCalculations: true },
      { id: 2, type: 'checking', includeInFireCalculations: false },
    ]
    const balances: FireBalance[] = [
      { accountId: 1, balance: 10_000, recordedAt: '2025-06-01' },
      { accountId: 1, balance: 30_000, recordedAt: '2026-06-01' },
      { accountId: 2, balance: 5_000, recordedAt: '2026-06-01' },
    ]
    expect(savingsRate(accounts, balances, 100_000, '2026-06-01')).toBeCloseTo(0.2, 6)
  })
})
