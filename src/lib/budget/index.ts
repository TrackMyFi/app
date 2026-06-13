import type { Transaction } from '../types/Transaction'

export type BudgetLineItem = {
  total: number
  transactions: Transaction[]
}

export type BudgetMonthSummary = {
  income: BudgetLineItem
  savings: BudgetLineItem
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

export function buildBudgetMonth(txns: Transaction[]): BudgetMonthSummary {
  const savings = txns.filter((t) => t.isContribution === true)
  const income = txns.filter((t) => t.type === 'income' && !t.isContribution)
  const fixed = txns.filter((t) => t.type === 'expense' && t.category === 'fixed' && !t.isContribution)
  const discretionary = txns.filter((t) => t.type === 'expense' && t.category === 'discretionary' && !t.isContribution)

  const incomeItem: BudgetLineItem = { total: sumAmount(income), transactions: income }
  const savingsItem: BudgetLineItem = { total: sumAmount(savings), transactions: savings }
  const fixedItem: BudgetLineItem = { total: sumAmount(fixed), transactions: fixed }
  const discretionaryItem: BudgetLineItem = { total: sumAmount(discretionary), transactions: discretionary }

  const freeMoney = incomeItem.total - savingsItem.total - fixedItem.total
  const freeMoneyRemaining = freeMoney - discretionaryItem.total

  return {
    income: incomeItem,
    savings: savingsItem,
    fixed: fixedItem,
    discretionary: discretionaryItem,
    freeMoney,
    freeMoneyRemaining,
  }
}
