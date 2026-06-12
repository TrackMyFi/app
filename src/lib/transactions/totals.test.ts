import { describe, it, expect } from 'vitest'
import { runningTotals } from './totals'

describe('runningTotals', () => {
  it('sums income and expense, ignores transfers, computes net', () => {
    const rows = [
      { type: 'income', amount: 1000 },
      { type: 'expense', amount: 40 },
      { type: 'expense', amount: 60 },
      { type: 'transfer', amount: 500 },
    ]
    expect(runningTotals(rows)).toEqual({ income: 1000, expense: 100, net: 900 })
  })

  it('returns zeros for an empty set', () => {
    expect(runningTotals([])).toEqual({ income: 0, expense: 0, net: 0 })
  })
})
