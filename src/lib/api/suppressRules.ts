import { invoke } from '@tauri-apps/api/core'
import type { SuppressRule } from '../types/SuppressRule'

export const listSuppressRules = () =>
  invoke<SuppressRule[]>('list_suppress_rules_cmd')

/** Returns the created rule plus the total number of currently suppressed
 *  transactions (rules apply retroactively). */
export const createSuppressRule = (
  keyword: string,
  kind: string,
  accountId: number | null,
  createdAt: string,
) => invoke<[SuppressRule, number]>('create_suppress_rule_cmd', { keyword, kind, accountId, createdAt })

export const updateSuppressRule = (id: number, keyword: string, kind: string, accountId: number | null) =>
  invoke<void>('update_suppress_rule_cmd', { id, keyword, kind, accountId })

export const deleteSuppressRule = (id: number) =>
  invoke<void>('delete_suppress_rule_cmd', { id })
