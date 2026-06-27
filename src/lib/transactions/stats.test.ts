import { describe, it, expect } from 'vitest'
import { median, computeMedian } from './stats'
import type { PeriodStats } from './stats'

function period(overrides: Partial<PeriodStats> & { period: string }): PeriodStats {
  return {
    income: 0,
    expense: 0,
    savings: 0,
    net: 0,
    catFixed: 0,
    catDiscretionary: 0,
    catUncategorized: 0,
    ...overrides,
  }
}

describe('median', () => {
  it('returns 0 for empty array', () => {
    expect(median([])).toBe(0)
  })

  it('returns the single element', () => {
    expect(median([5])).toBe(5)
  })

  it('returns middle for odd count', () => {
    expect(median([3, 1, 2])).toBe(2)
  })

  it('returns average of two middle for even count', () => {
    expect(median([4, 1, 3, 2])).toBe(2.5)
  })
})

describe('computeMedian', () => {
  it('returns null for empty input', () => {
    expect(computeMedian([])).toBeNull()
  })

  it('returns correct periodCount', () => {
    const result = computeMedian([
      period({ period: '2025-01', income: 1000 }),
      period({ period: '2025-02', income: 2000 }),
    ])
    expect(result!.periodCount).toBe(2)
  })

  it('computes median income across periods', () => {
    const result = computeMedian([
      period({ period: '2025-01', income: 1000 }),
      period({ period: '2025-02', income: 2000 }),
      period({ period: '2025-03', income: 3000 }),
    ])
    expect(result!.totals.income).toBe(2000)
  })

  it('computes median expense across periods', () => {
    const result = computeMedian([
      period({ period: '2025-01', expense: 500 }),
      period({ period: '2025-02', expense: 1500 }),
    ])
    expect(result!.totals.expense).toBe(1000)
  })

  it('computes per-category medians independently', () => {
    const result = computeMedian([
      period({ period: '2025-01', catFixed: 200, catDiscretionary: 100, catUncategorized: 50 }),
      period({ period: '2025-02', catFixed: 400, catDiscretionary: 300, catUncategorized: 150 }),
    ])
    expect(result!.breakdown.byCategory.get('fixed')).toBe(300)
    expect(result!.breakdown.byCategory.get('discretionary')).toBe(200)
    expect(result!.breakdown.byCategory.get('uncategorized')).toBe(100)
  })

  it('includes savings in breakdown byCategory', () => {
    const result = computeMedian([
      period({ period: '2025-01', savings: 500 }),
      period({ period: '2025-02', savings: 1500 }),
    ])
    expect(result!.breakdown.byCategory.get('savings')).toBe(1000)
  })

  it('breakdown income matches totals income', () => {
    const result = computeMedian([
      period({ period: '2025-01', income: 3000 }),
      period({ period: '2025-02', income: 5000 }),
    ])
    expect(result!.breakdown.income).toBe(result!.totals.income)
  })
})
