import { describe, it, expect } from 'vitest'
import type { Account } from '../types/Account'
import { groupAccounts, netWorth } from './groups'

const account = (overrides: Partial<Account>): Account => ({
  id: 1,
  name: 'Account',
  type: 'checking',
  institution: null,
  isActive: true,
  includeInFireCalculations: false,
  createdAt: '2026-01-01',
  simplefinId: null,
  countPaymentsAsExpense: false,
  traditionalPct: null,
  ...overrides,
})

describe('groupAccounts', () => {
  it('buckets non-FIRE accounts into Budget and FIRE accounts into Tracking', () => {
    const checking = account({ id: 1, type: 'checking', includeInFireCalculations: false })
    const brokerage = account({ id: 2, type: 'brokerage', includeInFireCalculations: true })
    const balances: Record<number, number> = { 1: 500, 2: 10000 }

    const groups = groupAccounts([checking, brokerage], id => balances[id])

    expect(groups.map(g => g.key)).toEqual(['budget', 'tracking'])
    expect(groups[0].accounts).toEqual([checking])
    expect(groups[1].accounts).toEqual([brokerage])
  })

  it('buckets real estate and mortgages into Equity regardless of FIRE flag', () => {
    const house = account({ id: 1, type: 'real_estate', includeInFireCalculations: true })
    const mortgage = account({ id: 2, type: 'mortgage', includeInFireCalculations: false })
    const balances: Record<number, number> = { 1: 300000, 2: 200000 }

    const groups = groupAccounts([house, mortgage], id => balances[id])

    expect(groups).toHaveLength(1)
    expect(groups[0].key).toBe('equity')
    expect(groups[0].accounts.map(a => a.id)).toEqual([1, 2])
  })

  it('sorts accounts within a group by balance descending', () => {
    const small = account({ id: 1, type: 'checking' })
    const large = account({ id: 2, type: 'savings' })
    const balances: Record<number, number> = { 1: 100, 2: 5000 }

    const groups = groupAccounts([small, large], id => balances[id])

    expect(groups[0].accounts.map(a => a.id)).toEqual([2, 1])
  })

  it('subtracts liability balances from the group total', () => {
    const checking = account({ id: 1, type: 'checking' })
    const creditCard = account({ id: 2, type: 'liability' })
    const balances: Record<number, number> = { 1: 1000, 2: 300 }

    const groups = groupAccounts([checking, creditCard], id => balances[id])

    expect(groups[0].total).toBe(700)
  })

  it('omits groups with no accounts', () => {
    const checking = account({ id: 1, type: 'checking' })
    const groups = groupAccounts([checking], () => 100)
    expect(groups.map(g => g.key)).toEqual(['budget'])
  })

  it('excludes archived accounts', () => {
    const active = account({ id: 1, isActive: true })
    const archived = account({ id: 2, isActive: false })
    const groups = groupAccounts([active, archived], () => 100)
    expect(groups[0].accounts).toEqual([active])
  })
})

describe('netWorth', () => {
  it('sums signed balances across all active accounts', () => {
    const checking = account({ id: 1, type: 'checking' })
    const creditCard = account({ id: 2, type: 'liability' })
    const archived = account({ id: 3, isActive: false })
    const balances: Record<number, number> = { 1: 1000, 2: 300, 3: 99999 }

    expect(netWorth([checking, creditCard, archived], id => balances[id])).toBe(700)
  })
})
