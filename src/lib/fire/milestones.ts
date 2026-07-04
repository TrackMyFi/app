import { DateTime } from 'luxon'
import { fireNumber } from './metrics'
import { projectedFiDate } from './projection'

export type MilestoneKey = 'first100k' | 'coast' | 'lean' | 'half' | 'fire' | 'fat'

export interface Milestone {
  key: MilestoneKey
  label: string
  target: number
  achieved: boolean
  /** Projected crossing date on the current trajectory; null if achieved or unreachable. */
  projectedDate: DateTime | null
}

export interface MilestoneInputs {
  investable: number
  monthlyContribution: number
  expectedReturnRate: number
  inflationRate: number
  annualExpensesTarget: number
  leanFireAnnualExpenses: number | null
  fatFireAnnualExpenses: number | null
  /** From `coastStatus`; null when date of birth is unknown. */
  coastNumber: number | null
  /** Safe withdrawal rate; defaults to the 4% rule when omitted. */
  withdrawalRate?: number
}

/**
 * The ladder of FIRE milestones between $0 and beyond the FIRE number, sorted
 * by target. Milestones that don't apply (no lean/fat targets, coast number
 * unknown or degenerate) are omitted rather than shown as zero.
 */
export function buildMilestones(inputs: MilestoneInputs, from: DateTime = DateTime.now()): Milestone[] {
  const fireNum = fireNumber(inputs.annualExpensesTarget, inputs.withdrawalRate)
  if (fireNum <= 0) return []

  const candidates: { key: MilestoneKey; label: string; target: number }[] = [
    { key: 'half', label: 'Halfway to FI', target: fireNum / 2 },
    { key: 'fire', label: 'Financial Independence', target: fireNum },
  ]
  if (fireNum > 100_000) {
    candidates.push({ key: 'first100k', label: 'First $100k', target: 100_000 })
  }
  if (inputs.coastNumber !== null && inputs.coastNumber > 0 && inputs.coastNumber < fireNum) {
    candidates.push({ key: 'coast', label: 'Coast FI', target: inputs.coastNumber })
  }
  const lean = inputs.leanFireAnnualExpenses
  if (lean !== null && lean > 0 && fireNumber(lean, inputs.withdrawalRate) < fireNum) {
    candidates.push({ key: 'lean', label: 'Lean FI', target: fireNumber(lean, inputs.withdrawalRate) })
  }
  const fat = inputs.fatFireAnnualExpenses
  if (fat !== null && fat > 0 && fireNumber(fat, inputs.withdrawalRate) > fireNum) {
    candidates.push({ key: 'fat', label: 'Fat FI', target: fireNumber(fat, inputs.withdrawalRate) })
  }

  return candidates
    .sort((a, b) => a.target - b.target)
    .map((c) => {
      const achieved = inputs.investable >= c.target
      return {
        ...c,
        achieved,
        projectedDate: achieved ? null : projectedFiDate(
          inputs.investable, inputs.monthlyContribution,
          inputs.expectedReturnRate, inputs.inflationRate, c.target, from,
        ),
      }
    })
}
