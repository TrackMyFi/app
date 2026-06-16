import { describe, it, expect } from 'vitest'
import { projectRunningBalances } from './balanceProjection'

describe('projectRunningBalances', () => {
  it('returns empty array for empty input', () => {
    expect(projectRunningBalances([], [], 1000)).toEqual([])
  })

  it('adds income to the running balance', () => {
    const rows = [
      { date: '2026-03-01', amount: 1000, type: 'income' },
      { date: '2026-03-15', amount: 500, type: 'income' },
    ]
    expect(projectRunningBalances(rows, [true, true], 200)).toEqual([1200, 1700])
  })

  it('subtracts expenses from the running balance', () => {
    const rows = [
      { date: '2026-03-01', amount: 100, type: 'expense' },
      { date: '2026-03-15', amount: 50, type: 'expense' },
    ]
    expect(projectRunningBalances(rows, [true, true], 1000)).toEqual([900, 850])
  })

  it('subtracts transfer rows (source account perspective)', () => {
    const rows = [{ date: '2026-03-01', amount: 200, type: 'transfer' }]
    expect(projectRunningBalances(rows, [true], 1000)).toEqual([800])
  })

  it('excluded rows return null and do not affect running total', () => {
    const rows = [
      { date: '2026-03-01', amount: 100, type: 'expense' },
      { date: '2026-03-15', amount: 50, type: 'expense' },
    ]
    // Row 0 excluded: running total skips it, row 1 sees base 1000
    expect(projectRunningBalances(rows, [false, true], 1000)).toEqual([null, 950])
  })

  it('sorts rows by date before cascading regardless of CSV order', () => {
    // CSV order: March 15 first, March 1 second
    const rows = [
      { date: '2026-03-15', amount: 50, type: 'expense' },
      { date: '2026-03-01', amount: 100, type: 'expense' },
    ]
    // Date-sorted processing: March 1 (-100 → 900), March 15 (-50 → 850)
    // Row 0 (March 15) shows its post-date balance: 850
    // Row 1 (March 1) shows its post-date balance: 900
    expect(projectRunningBalances(rows, [true, true], 1000)).toEqual([850, 900])
  })

  it('handles a mix of included and excluded rows in unsorted date order', () => {
    const rows = [
      { date: '2026-03-15', amount: 50, type: 'expense' },  // included
      { date: '2026-03-01', amount: 100, type: 'expense' }, // excluded
      { date: '2026-03-10', amount: 200, type: 'income' },  // included
    ]
    // Only included: March 10 (+200 → 700), March 15 (-50 → 650)
    expect(projectRunningBalances(rows, [true, false, true], 500)).toEqual([650, null, 700])
  })
})
