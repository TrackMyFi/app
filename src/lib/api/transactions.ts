import { invoke } from '@tauri-apps/api/core'
import type { TransactionPage } from '../types/TransactionPage'

export interface TransactionFilter {
  accountId?: number | null
  type?: string | null
  category?: string | null
  startDate?: string | null
  endDate?: string | null
  search?: string | null
  limit?: number | null
  offset?: number | null
}

export interface NewTransaction {
  accountId: number
  transferAccountId: number | null
  amount: number
  description: string
  date: string
  type: string
  category: string
  isContribution: boolean
  importSource: string
  updateBalance: boolean
  createdAt: string
}

export interface UpdateTransaction {
  id: number
  accountId: number
  transferAccountId: number | null
  amount: number
  description: string
  date: string
  type: string
  category: string
  isContribution: boolean
  updateBalance: boolean
  updatedAt: string
}

export const listTransactions = (filter: TransactionFilter = {}) =>
  invoke<TransactionPage>('list_transactions_cmd', { filter })
export const createTransaction = (transaction: NewTransaction) =>
  invoke<number>('create_transaction_cmd', { transaction })
export const updateTransaction = (transaction: UpdateTransaction) =>
  invoke<void>('update_transaction_cmd', { transaction })
export const deleteTransaction = (id: number) =>
  invoke<void>('delete_transaction_cmd', { id })
export const bulkCreateTransactions = (transactions: NewTransaction[]) =>
  invoke<number>('bulk_create_transactions_cmd', { transactions })
