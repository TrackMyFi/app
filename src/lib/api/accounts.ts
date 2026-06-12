import { invoke } from '@tauri-apps/api/core'
import type { Account } from '../types/Account'
import type { AccountBalance } from '../types/AccountBalance'

export const listAccounts = () => invoke<Account[]>('list_accounts_cmd')
export const createAccount = (account: {
  name: string; type: string; institution: string | null;
  includeInFireCalculations: boolean; createdAt: string
}) => invoke<number>('create_account_cmd', { account })
export const archiveAccount = (id: number) => invoke<void>('archive_account_cmd', { id })
export const addBalance = (balance: { accountId: number; balance: number; recordedAt: string }) =>
  invoke<void>('add_balance_cmd', { balance })
export const listAllBalances = () => invoke<AccountBalance[]>('list_all_balances_cmd')
