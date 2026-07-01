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

  it("adds an inbound ('in') transfer row for an asset account (this account is the destination)", () => {
    const rows = [{ date: '2026-03-01', amount: 200, type: 'transfer', direction: 'in' as const }]
    expect(projectRunningBalances(rows, [true], 1000)).toEqual([1200])
  })

  it("an inbound direction is ignored for a liability account (still debt-reducing)", () => {
    const rows = [{ date: '2026-03-01', amount: 200, type: 'transfer', direction: 'in' as const }]
    expect(projectRunningBalances(rows, [true], 1000, true)).toEqual([800])
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

  it('cascades same-date rows in reverse of display order so the column reads consistently', () => {
    // Rendered newest-first: row 0 above row 1, both on the same date.
    // Each rendered row should differ from the one below it by its OWN delta:
    // balance[0] - balance[1] === signedDelta(row 0) === -100.
    const rows = [
      { date: '2026-03-01', amount: 100, type: 'expense' }, // displayed on top
      { date: '2026-03-01', amount: 50, type: 'expense' },  // displayed below
    ]
    // Bottom row (idx 1) processed first: 1000 - 50 = 950; then idx 0: 950 - 100 = 850.
    expect(projectRunningBalances(rows, [true, true], 1000)).toEqual([850, 950])
  })

  it('inverts income/expense and treats a payment as debt reduction for a liability', () => {
    // Balance is positive debt owed. Purchase raises it, refund lowers it, and a
    // payment (transfer) lowers it. Starting debt 500.
    const rows = [
      { date: '2026-03-01', amount: 40, type: 'expense' },   // purchase → +40 → 540
      { date: '2026-03-05', amount: 100, type: 'income' },   // refund   → -100 → 440
      { date: '2026-03-10', amount: 200, type: 'transfer' }, // payment  → -200 → 240
    ]
    expect(projectRunningBalances(rows, [true, true, true], 500, true)).toEqual([540, 440, 240])
  })
})
