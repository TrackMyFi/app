import { describe, it, expect } from 'vitest'
import { applyMapping, detectDuplicates, type MappingConfig } from './mapping'

const config: MappingConfig = {
  dateColumn: 'Posting Date',
  descriptionColumn: 'Description',
  dateFormat: 'MM/dd/yyyy',
  amountMode: 'single',
  amountColumn: 'Amount',
  amountSign: 'negative-is-expense',
  creditColumn: '',
  debitColumn: '',
  invertSplit: false,
  defaultCategory: 'uncategorized',
}

const rows = [
  { 'Posting Date': '03/01/2026', Amount: '-40.00', Description: 'Coffee' },
  { 'Posting Date': '03/02/2026', Amount: '1500.00', Description: 'Paycheck' },
]

describe('applyMapping', () => {
  it('maps rows to parsed transactions with inferred type and ISO date', () => {
    expect(applyMapping(rows, config)).toEqual([
      { date: '2026-03-01', amount: 40, description: 'Coffee', type: 'expense', category: 'uncategorized' },
      { date: '2026-03-02', amount: 1500, description: 'Paycheck', type: 'income', category: 'uncategorized' },
    ])
  })

  it('flips inference when amountSign is positive-is-expense', () => {
    const flipped = applyMapping(
      [{ 'Posting Date': '03/01/2026', Amount: '40.00', Description: 'Coffee' }],
      { ...config, amountSign: 'positive-is-expense' },
    )
    expect(flipped[0]).toMatchObject({ type: 'expense', amount: 40 })
  })
})

describe('detectDuplicates', () => {
  it('flags parsed rows matching an existing transaction on account+date+amount+description', () => {
    const parsed = applyMapping(rows, config)
    const existing = [{ accountId: 7, date: '2026-03-01', amount: 40, description: 'Coffee' }]
    const flags = detectDuplicates(parsed, existing, 7)
    expect(flags).toEqual([true, false])
  })

  it('does not flag when the account differs', () => {
    const parsed = applyMapping(rows, config)
    const existing = [{ accountId: 99, date: '2026-03-01', amount: 40, description: 'Coffee' }]
    expect(detectDuplicates(parsed, existing, 7)).toEqual([false, false])
  })
})

describe('applyMapping split mode', () => {
  const splitConfig: MappingConfig = {
    dateColumn: 'Date',
    descriptionColumn: 'Description',
    dateFormat: 'MM/dd/yyyy',
    amountMode: 'split',
    amountColumn: '',
    amountSign: 'negative-is-expense',
    creditColumn: 'Credit',
    debitColumn: 'Debit',
    invertSplit: false,
    defaultCategory: 'uncategorized',
  }

  const rows = [
    { Date: '03/01/2026', Credit: '0', Debit: '42.50', Description: 'Coffee' },
    { Date: '03/02/2026', Credit: '1500.00', Debit: '0', Description: 'Paycheck' },
  ]

  it('maps debit to expense and credit to income for a non-liability account', () => {
    expect(applyMapping(rows, splitConfig, false)).toEqual([
      { date: '2026-03-01', amount: 42.5, description: 'Coffee', type: 'expense', category: 'uncategorized' },
      { date: '2026-03-02', amount: 1500, description: 'Paycheck', type: 'income', category: 'uncategorized' },
    ])
  })

  it('flips direction for a liability account (credit = expense, debit = income)', () => {
    expect(applyMapping(rows, splitConfig, true)).toEqual([
      { date: '2026-03-01', amount: 42.5, description: 'Coffee', type: 'income', category: 'uncategorized' },
      { date: '2026-03-02', amount: 1500, description: 'Paycheck', type: 'expense', category: 'uncategorized' },
    ])
  })

  it('inverts direction when invertSplit is true (non-liability: credit becomes expense)', () => {
    expect(applyMapping(rows, { ...splitConfig, invertSplit: true }, false)).toEqual([
      { date: '2026-03-01', amount: 42.5, description: 'Coffee', type: 'income', category: 'uncategorized' },
      { date: '2026-03-02', amount: 1500, description: 'Paycheck', type: 'expense', category: 'uncategorized' },
    ])
  })

  it('uses the larger column when both credit and debit are non-zero', () => {
    const r = [{ Date: '03/01/2026', Credit: '5.00', Debit: '42.50', Description: 'Mixed' }]
    expect(applyMapping(r, splitConfig, false)[0]).toMatchObject({ amount: 42.5, type: 'expense' })
  })

  it('falls back to amount 0 type expense when both columns are zero or blank', () => {
    const zeroRow = [{ Date: '03/01/2026', Credit: '0', Debit: '0', Description: 'Zero' }]
    const blankRow = [{ Date: '03/01/2026', Credit: '', Debit: '', Description: 'Blank' }]
    expect(applyMapping(zeroRow, splitConfig, false)[0]).toMatchObject({ amount: 0, type: 'expense' })
    expect(applyMapping(blankRow, splitConfig, false)[0]).toMatchObject({ amount: 0, type: 'expense' })
  })

  it('credit wins when credit and debit amounts are equal', () => {
    const r = [{ Date: '03/01/2026', Credit: '42.50', Debit: '42.50', Description: 'Tie' }]
    // credit wins (>=), non-liability → credit = income
    expect(applyMapping(r, splitConfig, false)[0]).toMatchObject({ amount: 42.5, type: 'income' })
  })
})
