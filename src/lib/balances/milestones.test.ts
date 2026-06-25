import { describe, it, expect } from 'vitest'
import { crossedMilestone } from './milestones'

describe('crossedMilestone', () => {
  it('returns the milestone when the balance crosses it upward', () => {
    expect(crossedMilestone(98_000, 101_000)).toBe(100_000)
  })

  it('returns the highest milestone when several are crossed at once', () => {
    expect(crossedMilestone(8_000, 120_000)).toBe(100_000)
  })

  it('returns null when no milestone sits in the gap', () => {
    expect(crossedMilestone(101_000, 109_000)).toBeNull()
  })

  it('treats landing exactly on a milestone as crossing it', () => {
    expect(crossedMilestone(99_999, 100_000)).toBe(100_000)
  })

  it('does not fire when already past the milestone', () => {
    expect(crossedMilestone(100_000, 110_000)).toBeNull()
  })

  it('ignores flat or falling balances', () => {
    expect(crossedMilestone(100_000, 100_000)).toBeNull()
    expect(crossedMilestone(120_000, 90_000)).toBeNull()
  })

  it('treats a null previous balance as zero (first snapshot)', () => {
    expect(crossedMilestone(null, 12_000)).toBe(10_000)
    expect(crossedMilestone(null, 500)).toBeNull()
  })

  it('keeps celebrating each million past the first', () => {
    expect(crossedMilestone(1_900_000, 2_010_000)).toBe(2_000_000)
  })
})
