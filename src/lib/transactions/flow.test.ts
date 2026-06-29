import { describe, it, expect } from 'vitest'
import { classifyFlow, cashFlowTotals, savingsRate, transferDirection } from './flow'
import type { Transaction } from '../types/Transaction'
import type { Account } from '../types/Account'

const accounts: Account[] = [
  { id: 1, name: 'Checking', type: 'checking', institution: null, isActive: true, includeInFireCalculations: false, createdAt: '' },
  { id: 2, name: 'Brokerage', type: 'brokerage', institution: null, isActive: true, includeInFireCalculations: true, createdAt: '' },
  { id: 3, name: 'Credit Card', type: 'liability', institution: null, isActive: true, includeInFireCalculations: false, createdAt: '' },
]

function tx(overrides: Partial<Transaction>): Transaction {
  return {
    id: 1, accountId: 1, transferAccountId: null, amount: 100, description: '', date: '2026-05-01',
    type: 'expense', category: 'discretionary', isContribution: false, isWithdrawal: false, importSource: 'manual',
    generatedBalanceId: null, generatedBalanceToId: null, paycheckId: null, createdAt: '', updatedAt: '',
    ...overrides,
  }
}

describe('transferDirection', () => {
  it('is neutral for asset → asset', () => {
    expect(transferDirection('checking', 'brokerage')).toBe('neutral')
  })
  it('is outflow for asset → liability (paying down debt)', () => {
    expect(transferDirection('checking', 'liability')).toBe('outflow')
  })
  it('is inflow for liability → asset (refund/credit)', () => {
    expect(transferDirection('liability', 'checking')).toBe('inflow')
  })
})

describe('classifyFlow', () => {
  it('treats income as an inflow', () => {
    const f = classifyFlow(tx({ type: 'income', amount: 1000 }), accounts)
    expect(f).toMatchObject({ direction: 'inflow', inflow: 1000, outflow: 0, isSavings: false })
  })

  it('treats expense as an outflow bucketed by category', () => {
    const f = classifyFlow(tx({ type: 'expense', category: 'fixed', amount: 50 }), accounts)
    expect(f).toMatchObject({ direction: 'outflow', outflow: 50, bucket: 'fixed', isSavings: false })
  })

  it('treats a contribution transfer as a savings outflow', () => {
    const f = classifyFlow(
      tx({ type: 'transfer', accountId: 1, transferAccountId: 2, isContribution: true, amount: 500 }),
      accounts,
    )
    expect(f).toMatchObject({ outflow: 500, bucket: 'savings', isSavings: true })
  })

  it('treats a withdrawal as negative savings (dis-saving)', () => {
    const f = classifyFlow(
      tx({ type: 'transfer', accountId: 2, transferAccountId: 1, isContribution: true, isWithdrawal: true, amount: 400 }),
      accounts,
    )
    expect(f).toMatchObject({ outflow: -400, bucket: 'savings', isSavings: true })
  })

  it('treats an asset → asset transfer as neutral (no cash flow)', () => {
    const f = classifyFlow(tx({ type: 'transfer', accountId: 1, transferAccountId: 2, amount: 500 }), accounts)
    expect(f).toMatchObject({ direction: 'neutral', inflow: 0, outflow: 0, isSavings: false })
  })

  it('treats an asset → liability transfer as cash-flow neutral (CC payment must not double-count the purchase)', () => {
    const f = classifyFlow(tx({ type: 'transfer', accountId: 1, transferAccountId: 3, amount: 200 }), accounts)
    expect(f).toMatchObject({ direction: 'outflow', inflow: 0, outflow: 0, isSavings: false })
  })

  it('treats a liability → asset transfer as cash-flow neutral (loan disbursement is not income)', () => {
    const f = classifyFlow(tx({ type: 'transfer', accountId: 3, transferAccountId: 1, amount: 75 }), accounts)
    expect(f).toMatchObject({ direction: 'inflow', inflow: 0, outflow: 0, isSavings: false })
  })
})

describe('cashFlowTotals', () => {
  it('separates spending from savings and leaves net unaffected by contributions', () => {
    const txns = [
      tx({ type: 'income', amount: 1000 }),
      tx({ type: 'expense', category: 'fixed', amount: 200 }),
      tx({ type: 'expense', category: 'discretionary', amount: 100 }),
      tx({ type: 'transfer', accountId: 1, transferAccountId: 2, isContribution: true, amount: 500 }),
    ]
    expect(cashFlowTotals(txns, accounts)).toEqual({ income: 1000, expense: 300, savings: 500, net: 700 })
  })

  it('nets a withdrawal out of the savings total', () => {
    const txns = [
      tx({ type: 'income', amount: 1000 }),
      tx({ type: 'transfer', accountId: 1, transferAccountId: 2, isContribution: true, amount: 500 }),
      tx({ type: 'transfer', accountId: 2, transferAccountId: 1, isContribution: true, isWithdrawal: true, amount: 200 }),
    ]
    expect(cashFlowTotals(txns, accounts)).toEqual({ income: 1000, expense: 0, savings: 300, net: 1000 })
  })
})

describe('savingsRate', () => {
  it('is Savings ÷ Income', () => {
    expect(savingsRate({ income: 1000, expense: 300, savings: 500, net: 700 })).toBe(0.5)
  })
  it('is null when there is no income', () => {
    expect(savingsRate({ income: 0, expense: 0, savings: 0, net: 0 })).toBeNull()
  })
})
