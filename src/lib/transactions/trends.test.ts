import { describe, it, expect } from 'vitest'
import { DateTime } from 'luxon'
import { pctVsMedian, changeColor, trendIcon, proratedSuffix } from './trends'

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

describe('proratedSuffix', () => {
  const now = DateTime.fromISO('2026-07-03')

  it('is empty for a completed period', () => {
    expect(proratedSuffix('month', DateTime.fromISO('2026-06-01'), now)).toBe('')
    expect(proratedSuffix('year', DateTime.fromISO('2025-06-01'), now)).toBe('')
  })

  it('notes the day for an in-progress month', () => {
    expect(proratedSuffix('month', DateTime.fromISO('2026-07-01'), now)).toBe(' by the 3rd')
    expect(proratedSuffix('month', DateTime.fromISO('2026-07-01'), DateTime.fromISO('2026-07-21'))).toBe(' by the 21st')
    expect(proratedSuffix('month', DateTime.fromISO('2026-07-01'), DateTime.fromISO('2026-07-11'))).toBe(' by the 11th')
  })

  it('notes month and day for an in-progress year', () => {
    expect(proratedSuffix('year', DateTime.fromISO('2026-01-01'), now)).toBe(' by Jul 3')
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
