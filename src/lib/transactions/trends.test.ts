import { describe, it, expect } from 'vitest'
import { pctVsMedian, changeColor, trendIcon } from './trends'

describe('pctVsMedian', () => {
  it('returns null when the baseline is zero', () => {
    expect(pctVsMedian(100, 0)).toBeNull()
  })

  it('computes signed percentage change against the baseline', () => {
    expect(pctVsMedian(150, 100)).toBeCloseTo(0.5)
    expect(pctVsMedian(50, 100)).toBeCloseTo(-0.5)
  })
})

describe('changeColor', () => {
  it('treats a higher expense as unfavorable', () => {
    expect(changeColor('expense', 0.2)).toBe('text-error')
    expect(changeColor('expense', -0.2)).toBe('text-success')
  })

  it('treats higher income/savings/net as favorable', () => {
    expect(changeColor('income', 0.2)).toBe('text-success')
    expect(changeColor('savings', -0.2)).toBe('text-error')
  })

  it('is neutral for negligible or missing deltas', () => {
    expect(changeColor('expense', null)).toBe('text-muted')
    expect(changeColor('expense', 0.001)).toBe('text-muted')
  })
})

describe('trendIcon', () => {
  it('points up/down based on sign, minus when negligible or missing', () => {
    expect(trendIcon(0.1)).toBe('i-ph-arrow-up')
    expect(trendIcon(-0.1)).toBe('i-ph-arrow-down')
    expect(trendIcon(null)).toBe('i-ph-minus')
    expect(trendIcon(0.001)).toBe('i-ph-minus')
  })
})
