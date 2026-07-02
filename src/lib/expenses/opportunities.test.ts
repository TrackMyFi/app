import { describe, it, expect } from 'vitest'
import { detectCategorySpikes } from './opportunities'

describe('detectCategorySpikes', () => {
  it('flags a category running meaningfully above typical', () => {
    const spikes = detectCategorySpikes({ fixed: 500, discretionary: 700 }, { fixed: 500, discretionary: 500 })
    expect(spikes).toEqual([{ category: 'discretionary', amount: 700, typical: 500, pct: 0.4 }])
  })

  it('ignores a small percentage swing on a tiny category (minDelta guard)', () => {
    const spikes = detectCategorySpikes({ fixed: 500, discretionary: 6 }, { fixed: 500, discretionary: 2 })
    expect(spikes).toHaveLength(0)
  })

  it('ignores decreases — those are already good news elsewhere', () => {
    const spikes = detectCategorySpikes({ fixed: 500, discretionary: 300 }, { fixed: 500, discretionary: 500 })
    expect(spikes).toHaveLength(0)
  })

  it('returns nothing without a typical baseline', () => {
    expect(detectCategorySpikes({ fixed: 500, discretionary: 700 }, null)).toEqual([])
  })

  it('sorts multiple spikes by percentage descending', () => {
    const spikes = detectCategorySpikes({ fixed: 700, discretionary: 900 }, { fixed: 500, discretionary: 500 })
    expect(spikes.map((s) => s.category)).toEqual(['discretionary', 'fixed'])
  })
})
