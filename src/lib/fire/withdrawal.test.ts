import { describe, it, expect } from 'vitest'
import { SAFE_WITHDRAWAL_RATE, safeMonthlyWithdrawal } from './withdrawal'
import { fireNumber } from './metrics'

describe('safeMonthlyWithdrawal', () => {
  it('pays 4% of the portfolio per year, split monthly', () => {
    expect(safeMonthlyWithdrawal(1_200_000)).toBeCloseTo(4_000, 6)
  })

  it('pays nothing when the portfolio is underwater', () => {
    expect(safeMonthlyWithdrawal(-50_000)).toBe(0)
    expect(safeMonthlyWithdrawal(0)).toBe(0)
  })

  it('accepts a custom withdrawal rate', () => {
    expect(safeMonthlyWithdrawal(1_200_000, 0.035)).toBeCloseTo(3_500, 6)
  })

  it('at the FIRE number, pays exactly the target annual expenses', () => {
    const expenses = 40_000
    expect(safeMonthlyWithdrawal(fireNumber(expenses)) * 12).toBeCloseTo(expenses, 6)
  })

  it('stays the inverse of the ×25 FIRE multiplier', () => {
    expect(SAFE_WITHDRAWAL_RATE).toBeCloseTo(1 / 25, 10)
  })
})
