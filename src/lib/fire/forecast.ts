import { DateTime } from 'luxon'
import { fireNumber } from './metrics'
import { monthsToFire, projectedFiDate, realMonthlyReturn } from './projection'
import { coastStatus } from './coast'
import { requiredMonthlyContribution } from './requiredContribution'

export type FireVariant = 'lean' | 'regular' | 'fat'

export interface ForecastInputs {
  currentAge: number
  targetRetirementAge: number
  annualExpensesTarget: number
  leanFireAnnualExpenses: number | null
  fatFireAnnualExpenses: number | null
  expectedReturnRate: number
  inflationRate: number
  investable: number
  monthlyContribution: number
  /** Safe withdrawal rate; defaults to the 4% rule when omitted. */
  withdrawalRate?: number
}

export interface VariantForecast {
  variant: FireVariant
  expenses: number
  fireNumber: number
  fiDate: DateTime | null
  yearsToFi: number | null
  coastNumber: number
  coasting: boolean
  coastCrossingDate: DateTime | null
  requiredMonthly: number | null
}

export function variantExpenses(inputs: ForecastInputs, variant: FireVariant): number {
  if (variant === 'lean') return inputs.leanFireAnnualExpenses ?? inputs.annualExpensesTarget
  if (variant === 'fat') return inputs.fatFireAnnualExpenses ?? inputs.annualExpensesTarget
  return inputs.annualExpensesTarget
}

export function buildVariantForecast(
  inputs: ForecastInputs, variant: FireVariant, from: DateTime = DateTime.now(),
): VariantForecast {
  const expenses = variantExpenses(inputs, variant)
  const fireNum = fireNumber(expenses, inputs.withdrawalRate)
  const mr = realMonthlyReturn(inputs.expectedReturnRate, inputs.inflationRate)
  const months = monthsToFire(inputs.investable, inputs.monthlyContribution, mr, fireNum)
  const fiDate = projectedFiDate(
    inputs.investable, inputs.monthlyContribution,
    inputs.expectedReturnRate, inputs.inflationRate, fireNum, from,
  )
  const cs = coastStatus(
    inputs.investable, inputs.monthlyContribution, fireNum,
    inputs.currentAge, inputs.targetRetirementAge,
    inputs.expectedReturnRate, inputs.inflationRate, from,
  )
  const monthsToRetirement = (inputs.targetRetirementAge - inputs.currentAge) * 12
  const requiredMonthly = requiredMonthlyContribution(
    inputs.investable, fireNum,
    inputs.expectedReturnRate, inputs.inflationRate, monthsToRetirement,
  )
  return {
    variant, expenses, fireNumber: fireNum, fiDate,
    yearsToFi: months === null ? null : months / 12,
    coastNumber: cs.coastNumber, coasting: cs.coasting, coastCrossingDate: cs.crossingDate,
    requiredMonthly,
  }
}

export function buildForecast(inputs: ForecastInputs, from: DateTime = DateTime.now()): VariantForecast[] {
  const variants: FireVariant[] = ['lean', 'regular', 'fat']
  return variants.map(v => buildVariantForecast(inputs, v, from))
}
