import { describe, expect, it } from 'vitest'
import { accountsNeedingAttention } from './attention'
import type { Account } from '../types/Account'
import type { SimpleFinRemoteAccount } from '../types/SimpleFinRemoteAccount'
import type { SimpleFinStatus } from '../types/SimpleFinStatus'

function account(id: number, simplefinId: string | null): Account {
  return {
    id,
    name: `Account ${id}`,
    type: 'checking',
    institution: null,
    isActive: true,
    includeInFireCalculations: false,
    createdAt: '2026-01-01',
    simplefinId,
    countPaymentsAsExpense: false,
    traditionalPct: null,
  }
}

function remote(id: string, org: string | null, linkedAccountId: number | null): SimpleFinRemoteAccount {
  return {
    id,
    name: `Remote ${id}`,
    org,
    balance: 100,
    balanceDate: '2026-07-01',
    currency: 'USD',
    linkedAccountId,
  }
}

function status(overrides: Partial<SimpleFinStatus>): SimpleFinStatus {
  return {
    connected: true,
    claimedAt: '2026-01-01T00:00:00Z',
    lastAttemptAt: null,
    lastSuccessAt: null,
    lastError: null,
    bridgeErrors: [],
    accounts: [],
    ...overrides,
  }
}

describe('accountsNeedingAttention', () => {
  it('returns empty when status is null or disconnected', () => {
    const accounts = [account(1, 'sf-1')]
    expect(accountsNeedingAttention(accounts, null).size).toBe(0)
    expect(
      accountsNeedingAttention(accounts, status({ connected: false, lastError: 'boom' })).size,
    ).toBe(0)
  })

  it('flags linked accounts whose org appears in a bridge error, case-insensitively', () => {
    const accounts = [account(1, 'sf-1'), account(2, 'sf-2')]
    const s = status({
      accounts: [remote('sf-1', 'Chase', 1), remote('sf-2', 'Fidelity', 2)],
      bridgeErrors: ['Connection to CHASE may need attention'],
    })
    const result = accountsNeedingAttention(accounts, s)
    expect(result.get(1)).toBe('Connection to CHASE may need attention')
    expect(result.has(2)).toBe(false)
  })

  it('ignores unlinked accounts and errors naming no known org', () => {
    const accounts = [account(1, null), account(2, 'sf-2')]
    const s = status({
      accounts: [remote('sf-2', 'Fidelity', 2)],
      bridgeErrors: ['Connection to Some Other Bank may need attention'],
    })
    expect(accountsNeedingAttention(accounts, s).size).toBe(0)
  })

  it('skips remote accounts with no org rather than matching everything', () => {
    const accounts = [account(1, 'sf-1')]
    const s = status({
      accounts: [remote('sf-1', null, 1)],
      bridgeErrors: ['Connection to Chase may need attention'],
    })
    expect(accountsNeedingAttention(accounts, s).size).toBe(0)
  })

  it('flags every linked account when the whole connection errored', () => {
    const accounts = [account(1, 'sf-1'), account(2, 'sf-2'), account(3, null)]
    const s = status({
      accounts: [remote('sf-1', 'Chase', 1), remote('sf-2', 'Fidelity', 2)],
      lastError: 'Access token revoked',
    })
    const result = accountsNeedingAttention(accounts, s)
    expect(result.get(1)).toBe('Access token revoked')
    expect(result.get(2)).toBe('Access token revoked')
    expect(result.has(3)).toBe(false)
  })
})
