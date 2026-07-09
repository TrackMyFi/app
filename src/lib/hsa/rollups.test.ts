import { describe, expect, it } from 'vitest'
import { reimbursedTotal, totalExpenses, unreimbursedTotal } from './rollups'
import type { HsaExpense } from '../types/HsaExpense'

function expense(overrides: Partial<HsaExpense> = {}): HsaExpense {
  return {
    id: 1,
    accountId: null,
    date: '2026-01-15',
    description: 'Annual physical',
    category: 'medical',
    amount: 100,
    person: null,
    provider: null,
    notes: null,
    reimbursed: false,
    reimbursedDate: null,
    createdAt: '2026-01-15T00:00:00.000Z',
    updatedAt: '2026-01-15T00:00:00.000Z',
    hasAttachment: false,
    ...overrides,
  }
}

describe('hsa rollups', () => {
  const expenses = [
    expense({ id: 1, amount: 120.5, reimbursed: false }),
    expense({ id: 2, amount: 80, reimbursed: true, reimbursedDate: '2026-02-01' }),
    expense({ id: 3, amount: 45.25, reimbursed: false }),
  ]

  it('sums all expenses', () => {
    expect(totalExpenses(expenses)).toBeCloseTo(245.75)
  })

  it('sums only unreimbursed expenses', () => {
    expect(unreimbursedTotal(expenses)).toBeCloseTo(165.75)
  })

  it('sums only reimbursed expenses', () => {
    expect(reimbursedTotal(expenses)).toBeCloseTo(80)
  })

  it('returns 0 for empty lists', () => {
    expect(totalExpenses([])).toBe(0)
    expect(unreimbursedTotal([])).toBe(0)
    expect(reimbursedTotal([])).toBe(0)
  })
})
