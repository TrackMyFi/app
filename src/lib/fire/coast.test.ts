import { describe, it, expect } from 'vitest'
import { DateTime } from 'luxon'
import { realAnnualReturn, coastFireNumber, coastStatus } from './coast'

const from = DateTime.fromISO('2026-01-01')

describe('realAnnualReturn', () => {
  it('with zero inflation equals the nominal return', () => {
    expect(realAnnualReturn(0.07, 0)).toBeCloseTo(0.07, 6)
  })
  it('is reduced by inflation', () => {
    expect(realAnnualReturn(0.07, 0.03)).toBeCloseTo((1.07 / 1.03) - 1, 6)
  })
})

describe('coastFireNumber', () => {
  it('discounts the FIRE number back over years to retirement at the real return', () => {
    // 7% real, 10 years: 1_000_000 / 1.07^10
    const expected = 1_000_000 / Math.pow(1.07, 10)
    expect(coastFireNumber(1_000_000, 40, 50, 0.07, 0)).toBeCloseTo(expected, 2)
  })
  it('returns the FIRE number when already at/past retirement age', () => {
    expect(coastFireNumber(1_000_000, 50, 50, 0.07, 0)).toBe(1_000_000)
    expect(coastFireNumber(1_000_000, 55, 50, 0.07, 0)).toBe(1_000_000)
  })
})

describe('coastStatus', () => {
  it('reports coasting when investable already meets the coast number', () => {
    const coastNum = coastFireNumber(1_000_000, 40, 50, 0.07, 0)
    const r = coastStatus(coastNum + 1, 0, 1_000_000, 40, 50, 0.07, 0, from)
    expect(r.coasting).toBe(true)
    expect(r.crossingDate).toBeNull()
    expect(r.coastNumber).toBeCloseTo(coastNum, 2)
  })

  it('projects a future crossing date when not yet coasting', () => {
    const r = coastStatus(100_000, 2_000, 1_000_000, 40, 50, 0.07, 0, from)
    expect(r.coasting).toBe(false)
    expect(r.crossingDate).not.toBeNull()
    expect(r.crossingDate!.toMillis()).toBeGreaterThan(from.toMillis())
  })

  it('returns a null crossing date when the coast number is unreachable', () => {
    const r = coastStatus(0, 0, 1_000_000, 40, 50, 0.07, 0, from)
    expect(r.coasting).toBe(false)
    expect(r.crossingDate).toBeNull()
  })
})
