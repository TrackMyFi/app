import { invoke } from '@tauri-apps/api/core'
import type { Transaction } from '../types/Transaction'

export const listContributionTxns = (year: number) =>
  invoke<Transaction[]>('list_contribution_txns_cmd', { year })

export const listAllContributionTxns = () =>
  invoke<Transaction[]>('list_all_contribution_txns_cmd')

export const listContributionYears = () =>
  invoke<string[]>('list_contribution_years_cmd')
