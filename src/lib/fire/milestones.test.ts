import { describe, it, expect } from 'vitest'
import { DateTime } from 'luxon'
import { buildMilestones, type MilestoneInputs } from './milestones'

const from = DateTime.fromISO('2026-01-01')

const base: MilestoneInputs = {
  investable: 250_000,
  monthlyContribution: 2_000,
  expectedReturnRate: 0.07,
  inflationRate: 0.03,
  annualExpensesTarget: 40_000, // FIRE number $1M
  leanFireAnnualExpenses: 28_000, // lean $700k
  fatFireAnnualExpenses: 60_000, // fat $1.5M
  coastNumber: 400_000,
}

describe('buildMilestones', () => {
  it('returns the full ladder sorted by target', () => {
    const ms = buildMilestones(base, from)
    expect(ms.map(m => m.key)).toEqual(['first100k', 'coast', 'half', 'lean', 'fire', 'fat'])
    const targets = ms.map(m => m.target)
    expect(targets).toEqual([...targets].sort((a, b) => a - b))
    expect(ms.find(m => m.key === 'fire')!.target).toBe(1_000_000)
    expect(ms.find(m => m.key === 'lean')!.target).toBe(700_000)
  })

  it('marks passed milestones achieved with no projected date', () => {
    const ms = buildMilestones(base, from)
    const first = ms.find(m => m.key === 'first100k')!
    expect(first.achieved).toBe(true)
    expect(first.projectedDate).toBeNull()
  })

  it('projects future dates for unachieved milestones, later for higher targets', () => {
    const ms = buildMilestones(base, from)
    const pending = ms.filter(m => !m.achieved)
    expect(pending.length).toBeGreaterThan(0)
    for (const m of pending) {
      expect(m.projectedDate).not.toBeNull()
      expect(m.projectedDate!.toMillis()).toBeGreaterThan(from.toMillis())
    }
    const dates = pending.map(m => m.projectedDate!.toMillis())
    expect(dates).toEqual([...dates].sort((a, b) => a - b))
  })

  it('omits lean/fat/coast milestones that are missing or degenerate', () => {
    const ms = buildMilestones({
      ...base,
      leanFireAnnualExpenses: null,
      fatFireAnnualExpenses: null,
      coastNumber: null,
    }, from)
    expect(ms.map(m => m.key)).toEqual(['first100k', 'half', 'fire'])
    // Lean above regular / coast at-or-above the FIRE number would duplicate the ladder
    const degenerate = buildMilestones({
      ...base,
      leanFireAnnualExpenses: 45_000,
      coastNumber: 1_000_000,
    }, from)
    expect(degenerate.find(m => m.key === 'lean')).toBeUndefined()
    expect(degenerate.find(m => m.key === 'coast')).toBeUndefined()
  })

  it('omits First $100k when the FIRE number is at or below $100k', () => {
    const ms = buildMilestones({ ...base, annualExpensesTarget: 4_000 }, from)
    expect(ms.find(m => m.key === 'first100k')).toBeUndefined()
  })

  it('scales every target by a custom withdrawal rate', () => {
    const ms = buildMilestones({ ...base, withdrawalRate: 0.035, coastNumber: null }, from)
    expect(ms.find(m => m.key === 'fire')!.target).toBeCloseTo(40_000 / 0.035, 6)
    expect(ms.find(m => m.key === 'lean')!.target).toBeCloseTo(28_000 / 0.035, 6)
    expect(ms.find(m => m.key === 'half')!.target).toBeCloseTo(40_000 / 0.035 / 2, 6)
  })

  it('returns an empty ladder without an expenses target', () => {
    expect(buildMilestones({ ...base, annualExpensesTarget: 0 }, from)).toEqual([])
  })

  it('marks unreachable milestones with a null date', () => {
    const ms = buildMilestones({ ...base, investable: 0, monthlyContribution: 0, expectedReturnRate: 0, inflationRate: 0 }, from)
    for (const m of ms) {
      expect(m.achieved).toBe(false)
      expect(m.projectedDate).toBeNull()
    }
  })
})
