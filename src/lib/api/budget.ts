import { invoke } from '@tauri-apps/api/core'
import type { Transaction } from '../types/Transaction'

export type BudgetMonth = { year: number; month: number }

export type RawBudgetMonthTarget = {
  savingsTarget: number
  sourceYear: number
  sourceMonth: number
}

export const listBudgetMonths = (): Promise<BudgetMonth[]> =>
  invoke('list_budget_months_cmd')

export const listBudgetTxns = (year: number, month: number): Promise<Transaction[]> =>
  invoke('list_budget_txns_cmd', { year, month })

export const getBudgetMonthTarget = (year: number, month: number): Promise<RawBudgetMonthTarget | null> =>
  invoke('get_budget_month_target_cmd', { year, month })

export const setBudgetMonthTarget = (year: number, month: number, savingsTarget: number): Promise<void> =>
  invoke('set_budget_month_target_cmd', { year, month, savingsTarget })

export type BudgetPaycheckSummary = {
  grossIncome: number
  netIncome: number
  taxes: number
}

export const getBudgetPaycheckSummary = (year: number, month: number): Promise<BudgetPaycheckSummary> =>
  invoke('get_budget_paycheck_summary_cmd', { year, month })
