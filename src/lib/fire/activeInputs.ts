import type { Account } from '../types/Account'
import type { AccountBalance } from '../types/AccountBalance'
import type { FireAccount, FireBalance } from './types'

/**
 * Map stored accounts/balances into FIRE-math inputs, EXCLUDING archived
 * (inactive) accounts and any balance snapshots belonging to them. Archived
 * accounts must not contribute to net worth, investable net worth, the
 * net-worth-over-time series, or the savings-rate approximation.
 */
export function activeFireInputs(
  accounts: Account[],
  balances: AccountBalance[],
): { accounts: FireAccount[]; balances: FireBalance[] } {
  const active = accounts.filter(a => a.isActive)
  const activeIds = new Set(active.map(a => a.id))
  return {
    accounts: active.map(a => ({
      id: a.id,
      type: a.type,
      includeInFireCalculations: a.includeInFireCalculations,
      traditionalPct: a.traditionalPct,
    })),
    balances: balances
      .filter(b => activeIds.has(b.accountId))
      .map(b => ({ id: b.id, accountId: b.accountId, balance: b.balance, recordedAt: b.recordedAt })),
  }
}
