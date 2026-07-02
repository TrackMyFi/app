import { describe, it, expect } from 'vitest'
import { DateTime } from 'luxon'
import { monthChange, yearToDateChange } from './change'

const now = DateTime.fromISO('2026-07-02')

describe('monthChange', () => {
  it('compares against the latest balance on/before the last day of the prior month', () => {
    const balances = [
      { id: 1, accountId: 1, balance: 1000, recordedAt: '2026-06-30' },
      { id: 2, accountId: 1, balance: 1200, recordedAt: '2026-07-01' },
    ]
    const result = monthChange(balances, 1, 1200, now)
    expect(result).toEqual({ amount: 200, percent: 0.2 })
  })

  it('ignores balances from other accounts', () => {
    const balances = [
      { id: 1, accountId: 2, balance: 5000, recordedAt: '2026-06-30' },
    ]
    expect(monthChange(balances, 1, 1200, now)).toBeNull()
  })

  it('returns null when no balance exists before the prior month end', () => {
    const balances = [
      { id: 1, accountId: 1, balance: 1200, recordedAt: '2026-07-01' },
    ]
    expect(monthChange(balances, 1, 1200, now)).toBeNull()
  })

  it('returns a null percent instead of dividing by zero when the prior balance was 0', () => {
    const balances = [
      { id: 1, accountId: 1, balance: 0, recordedAt: '2026-06-15' },
    ]
    const result = monthChange(balances, 1, 500, now)
    expect(result).toEqual({ amount: 500, percent: null })
  })

  it('breaks same-date ties by the higher id', () => {
    const balances = [
      { id: 1, accountId: 1, balance: 1800, recordedAt: '2026-06-30' },
      { id: 2, accountId: 1, balance: 1300, recordedAt: '2026-06-30' },
    ]
    const result = monthChange(balances, 1, 1300, now)
    expect(result).toEqual({ amount: 0, percent: 0 })
  })
})

describe('yearToDateChange', () => {
  it('compares against the latest balance on/before the last day of the prior year', () => {
    const balances = [
      { id: 1, accountId: 1, balance: 800, recordedAt: '2025-12-31' },
      { id: 2, accountId: 1, balance: 900, recordedAt: '2026-03-01' },
    ]
    const result = yearToDateChange(balances, 1, 1200, now)
    expect(result?.amount).toBe(400)
    expect(result?.percent).toBeCloseTo(0.5)
  })

  it('returns null for an account with no balance history from before this year', () => {
    const balances = [
      { id: 1, accountId: 1, balance: 1200, recordedAt: '2026-02-01' },
    ]
    expect(yearToDateChange(balances, 1, 1200, now)).toBeNull()
  })

  it('reports a negative amount and percent when the balance dropped', () => {
    const balances = [
      { id: 1, accountId: 1, balance: 2000, recordedAt: '2025-12-31' },
    ]
    const result = yearToDateChange(balances, 1, 1500, now)
    expect(result).toEqual({ amount: -500, percent: -0.25 })
  })
})
