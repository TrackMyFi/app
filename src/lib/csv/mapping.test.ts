import { describe, it, expect } from 'vitest'
import { applyMapping, autoDetectMapping, detectDuplicates, parseAmount, type MappingConfig } from './mapping'

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

describe('parseAmount', () => {
  it('handles standard negative amounts', () => {
    expect(parseAmount('-42.50')).toBe(-42.5)
  })

  it('handles parentheses-notation negatives from bank exports', () => {
    expect(parseAmount('(42.50)')).toBe(-42.5)
    expect(parseAmount('($42.50)')).toBe(-42.5)
  })

  it('strips currency symbols and commas', () => {
    expect(parseAmount('$1,234.56')).toBe(1234.56)
  })

  it('returns 0 for non-numeric values like N/A', () => {
    expect(parseAmount('N/A')).toBe(0)
    expect(parseAmount('')).toBe(0)
  })
})

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

describe('autoDetectMapping', () => {
  it('detects Date/Credit/Debit headers as split mode', () => {
    const result = autoDetectMapping(
      ['Date', 'Description', 'Credit', 'Debit'],
      [{ Date: '03/01/2026', Description: 'Coffee', Credit: '0', Debit: '42.50' }],
    )
    expect(result.dateColumn).toBe('Date')
    expect(result.descriptionColumn).toBe('Description')
    expect(result.amountMode).toBe('split')
    expect(result.creditColumn).toBe('Credit')
    expect(result.debitColumn).toBe('Debit')
  })

  it('detects single Amount column when no credit/debit present', () => {
    const result = autoDetectMapping(
      ['Posting Date', 'Memo', 'Amount'],
      [{ 'Posting Date': '03/01/2026', Memo: 'Coffee', Amount: '-42.50' }],
    )
    expect(result.dateColumn).toBe('Posting Date')
    expect(result.descriptionColumn).toBe('Memo')
    expect(result.amountMode).toBe('single')
    expect(result.amountColumn).toBe('Amount')
  })

  it('auto-detects MM/dd/yyyy date format', () => {
    const result = autoDetectMapping(
      ['Date', 'Amount'],
      [{ Date: '03/15/2026', Amount: '-42.50' }],
    )
    expect(result.dateFormat).toBe('MM/dd/yyyy')
  })

  it('auto-detects yyyy-MM-dd date format', () => {
    const result = autoDetectMapping(
      ['Date', 'Amount'],
      [{ Date: '2026-03-15', Amount: '-42.50' }],
    )
    expect(result.dateFormat).toBe('yyyy-MM-dd')
  })

  it('returns empty object when no headers match', () => {
    const result = autoDetectMapping(['Foo', 'Bar', 'Baz'], [])
    expect(result).toEqual({})
  })

  it('matching is case-insensitive', () => {
    const result = autoDetectMapping(
      ['TRANSACTION DATE', 'DESCRIPTION', 'AMOUNT'],
      [{ 'TRANSACTION DATE': '03/01/2026', DESCRIPTION: 'Coffee', AMOUNT: '-42.50' }],
    )
    expect(result.dateColumn).toBe('TRANSACTION DATE')
    expect(result.descriptionColumn).toBe('DESCRIPTION')
    expect(result.amountColumn).toBe('AMOUNT')
  })
})
