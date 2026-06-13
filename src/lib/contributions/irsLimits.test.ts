import { describe, it, expect } from 'vitest'
import { resolveYearLimits } from './irsLimits'

describe('resolveYearLimits', () => {
  it('returns exact limits for a known year, not estimated', () => {
    const { limits, estimated } = resolveYearLimits(2025)
    expect(limits.k401).toBe(23500)
    expect(limits.ira).toBe(7000)
    expect(limits.hsaSelf).toBe(4300)
    expect(limits.hsaFamily).toBe(8550)
    expect(estimated).toBe(false)
  })

  it('falls back to most recent known year for a future year, flagged estimated', () => {
    const { limits, estimated, estimatedFrom } = resolveYearLimits(2099)
    expect(limits.k401).toBe(23500) // 2026 values (latest known)
    expect(estimated).toBe(true)
    expect(estimatedFrom).toBe(2026)
  })

  it('falls back to the oldest known year for a year older than all entries', () => {
    const { limits, estimated, estimatedFrom } = resolveYearLimits(2000)
    expect(limits.k401).toBe(23000) // 2024 values (oldest known)
    expect(estimated).toBe(true)
    expect(estimatedFrom).toBe(2024)
  })
})
