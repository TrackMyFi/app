import type { HsaExpense } from '../types/HsaExpense'

/** Total of every logged expense, reimbursed or not. */
export function totalExpenses(expenses: HsaExpense[]): number {
  return expenses.reduce((sum, e) => sum + e.amount, 0)
}

/**
 * Total of expenses not yet reimbursed — the "receipt bank": the amount that
 * can still be withdrawn from an HSA tax-free at any time in the future.
 */
export function unreimbursedTotal(expenses: HsaExpense[]): number {
  return expenses.filter((e) => !e.reimbursed).reduce((sum, e) => sum + e.amount, 0)
}

/** Total already taken out of an HSA against these expenses. */
export function reimbursedTotal(expenses: HsaExpense[]): number {
  return expenses.filter((e) => e.reimbursed).reduce((sum, e) => sum + e.amount, 0)
}
