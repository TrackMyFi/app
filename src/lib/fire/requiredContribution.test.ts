import { describe, it, expect } from 'vitest'
import { requiredMonthlyContribution } from './requiredContribution'

describe('requiredMonthlyContribution', () => {
  it('back-solves the annuity payment that reaches the target (verifies via forward FV)', () => {
    const pv = 100_000, target = 1_000_000, n = 240 // 20 years
    const pmt = requiredMonthlyContribution(pv, target, 0.07, 0, n)!
    // Forward-simulate FV with this pmt; should land on target.
    const r = Math.pow(1.07, 1 / 12) - 1
    let fv = pv
    for (let m = 0; m < n; m++) fv = fv * (1 + r) + pmt
    expect(fv).toBeCloseTo(target, 0)
  })

  it('returns 0 when the present value alone already reaches the target', () => {
    expect(requiredMonthlyContribution(2_000_000, 1_000_000, 0.07, 0, 240)).toBe(0)
  })

  it('handles ~zero real return with the linear formula', () => {
    // real return 0 → pmt = (target - pv) / n
    expect(requiredMonthlyContribution(0, 1200, 0.03, 0.03, 12)).toBeCloseTo(100, 6)
  })

  it('returns null when there is no time left (months <= 0)', () => {
    expect(requiredMonthlyContribution(0, 1_000_000, 0.07, 0, 0)).toBeNull()
    expect(requiredMonthlyContribution(0, 1_000_000, 0.07, 0, -12)).toBeNull()
  })
})
