import { invoke } from '@tauri-apps/api/core'
import type { Account } from '../types/Account'
import type { AccountBalance } from '../types/AccountBalance'
import type { BalanceMonthSummary } from '../types/BalanceMonthSummary'

export const listAccounts = () => invoke<Account[]>('list_accounts_cmd')
export const createAccount = (account: {
  name: string; type: string; institution: string | null;
  includeInFireCalculations: boolean; createdAt: string
}) => invoke<number>('create_account_cmd', { account })
export const archiveAccount = (id: number) => invoke<void>('archive_account_cmd', { id })
export const unarchiveAccount = (id: number) => invoke<void>('unarchive_account_cmd', { id })
export const deleteAccount = (id: number) => invoke<void>('delete_account_cmd', { id })
export const addBalance = (balance: { accountId: number; balance: number; recordedAt: string }) =>
  invoke<void>('add_balance_cmd', { balance })
export const updateAccount = (id: number, account: {
  name: string; type: string; institution: string | null;
  includeInFireCalculations: boolean; createdAt: string
}) => invoke<void>('update_account_cmd', { id, account })
export const updateBalance = (balance: { id: number; balance: number; recordedAt: string }) =>
  invoke<void>('update_balance_cmd', { balance })
export const deleteBalance = (id: number) => invoke<void>('delete_balance_cmd', { id })
export const listAllBalances = () => invoke<AccountBalance[]>('list_all_balances_cmd')

export const listLatestBalances = () =>
  invoke<AccountBalance[]>('list_latest_balances_cmd')

export const listBalanceMonthSummaries = (accountId: number) =>
  invoke<BalanceMonthSummary[]>('list_balance_month_summaries_cmd', { accountId })

export const listBalancesForMonth = (accountId: number, month: string) =>
  invoke<AccountBalance[]>('list_balances_for_month_cmd', { accountId, month })
