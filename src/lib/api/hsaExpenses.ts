import { invoke } from '@tauri-apps/api/core'
import type { HsaExpense } from '../types/HsaExpense'

export interface HsaExpenseFilter {
  accountId?: number | null
  category?: string | null
  reimbursed?: boolean | null
  startDate?: string | null
  endDate?: string | null
  search?: string | null
}

export interface NewHsaExpense {
  accountId?: number | null
  date: string
  description: string
  category: string
  amount: number
  person?: string | null
  provider?: string | null
  notes?: string | null
  reimbursed: boolean
  reimbursedDate?: string | null
  createdAt: string
}

export interface UpdateHsaExpense {
  id: number
  accountId?: number | null
  date: string
  description: string
  category: string
  amount: number
  person?: string | null
  provider?: string | null
  notes?: string | null
  reimbursed: boolean
  reimbursedDate?: string | null
  updatedAt: string
}

export const listHsaExpenses = (filter: HsaExpenseFilter = {}) =>
  invoke<HsaExpense[]>('list_hsa_expenses_cmd', { filter })
export const getHsaExpense = (id: number) =>
  invoke<HsaExpense>('get_hsa_expense_cmd', { id })
export const createHsaExpense = (expense: NewHsaExpense) =>
  invoke<HsaExpense>('create_hsa_expense_cmd', { expense })
export const updateHsaExpense = (expense: UpdateHsaExpense) =>
  invoke<HsaExpense>('update_hsa_expense_cmd', { expense })
export const deleteHsaExpense = (id: number) =>
  invoke<void>('delete_hsa_expense_cmd', { id })
