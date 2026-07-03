import { invoke } from '@tauri-apps/api/core'
import type { TransactionPage } from '../types/TransactionPage'
import type { Transaction } from '../types/Transaction'
import type { PeriodStats } from '../types/PeriodStats'

export type { PeriodStats }

export interface TransactionFilter {
  accountIds?: number[]
  types?: string[]
  categories?: string[]
  startDate?: string | null
  endDate?: string | null
  searchTerms?: string[]
  /** Rule-suppressed rows are hidden by default; set true to include them. */
  includeSuppressed?: boolean
  limit?: number | null
  offset?: number | null
}

export interface PeriodStatsFilter {
  accountIds?: number[]
  types?: string[]
  categories?: string[]
  searchTerms?: string[]
  /** "month" → YYYY-MM groups;  "year" → YYYY groups. */
  groupBy: 'month' | 'year'
  /** Current period to exclude so a period isn't compared against itself. */
  excludePeriod: string
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
  isWithdrawal: boolean
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
  isWithdrawal: boolean
  updateBalance: boolean
  updatedAt: string
}

export const listTransactions = (filter: TransactionFilter = {}) =>
  invoke<TransactionPage>('list_transactions_cmd', { filter })
export const periodStats = (filter: PeriodStatsFilter) =>
  invoke<PeriodStats[]>('period_stats_cmd', { filter })
export const getTransaction = (id: number) =>
  invoke<Transaction>('get_transaction_cmd', { id })
export const createTransaction = (transaction: NewTransaction) =>
  invoke<number>('create_transaction_cmd', { transaction })
export const updateTransaction = (transaction: UpdateTransaction) =>
  invoke<void>('update_transaction_cmd', { transaction })
export const deleteTransaction = (id: number) =>
  invoke<void>('delete_transaction_cmd', { id })
/** Delete only the txn row, leaving any generated balance snapshots in place
 *  as manual anchors. Used by the SimpleFIN duplicate review. */
export const deleteTransactionKeepSnapshot = (id: number) =>
  invoke<void>('delete_transaction_keep_snapshot_cmd', { id })
export const bulkCreateTransactions = (transactions: NewTransaction[]) =>
  invoke<number>('bulk_create_transactions_cmd', { transactions })
export const bulkCreateTransactionsWithSnapshots = (transactions: NewTransaction[]) =>
  invoke<number>('bulk_create_transactions_with_snapshots_cmd', { transactions })
