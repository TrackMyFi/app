import { describe, it, expect } from 'vitest'
import { suggestVendorRules } from './vendorSuggestions'
import type { Transaction } from '../types/Transaction'
import type { Account } from '../types/Account'

const accounts: Account[] = [
  { id: 1, name: 'Checking', type: 'checking', institution: null, isActive: true, includeInFireCalculations: false, createdAt: '', simplefinId: null, countPaymentsAsExpense: false },
]

function tx(overrides: Partial<Transaction>): Transaction {
  return {
    id: 1, accountId: 1, transferAccountId: null, amount: 10, description: '', date: '2026-05-01',
    type: 'expense', category: 'discretionary', isContribution: false, isWithdrawal: false, importSource: 'manual',
    generatedBalanceId: null, generatedBalanceToId: null, paycheckId: null, vendorCategory: null, simplefinId: null, suppressedAs: null, createdAt: '', updatedAt: '',
    ...overrides,
  }
}

describe('suggestVendorRules', () => {
  it('suggests a rule for descriptions that splinter across order-reference suffixes', () => {
    const suggestions = suggestVendorRules([
      tx({ description: 'AMAZON MKTPL*0U36P84J3 SEATTLE WA', amount: 20 }),
      tx({ description: 'AMAZON MKTPL*T45SK5WG3 SEATTLE WA', amount: 15 }),
      tx({ description: 'AMAZON MKTPL*RS9YF06N3 SEATTLE WA', amount: 30 }),
    ], accounts, [])

    expect(suggestions).toHaveLength(1)
    expect(suggestions[0]).toMatchObject({ keyword: 'amazon', vendorName: 'Amazon', count: 3, total: 65 })
  })

  it('trims a noisy store number in the middle of the suggested keyword', () => {
    const suggestions = suggestVendorRules([
      tx({ description: 'PIZZA HUT 029908 HEBRON KY NULL', amount: 20 }),
      tx({ description: 'PIZZA HUT 029908 HEBRON KY', amount: 15 }),
    ], accounts, [])

    expect(suggestions[0]).toMatchObject({ keyword: 'pizza hut', vendorName: 'Pizza Hut' })
  })

  it('does not suggest a rule for a merchant that already forms a single group', () => {
    const suggestions = suggestVendorRules([
      tx({ description: 'NETFLIX.COM', amount: 15 }),
      tx({ description: 'NETFLIX.COM', amount: 15 }),
    ], accounts, [])

    expect(suggestions).toHaveLength(0)
  })

  it('excludes transactions already covered by an existing rule', () => {
    const suggestions = suggestVendorRules([
      tx({ description: 'AMAZON MKTPL*0U36P84J3 SEATTLE WA', amount: 20 }),
      tx({ description: 'AMAZON MKTPL*T45SK5WG3 SEATTLE WA', amount: 15 }),
    ], accounts, [{ keyword: 'amazon', vendorName: 'Amazon' }])

    expect(suggestions).toHaveLength(0)
  })

  it('ignores savings/transfers and rank suggestions by total spend', () => {
    const suggestions = suggestVendorRules([
      tx({ description: 'AMAZON MKTPL*0U36P84J3 SEATTLE WA', amount: 20 }),
      tx({ description: 'AMAZON MKTPL*T45SK5WG3 SEATTLE WA', amount: 15 }),
      tx({ description: 'BROKERAGE CONTRIBUTION', amount: 500, type: 'transfer', isContribution: true }),
      tx({ description: 'KROGER #434 BURLINGTON KY', amount: 200 }),
      tx({ description: 'KROGER #921 BURLINGTON KY', amount: 100 }),
    ], accounts, [])

    expect(suggestions).toHaveLength(2)
    expect(suggestions[0].vendorName).toBe('Kroger')
    expect(suggestions[1].vendorName).toBe('Amazon')
  })
})
