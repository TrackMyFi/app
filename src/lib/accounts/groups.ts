import type { Account } from '../types/Account'
import { isLiability, isEquity } from '../accountTypes'

export type AccountGroupKey = 'budget' | 'tracking' | 'equity'

export interface AccountGroup {
  key: AccountGroupKey
  label: string
  accounts: Account[]
  total: number
}

/** A liability's balance is stored positive but reduces net worth. */
export const signedBalance = (account: Account, balance: number) =>
  isLiability(account.type) ? -balance : balance

/**
 * Splits active accounts into the same three buckets shown on the Accounts
 * page (FIRE / non-FIRE / equity), but labeled for the nav sidebar the way
 * YNAB labels its own account groups: accounts you actively budget day to
 * day ("Budget") vs. accounts you just track toward net worth ("Tracking").
 */
export function groupAccounts(
  accounts: Account[],
  balanceOf: (accountId: number) => number,
): AccountGroup[] {
  const active = accounts.filter(a => a.isActive)
  const byBalanceDesc = (a: Account, b: Account) => balanceOf(b.id) - balanceOf(a.id)

  const equity = active.filter(a => isEquity(a.type)).sort(byBalanceDesc)
  const tracking = active.filter(a => a.includeInFireCalculations && !isEquity(a.type)).sort(byBalanceDesc)
  const budget = active.filter(a => !a.includeInFireCalculations && !isEquity(a.type)).sort(byBalanceDesc)

  const total = (list: Account[]) =>
    list.reduce((s, a) => s + signedBalance(a, balanceOf(a.id)), 0)

  const groups: AccountGroup[] = [
    { key: 'budget', label: 'Budget', accounts: budget, total: total(budget) },
    { key: 'tracking', label: 'Tracking', accounts: tracking, total: total(tracking) },
    { key: 'equity', label: 'Equity', accounts: equity, total: total(equity) },
  ]
  return groups.filter(g => g.accounts.length > 0)
}

export function netWorth(accounts: Account[], balanceOf: (accountId: number) => number): number {
  return accounts
    .filter(a => a.isActive)
    .reduce((s, a) => s + signedBalance(a, balanceOf(a.id)), 0)
}
