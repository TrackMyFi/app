import { describe, it, expect } from 'vitest'
import { DateTime } from 'luxon'
import { crossoverStatus } from './crossover'
import { portfolioMonthlyEarnings } from './metrics'

const from = DateTime.fromISO('2026-01-01')

describe('crossoverStatus', () => {
  it('reports crossed when monthly earnings already exceed the contribution', () => {
    // $1M at 7% earns ~$5.7k/mo nominal, contribution $2k/mo
    const r = crossoverStatus(1_000_000, 2_000, 0.07, from)
    expect(r.crossed).toBe(true)
    expect(r.date).toBeNull()
  })

  it('agrees with portfolioMonthlyEarnings at the boundary', () => {
    const earnings = portfolioMonthlyEarnings(500_000, 0.07)
    expect(crossoverStatus(500_000, earnings, 0.07, from).crossed).toBe(true)
    expect(crossoverStatus(500_000, earnings + 1, 0.07, from).crossed).toBe(false)
  })

  it('projects a future crossing date when not yet crossed', () => {
    const r = crossoverStatus(100_000, 3_000, 0.07, from)
    expect(r.crossed).toBe(false)
    expect(r.date).not.toBeNull()
    expect(r.date!.toMillis()).toBeGreaterThan(from.toMillis())
  })

  it('crossing happens sooner with a smaller contribution to out-earn', () => {
    const small = crossoverStatus(100_000, 1_000, 0.07, from)
    const large = crossoverStatus(100_000, 5_000, 0.07, from)
    expect(small.date!.toMillis()).toBeLessThan(large.date!.toMillis())
  })

  it('is unreachable with a non-positive return', () => {
    const r = crossoverStatus(100_000, 2_000, 0, from)
    expect(r.crossed).toBe(false)
    expect(r.date).toBeNull()
  })

  it('is unreachable with nothing invested and nothing contributed', () => {
    const r = crossoverStatus(0, 0, 0.07, from)
    expect(r.crossed).toBe(false)
    expect(r.date).toBeNull()
  })
})
