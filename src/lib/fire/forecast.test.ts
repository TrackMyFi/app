import { describe, it, expect } from 'vitest'
import { DateTime } from 'luxon'
import { buildForecast, variantExpenses, type ForecastInputs } from './forecast'

const from = DateTime.fromISO('2026-01-01')

const base: ForecastInputs = {
  currentAge: 40,
  targetRetirementAge: 55,
  annualExpensesTarget: 60_000,
  leanFireAnnualExpenses: 40_000,
  fatFireAnnualExpenses: 100_000,
  expectedReturnRate: 0.07,
  inflationRate: 0.03,
  investable: 300_000,
  monthlyContribution: 2_000,
}

describe('variantExpenses', () => {
  it('picks lean/regular/fat expenses', () => {
    expect(variantExpenses(base, 'lean')).toBe(40_000)
    expect(variantExpenses(base, 'regular')).toBe(60_000)
    expect(variantExpenses(base, 'fat')).toBe(100_000)
  })
  it('falls back to annualExpensesTarget when a variant field is null', () => {
    const noLean = { ...base, leanFireAnnualExpenses: null, fatFireAnnualExpenses: null }
    expect(variantExpenses(noLean, 'lean')).toBe(60_000)
    expect(variantExpenses(noLean, 'fat')).toBe(60_000)
  })
})

describe('buildForecast', () => {
  it('produces three variants with the expected FIRE numbers', () => {
    const f = buildForecast(base, from)
    expect(f.map(v => v.variant)).toEqual(['lean', 'regular', 'fat'])
    expect(f[0].fireNumber).toBe(40_000 * 25)
    expect(f[1].fireNumber).toBe(60_000 * 25)
    expect(f[2].fireNumber).toBe(100_000 * 25)
  })

  it('lean reaches FI no later than fat', () => {
    const f = buildForecast(base, from)
    const lean = f[0].fiDate, fat = f[2].fiDate
    expect(lean).not.toBeNull()
    if (lean && fat) expect(lean.toMillis()).toBeLessThanOrEqual(fat.toMillis())
  })

  it('fills coast and required-contribution fields for each variant', () => {
    const f = buildForecast(base, from)
    for (const v of f) {
      expect(typeof v.coastNumber).toBe('number')
      expect(typeof v.coasting).toBe('boolean')
      expect(v.requiredMonthly === null || typeof v.requiredMonthly === 'number').toBe(true)
    }
  })
})
