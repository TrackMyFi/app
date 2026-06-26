import type { Transaction } from '../types/Transaction'

export type BudgetLineItem = {
  total: number
  transactions: Transaction[]
}

export type PaycheckSummary = {
  grossIncome: number
  netIncome: number
  taxes: number
}

export type BudgetMonthSummary = {
  grossIncome: number
  netIncome: number
  income: BudgetLineItem
  savings: BudgetLineItem
  taxes: number
  fixed: BudgetLineItem
  discretionary: BudgetLineItem
  freeMoney: number
  freeMoneyRemaining: number
}

export type BudgetMonthTarget = {
  savingsTarget: number
  sourceYear: number
  sourceMonth: number
  isInherited: boolean
}

function sumAmount(txns: Transaction[]): number {
  return txns.reduce((acc, t) => acc + t.amount, 0)
}

// Savings is summed signed: a withdrawal (money pulled back out of investments)
// nets the savings total down rather than adding to it.
function sumSavings(txns: Transaction[]): number {
  return txns.reduce((acc, t) => acc + (t.isWithdrawal ? -t.amount : t.amount), 0)
}

export function buildBudgetMonth(txns: Transaction[], paycheckSummary: PaycheckSummary): BudgetMonthSummary {
  const savings = txns.filter((t) => t.isContribution === true)
  const nonPaycheckIncome = txns.filter((t) => t.type === 'income' && !t.isContribution && t.importSource !== 'paycheck')
  const fixed = txns.filter((t) => t.type === 'expense' && t.category === 'fixed' && !t.isContribution)
  const discretionary = txns.filter((t) => t.type === 'expense' && t.category === 'discretionary' && !t.isContribution)

  const incomeItem: BudgetLineItem = { total: sumAmount(nonPaycheckIncome), transactions: nonPaycheckIncome }
  const savingsItem: BudgetLineItem = { total: sumSavings(savings), transactions: savings }
  const fixedItem: BudgetLineItem = { total: sumAmount(fixed), transactions: fixed }
  const discretionaryItem: BudgetLineItem = { total: sumAmount(discretionary), transactions: discretionary }

  const nonPaycheckIncomeTotal = sumAmount(nonPaycheckIncome)
  const grossIncome = paycheckSummary.grossIncome + nonPaycheckIncomeTotal
  const netIncome = paycheckSummary.netIncome + nonPaycheckIncomeTotal
  const taxes = paycheckSummary.taxes
  const freeMoney = grossIncome - savingsItem.total - taxes - fixedItem.total
  const freeMoneyRemaining = freeMoney - discretionaryItem.total

  return {
    grossIncome,
    netIncome,
    income: incomeItem,
    savings: savingsItem,
    taxes,
    fixed: fixedItem,
    discretionary: discretionaryItem,
    freeMoney,
    freeMoneyRemaining,
  }
}
