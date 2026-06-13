import { describe, it, expect } from 'vitest'
import {
  trailingMonthlyContribution,
  hasFullYearOfContributions,
  derivedMonthlyContribution,
} from './contributionRate'
import type { Transaction } from '../types/Transaction'

// Minimal Transaction factory — only fields the functions read matter.
function txn(p: Partial<Transaction>): Transaction {
  return {
    id: 1, accountId: 1, transferAccountId: null, amount: 0, description: '',
    date: '2026-01-01', type: 'expense', category: '', isContribution: false,
    importSource: 'manual', generatedBalanceId: null, generatedBalanceToId: null,
    paycheckId: null, createdAt: '', updatedAt: '', ...p,
  }
}

const asOf = '2026-06-30'

describe('trailingMonthlyContribution', () => {
  it('sums contributions in the trailing 12 months and divides by 12', () => {
    const txns = [
      txn({ amount: 600, date: '2026-06-01', isContribution: true }),
      txn({ amount: 600, date: '2026-01-15', isContribution: true }),
      txn({ amount: 999, date: '2026-03-01', isContribution: false }), // not a contribution
    ]
    expect(trailingMonthlyContribution(txns, asOf)).toBe(100) // 1200 / 12
  })

  it('excludes contributions older than 12 months and any in the future', () => {
    const txns = [
      txn({ amount: 1200, date: '2025-06-01', isContribution: true }), // exactly 12mo+1day before → out
      txn({ amount: 1200, date: '2026-12-01', isContribution: true }), // future → out
    ]
    expect(trailingMonthlyContribution(txns, asOf)).toBe(0)
  })
})

describe('hasFullYearOfContributions', () => {
  it('is true when a contribution exists at or before the 12-month cutoff', () => {
    const txns = [txn({ amount: 100, date: '2025-06-30', isContribution: true })]
    expect(hasFullYearOfContributions(txns, asOf)).toBe(true)
  })

  it('is false when all contributions are newer than the cutoff', () => {
    const txns = [txn({ amount: 100, date: '2026-02-01', isContribution: true })]
    expect(hasFullYearOfContributions(txns, asOf)).toBe(false)
  })

  it('is false with no contributions', () => {
    expect(hasFullYearOfContributions([], asOf)).toBe(false)
  })
})

describe('derivedMonthlyContribution', () => {
  it('uses actual trailing average when a full year of history exists', () => {
    const txns = [
      // At/before the 12-mo cutoff: establishes a full year of history, not summed itself.
      txn({ amount: 9999, date: '2025-06-30', isContribution: true }),
      // Inside the trailing window: these are summed.
      txn({ amount: 1200, date: '2025-08-01', isContribution: true }),
      txn({ amount: 1200, date: '2026-05-01', isContribution: true }),
    ]
    const r = derivedMonthlyContribution(txns, asOf, 500)
    expect(r.estimated).toBe(false)
    expect(r.monthly).toBe(200) // (1200 + 1200) / 12
  })

  it('falls back to the supplied estimate when <12 months of history', () => {
    const txns = [txn({ amount: 1200, date: '2026-05-01', isContribution: true })]
    const r = derivedMonthlyContribution(txns, asOf, 500)
    expect(r.estimated).toBe(true)
    expect(r.monthly).toBe(500)
  })
})
