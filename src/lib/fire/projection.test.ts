import { describe, it, expect } from 'vitest'
import { DateTime } from 'luxon'
import { realMonthlyReturn, monthsToFire, savingsRate, projectionSeries } from './projection'
import type { FireAccount, FireBalance } from './types'

describe('projectionSeries', () => {
  const from = DateTime.fromISO('2026-01-01')

  it('emits months+1 points starting at the present value', () => {
    const pts = projectionSeries(1000, 100, 0.07, 0, 3, from)
    expect(pts).toHaveLength(4)
    expect(pts[0]).toEqual({ date: '2026-01-01', value: 1000, growth: 0 })
  })

  it('compounds each month then adds the contribution', () => {
    const r = Math.pow(1.07, 1 / 12) - 1
    const pts = projectionSeries(1000, 100, 0.07, 0, 1, from)
    expect(pts[1].growth).toBeCloseTo(1000 * r, 6)
    expect(pts[1].value).toBeCloseTo(1000 * (1 + r) + 100, 6)
    expect(pts[1].date).toBe('2026-02-01')
  })
})

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
