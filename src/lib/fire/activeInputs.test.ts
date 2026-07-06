import { describe, it, expect } from 'vitest'
import { activeFireInputs } from './activeInputs'
import type { Account } from '../types/Account'
import type { AccountBalance } from '../types/AccountBalance'

const accounts: Account[] = [
  { id: 1, name: 'Brokerage', type: 'brokerage', institution: null, isActive: true, includeInFireCalculations: true, createdAt: '2026-01-01', simplefinId: null, countPaymentsAsExpense: false, traditionalPct: null },
  { id: 2, name: 'Old 401k', type: '401k', institution: null, isActive: false, includeInFireCalculations: true, createdAt: '2026-01-01', simplefinId: null, countPaymentsAsExpense: false, traditionalPct: null },
]
const balances: AccountBalance[] = [
  { id: 1, accountId: 1, balance: 1000, recordedAt: '2026-01-01', linkedTransactionId: null, source: 'manual' },
  { id: 2, accountId: 2, balance: 5000, recordedAt: '2026-01-01', linkedTransactionId: null, source: 'manual' }, // belongs to archived account
]

describe('activeFireInputs', () => {
  it('excludes archived accounts', () => {
    const { accounts: a } = activeFireInputs(accounts, balances)
    expect(a.map(x => x.id)).toEqual([1])
  })

  it('excludes balances belonging to archived accounts', () => {
    const { balances: b } = activeFireInputs(accounts, balances)
    expect(b).toEqual([{ id: 1, accountId: 1, balance: 1000, recordedAt: '2026-01-01' }])
  })

  it('keeps all inputs when every account is active', () => {
    const allActive = accounts.map(a => ({ ...a, isActive: true }))
    const { accounts: a, balances: b } = activeFireInputs(allActive, balances)
    expect(a).toHaveLength(2)
    expect(b).toHaveLength(2)
  })
})
