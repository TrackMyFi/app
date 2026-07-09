import { describe, it, expect } from 'vitest'
import { detectRecurring } from './recurring'
import type { Transaction } from '../types/Transaction'
import type { Account } from '../types/Account'

const accounts: Account[] = [
  { id: 1, name: 'Checking', type: 'checking', institution: null, isActive: true, includeInFireCalculations: false, createdAt: '', simplefinId: null, countPaymentsAsExpense: false, traditionalPct: null },
]

let nextId = 1
function tx(overrides: Partial<Transaction>): Transaction {
  return {
    id: nextId++, accountId: 1, transferAccountId: null, amount: 10, description: '', date: '2026-05-01',
    type: 'expense', category: 'discretionary', isContribution: false, isWithdrawal: false, isRefund: false, importSource: 'manual',
    generatedBalanceId: null, generatedBalanceToId: null, paycheckId: null, vendorCategory: null, simplefinId: null, suppressedAs: null, rawDescription: null, createdAt: '', updatedAt: '',
    ...overrides,
  }
}

describe('detectRecurring', () => {
  it('flags a charge that appears most months at a consistent amount', () => {
    const charges = detectRecurring([
      tx({ description: 'NETFLIX', amount: 15.49, date: '2026-04-05' }),
      tx({ description: 'NETFLIX', amount: 15.49, date: '2026-05-05' }),
      tx({ description: 'NETFLIX', amount: 15.49, date: '2026-06-05' }),
      tx({ description: 'NETFLIX', amount: 16.49, date: '2026-07-05' }),
    ], accounts, { asOf: '2026-07-15' })

    expect(charges).toHaveLength(1)
    expect(charges[0].displayName).toBe('Netflix')
    expect(charges[0].monthsSeen).toBe(4)
    expect(charges[0].monthlyAmount).toBeCloseTo((15.49 * 3 + 16.49) / 4, 5)
    expect(charges[0].annualized).toBeCloseTo(charges[0].monthlyAmount * 12, 5)
  })

  it('does not flag a charge seen in fewer than the minimum months', () => {
    const charges = detectRecurring([
      tx({ description: 'ONE OFF STORE', amount: 40, date: '2026-06-01' }),
      tx({ description: 'ONE OFF STORE', amount: 40, date: '2026-07-01' }),
    ], accounts, { asOf: '2026-07-15' })
    expect(charges).toHaveLength(0)
  })

  it('does not flag a merchant whose amount varies too much month to month', () => {
    const charges = detectRecurring([
      tx({ description: 'GROCERY MART', amount: 40, date: '2026-04-01' }),
      tx({ description: 'GROCERY MART', amount: 180, date: '2026-05-01' }),
      tx({ description: 'GROCERY MART', amount: 65, date: '2026-06-01' }),
      tx({ description: 'GROCERY MART', amount: 210, date: '2026-07-01' }),
    ], accounts, { asOf: '2026-07-15' })
    expect(charges).toHaveLength(0)
  })

  it('excludes fixed-bucket bills even if they recur consistently — not an actionable "cancel it" opportunity', () => {
    const charges = detectRecurring([
      tx({ description: 'MORTGAGE CO', amount: 1800, category: 'fixed', date: '2026-04-01' }),
      tx({ description: 'MORTGAGE CO', amount: 1800, category: 'fixed', date: '2026-05-01' }),
      tx({ description: 'MORTGAGE CO', amount: 1800, category: 'fixed', date: '2026-06-01' }),
      tx({ description: 'MORTGAGE CO', amount: 1800, category: 'fixed', date: '2026-07-01' }),
    ], accounts, { asOf: '2026-07-15' })
    expect(charges).toHaveLength(0)
  })

  it('excludes uncategorized transactions — often a mislabeled recurring deposit (income), not a chargeable subscription', () => {
    const charges = detectRecurring([
      tx({ description: 'STRIPE TRANSFER CORPORATE ACH ST', amount: 2578.54, category: 'uncategorized', date: '2026-04-16' }),
      tx({ description: 'STRIPE TRANSFER CORPORATE ACH ST', amount: 2578.54, category: 'uncategorized', date: '2026-05-16' }),
      tx({ description: 'STRIPE TRANSFER CORPORATE ACH ST', amount: 2578.54, category: 'uncategorized', date: '2026-06-16' }),
      tx({ description: 'STRIPE TRANSFER CORPORATE ACH ST', amount: 2578.54, category: 'uncategorized', date: '2026-07-16' }),
    ], accounts, { asOf: '2026-07-15' })
    expect(charges).toHaveLength(0)
  })

  it('excludes savings/contributions and ignores transactions outside the trailing window', () => {
    const charges = detectRecurring([
      tx({ description: 'BROKERAGE', amount: 500, type: 'transfer', isContribution: true, date: '2026-07-01' }),
      tx({ description: 'OLD SUBSCRIPTION', amount: 10, date: '2025-01-01' }),
      tx({ description: 'OLD SUBSCRIPTION', amount: 10, date: '2025-02-01' }),
      tx({ description: 'OLD SUBSCRIPTION', amount: 10, date: '2025-03-01' }),
    ], accounts, { asOf: '2026-07-15' })
    expect(charges).toHaveLength(0)
  })

  it('ignores generic descriptions even if they repeat', () => {
    const charges = detectRecurring([
      tx({ description: 'DEBIT CARD PURCHASE', amount: 10, date: '2026-04-01' }),
      tx({ description: 'DEBIT CARD PURCHASE', amount: 10, date: '2026-05-01' }),
      tx({ description: 'DEBIT CARD PURCHASE', amount: 10, date: '2026-06-01' }),
    ], accounts, { asOf: '2026-07-15' })
    expect(charges).toHaveLength(0)
  })

  it('sorts by annualized cost descending', () => {
    const months = ['2026-04-01', '2026-05-01', '2026-06-01', '2026-07-01']
    const charges = detectRecurring([
      ...months.map((date) => tx({ description: 'CHEAP APP', amount: 3, date })),
      ...months.map((date) => tx({ description: 'GYM MEMBERSHIP', amount: 50, date })),
    ], accounts, { asOf: '2026-07-15' })
    expect(charges.map((c) => c.displayName)).toEqual(['Gym Membership', 'Cheap App'])
  })
})
