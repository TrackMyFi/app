import { describe, it, expect } from 'vitest'
import { buildBudgetMonth } from './index'
import type { Transaction, } from '../types/Transaction'
import type { PaycheckSummary } from './index'

function makeTxn(overrides: Partial<Transaction>): Transaction {
  return {
    id: 1,
    accountId: 1,
    transferAccountId: null,
    amount: 0,
    description: 'Test transaction',
    date: '2025-01-01',
    type: 'income',
    category: '',
    isContribution: false,
    isWithdrawal: false,
    importSource: 'manual',
    generatedBalanceId: null,
    generatedBalanceToId: null,
    paycheckId: null,
    createdAt: '2025-01-01',
    updatedAt: '2025-01-01',
    ...overrides,
  }
}

const zeroPaycheck: PaycheckSummary = { grossIncome: 0, netIncome: 0, taxes: 0 }

describe('buildBudgetMonth', () => {
  it('returns all zeros and empty arrays for an empty transaction list', () => {
    const result = buildBudgetMonth([], zeroPaycheck)
    expect(result.income.total).toBe(0)
    expect(result.income.transactions).toEqual([])
    expect(result.savings.total).toBe(0)
    expect(result.savings.transactions).toEqual([])
    expect(result.fixed.total).toBe(0)
    expect(result.fixed.transactions).toEqual([])
    expect(result.discretionary.total).toBe(0)
    expect(result.discretionary.transactions).toEqual([])
    expect(result.freeMoney).toBe(0)
    expect(result.freeMoneyRemaining).toBe(0)
  })

  it('places non-paycheck income txns into the income bucket', () => {
    const txns = [
      makeTxn({ id: 1, amount: 3000, type: 'income', isContribution: false, importSource: 'manual' }),
      makeTxn({ id: 2, amount: 2000, type: 'income', isContribution: false, importSource: 'manual' }),
    ]
    const result = buildBudgetMonth(txns, zeroPaycheck)
    expect(result.income.total).toBe(5000)
    expect(result.income.transactions).toHaveLength(2)
    expect(result.savings.total).toBe(0)
  })

  it('excludes paycheck income txns from the income bucket', () => {
    const txns = [
      makeTxn({ id: 1, amount: 3000, type: 'income', isContribution: false, importSource: 'paycheck' }),
      makeTxn({ id: 2, amount: 2000, type: 'income', isContribution: false, importSource: 'manual' }),
    ]
    const result = buildBudgetMonth(txns, zeroPaycheck)
    expect(result.income.total).toBe(2000)
    expect(result.income.transactions).toHaveLength(1)
  })

  it('places contribution txns (isContribution=true) into savings regardless of type', () => {
    const txns = [
      makeTxn({ id: 1, amount: 500, type: 'income', isContribution: true }),
      makeTxn({ id: 2, amount: 300, type: 'expense', isContribution: true }),
    ]
    const result = buildBudgetMonth(txns, zeroPaycheck)
    expect(result.savings.total).toBe(800)
    expect(result.savings.transactions).toHaveLength(2)
    expect(result.income.total).toBe(0)
    expect(result.fixed.total).toBe(0)
    expect(result.discretionary.total).toBe(0)
  })

  it('places expense/fixed txns into the fixed bucket', () => {
    const txns = [
      makeTxn({ id: 1, amount: 1200, type: 'expense', category: 'fixed', isContribution: false }),
      makeTxn({ id: 2, amount: 800, type: 'expense', category: 'fixed', isContribution: false }),
    ]
    const result = buildBudgetMonth(txns, zeroPaycheck)
    expect(result.fixed.total).toBe(2000)
    expect(result.fixed.transactions).toHaveLength(2)
    expect(result.income.total).toBe(0)
    expect(result.discretionary.total).toBe(0)
  })

  it('places expense/discretionary txns into the discretionary bucket', () => {
    const txns = [
      makeTxn({ id: 1, amount: 150, type: 'expense', category: 'discretionary', isContribution: false }),
      makeTxn({ id: 2, amount: 75, type: 'expense', category: 'discretionary', isContribution: false }),
    ]
    const result = buildBudgetMonth(txns, zeroPaycheck)
    expect(result.discretionary.total).toBe(225)
    expect(result.discretionary.transactions).toHaveLength(2)
    expect(result.fixed.total).toBe(0)
  })

  it('excludes transfer txns from all buckets', () => {
    const txns = [
      makeTxn({ id: 1, amount: 500, type: 'transfer', category: '' }),
      makeTxn({ id: 2, amount: 200, type: 'transfer', category: 'fixed' }),
    ]
    const result = buildBudgetMonth(txns, zeroPaycheck)
    expect(result.income.total).toBe(0)
    expect(result.savings.total).toBe(0)
    expect(result.fixed.total).toBe(0)
    expect(result.discretionary.total).toBe(0)
    expect(result.income.transactions).toHaveLength(0)
    expect(result.savings.transactions).toHaveLength(0)
    expect(result.fixed.transactions).toHaveLength(0)
    expect(result.discretionary.transactions).toHaveLength(0)
  })

  it('grossIncome = paycheckSummary.grossIncome + non-paycheck income total', () => {
    const txns = [
      makeTxn({ id: 1, amount: 1000, type: 'income', isContribution: false, importSource: 'manual' }),
    ]
    const result = buildBudgetMonth(txns, { grossIncome: 5000, netIncome: 3500, taxes: 1432.5 })
    expect(result.grossIncome).toBe(6000)
  })

  it('netIncome = paycheckSummary.netIncome + non-paycheck income total', () => {
    const txns = [
      makeTxn({ id: 1, amount: 1000, type: 'income', isContribution: false, importSource: 'manual' }),
    ]
    const result = buildBudgetMonth(txns, { grossIncome: 5000, netIncome: 3500, taxes: 1432.5 })
    expect(result.netIncome).toBe(4500)
  })

  it('taxes propagates from paycheckSummary', () => {
    const result = buildBudgetMonth([], { grossIncome: 5000, netIncome: 3500, taxes: 1432.5 })
    expect(result.taxes).toBe(1432.5)
  })

  it('computes freeMoney as grossIncome - savings - taxes - fixed', () => {
    const txns = [
      makeTxn({ id: 1, amount: 1000, type: 'income', isContribution: false, importSource: 'manual' }),
      makeTxn({ id: 2, amount: 500, type: 'income', isContribution: true }),  // savings
      makeTxn({ id: 3, amount: 1200, type: 'expense', category: 'fixed', isContribution: false }),
    ]
    const result = buildBudgetMonth(txns, { grossIncome: 5000, netIncome: 3500, taxes: 500 })
    // grossIncome = 5000 + 1000 = 6000
    // freeMoney = 6000 - 500 - 500 - 1200 = 3800
    expect(result.grossIncome).toBe(6000)
    expect(result.savings.total).toBe(500)
    expect(result.taxes).toBe(500)
    expect(result.fixed.total).toBe(1200)
    expect(result.freeMoney).toBe(3800)
  })

  it('computes freeMoneyRemaining as freeMoney - discretionary.total', () => {
    const txns = [
      makeTxn({ id: 1, amount: 1000, type: 'income', isContribution: false, importSource: 'manual' }),
      makeTxn({ id: 2, amount: 500, type: 'income', isContribution: true }),  // savings
      makeTxn({ id: 3, amount: 1200, type: 'expense', category: 'fixed', isContribution: false }),
      makeTxn({ id: 4, amount: 400, type: 'expense', category: 'discretionary', isContribution: false }),
      makeTxn({ id: 5, amount: 250, type: 'expense', category: 'discretionary', isContribution: false }),
    ]
    const result = buildBudgetMonth(txns, { grossIncome: 5000, netIncome: 3500, taxes: 500 })
    // grossIncome = 6000, freeMoney = 6000 - 500 - 500 - 1200 = 3800
    expect(result.freeMoney).toBe(3800)
    expect(result.freeMoneyRemaining).toBe(3800 - 650) // 3150
  })
})
