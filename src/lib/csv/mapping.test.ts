import { describe, it, expect } from 'vitest'
import { applyMapping, autoDetectMapping, detectDuplicates, detectTransferCounterparts, parseAmount, type ExistingRef, type MappingConfig } from './mapping'

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
  transferRules: [],
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
      { date: '2026-03-01', amount: 40, description: 'Coffee', type: 'expense', category: 'uncategorized', transferAccountId: null },
      { date: '2026-03-02', amount: 1500, description: 'Paycheck', type: 'income', category: 'uncategorized', transferAccountId: null },
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

describe('detectTransferCounterparts', () => {
  // Importing the Citi (id 5) statement. An existing transfer lives on the PNC
  // account (id 7) pointing at Citi — the same real-world card payment.
  const existingTransfer: ExistingRef = {
    accountId: 7,
    date: '2026-05-21',
    amount: 236.83,
    description: 'CITI CARD ONLINE PAYMENT ACH WEB',
    type: 'transfer',
    transferAccountId: 5,
  }

  const citiRow = [
    { 'Posting Date': '05/20/2026', Amount: '236.83', Description: 'ONLINE PAYMENT, THANK YOU' },
  ]

  it('flags the counterpart despite a different description and a 1-day date gap', () => {
    const parsed = applyMapping(citiRow, { ...config, amountSign: 'positive-is-expense' })
    expect(detectTransferCounterparts(parsed, [existingTransfer], 5)).toEqual([true])
  })

  it('does not flag when the date is outside the ±3-day window', () => {
    const parsed = applyMapping(
      [{ 'Posting Date': '05/26/2026', Amount: '236.83', Description: 'ONLINE PAYMENT, THANK YOU' }],
      config,
    )
    expect(detectTransferCounterparts(parsed, [existingTransfer], 5)).toEqual([false])
  })

  it('does not flag when amounts differ', () => {
    const parsed = applyMapping(
      [{ 'Posting Date': '05/20/2026', Amount: '100.00', Description: 'ONLINE PAYMENT, THANK YOU' }],
      config,
    )
    expect(detectTransferCounterparts(parsed, [existingTransfer], 5)).toEqual([false])
  })

  it("does not flag when the existing transfer's other side is a different account", () => {
    const parsed = applyMapping(citiRow, config)
    // Importing account 9, but the transfer's other side is account 5.
    expect(detectTransferCounterparts(parsed, [existingTransfer], 9)).toEqual([false])
  })

  it('does not flag a plain (non-transfer) existing row of the same amount', () => {
    const parsed = applyMapping(citiRow, config)
    const existing: ExistingRef = { ...existingTransfer, type: 'expense', transferAccountId: null }
    expect(detectTransferCounterparts(parsed, [existing], 5)).toEqual([false])
  })

  it('consumes each existing transfer once', () => {
    const parsed = applyMapping(
      [
        { 'Posting Date': '05/20/2026', Amount: '236.83', Description: 'ONLINE PAYMENT, THANK YOU' },
        { 'Posting Date': '05/20/2026', Amount: '236.83', Description: 'ANOTHER PAYMENT' },
      ],
      config,
    )
    // Only one existing transfer, so only one of the two rows is flagged.
    expect(detectTransferCounterparts(parsed, [existingTransfer], 5)).toEqual([true, false])
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
    transferRules: [],
  }

  const rows = [
    { Date: '03/01/2026', Credit: '0', Debit: '42.50', Description: 'Coffee' },
    { Date: '03/02/2026', Credit: '1500.00', Debit: '0', Description: 'Paycheck' },
  ]

  it('maps debit to expense and credit to income', () => {
    expect(applyMapping(rows, splitConfig)).toEqual([
      { date: '2026-03-01', amount: 42.5, description: 'Coffee', type: 'expense', category: 'uncategorized', transferAccountId: null },
      { date: '2026-03-02', amount: 1500, description: 'Paycheck', type: 'income', category: 'uncategorized', transferAccountId: null },
    ])
  })

  it('infers types independent of account type (purchase stays an expense)', () => {
    // Account type never flips the inferred type — a card purchase is an expense.
    // The liability sign is applied later when a balance delta is computed.
    expect(applyMapping(rows, splitConfig)).toEqual([
      { date: '2026-03-01', amount: 42.5, description: 'Coffee', type: 'expense', category: 'uncategorized', transferAccountId: null },
      { date: '2026-03-02', amount: 1500, description: 'Paycheck', type: 'income', category: 'uncategorized', transferAccountId: null },
    ])
  })

  it('inverts direction when invertSplit is true (credit becomes expense)', () => {
    expect(applyMapping(rows, { ...splitConfig, invertSplit: true })).toEqual([
      { date: '2026-03-01', amount: 42.5, description: 'Coffee', type: 'income', category: 'uncategorized', transferAccountId: null },
      { date: '2026-03-02', amount: 1500, description: 'Paycheck', type: 'expense', category: 'uncategorized', transferAccountId: null },
    ])
  })

  it('uses the larger column when both credit and debit are non-zero', () => {
    const r = [{ Date: '03/01/2026', Credit: '5.00', Debit: '42.50', Description: 'Mixed' }]
    expect(applyMapping(r, splitConfig)[0]).toMatchObject({ amount: 42.5, type: 'expense' })
  })

  it('falls back to amount 0 type expense when both columns are zero or blank', () => {
    const zeroRow = [{ Date: '03/01/2026', Credit: '0', Debit: '0', Description: 'Zero' }]
    const blankRow = [{ Date: '03/01/2026', Credit: '', Debit: '', Description: 'Blank' }]
    expect(applyMapping(zeroRow, splitConfig)[0]).toMatchObject({ amount: 0, type: 'expense' })
    expect(applyMapping(blankRow, splitConfig)[0]).toMatchObject({ amount: 0, type: 'expense' })
  })

  it('credit wins when credit and debit amounts are equal', () => {
    const r = [{ Date: '03/01/2026', Credit: '42.50', Debit: '42.50', Description: 'Tie' }]
    // credit wins (>=) → credit = income
    expect(applyMapping(r, splitConfig)[0]).toMatchObject({ amount: 42.5, type: 'income' })
  })
})

describe('applyMapping with category rules', () => {
  it('applies a matching rule to override the default category', () => {
    const result = applyMapping(
      [{ 'Posting Date': '03/01/2026', Amount: '-40.00', Description: 'Netflix monthly' }],
      config,
      [{ keyword: 'netflix', category: 'fixed' }],
    )
    expect(result[0].category).toBe('fixed')
  })

  it('uses defaultCategory when no rule matches', () => {
    const result = applyMapping(
      [{ 'Posting Date': '03/01/2026', Amount: '-40.00', Description: 'Coffee' }],
      { ...config, defaultCategory: 'discretionary' },
      [{ keyword: 'netflix', category: 'fixed' }],
    )
    expect(result[0].category).toBe('discretionary')
  })

  it('rule matching is case-insensitive', () => {
    const result = applyMapping(
      [{ 'Posting Date': '03/01/2026', Amount: '-40.00', Description: 'NETFLIX' }],
      config,
      [{ keyword: 'netflix', category: 'fixed' }],
    )
    expect(result[0].category).toBe('fixed')
  })

  it('first matching rule wins', () => {
    const result = applyMapping(
      [{ 'Posting Date': '03/01/2026', Amount: '-40.00', Description: 'Netflix and Amazon' }],
      config,
      [
        { keyword: 'netflix', category: 'fixed' },
        { keyword: 'amazon', category: 'discretionary' },
      ],
    )
    expect(result[0].category).toBe('fixed')
  })

  it('omitting rules falls back to defaultCategory (backwards compatible)', () => {
    const result = applyMapping(
      [{ 'Posting Date': '03/01/2026', Amount: '-40.00', Description: 'Coffee' }],
      { ...config, defaultCategory: 'savings' },
    )
    expect(result[0].category).toBe('savings')
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

describe('applyMapping with transfer rules', () => {
  const transferConfig: MappingConfig = {
    ...config,
    transferRules: [{ keyword: 'payment thank you', transferAccountId: 42 }],
  }

  it('marks a matching row as transfer and sets transferAccountId', () => {
    const result = applyMapping(
      [{ 'Posting Date': '03/05/2026', Amount: '1200.00', Description: 'PAYMENT THANK YOU' }],
      transferConfig,
    )
    expect(result[0].type).toBe('transfer')
    expect(result[0].transferAccountId).toBe(42)
  })

  it('forces category to uncategorized for transfer rows', () => {
    const result = applyMapping(
      [{ 'Posting Date': '03/05/2026', Amount: '1200.00', Description: 'PAYMENT THANK YOU' }],
      { ...transferConfig, defaultCategory: 'discretionary' },
    )
    expect(result[0].category).toBe('uncategorized')
  })

  it('transfer rule matching is case-insensitive', () => {
    const result = applyMapping(
      [{ 'Posting Date': '03/05/2026', Amount: '1200.00', Description: 'payment thank you' }],
      transferConfig,
    )
    expect(result[0].type).toBe('transfer')
    expect(result[0].transferAccountId).toBe(42)
  })

  it('transfer rule takes priority over category rule', () => {
    const result = applyMapping(
      [{ 'Posting Date': '03/05/2026', Amount: '1200.00', Description: 'PAYMENT THANK YOU' }],
      transferConfig,
      [{ keyword: 'payment', category: 'fixed' }],
    )
    expect(result[0].type).toBe('transfer')
    expect(result[0].category).toBe('uncategorized')
  })

  it('non-matching rows have transferAccountId null', () => {
    const result = applyMapping(
      [{ 'Posting Date': '03/01/2026', Amount: '-40.00', Description: 'Coffee' }],
      transferConfig,
    )
    expect(result[0].type).toBe('expense')
    expect(result[0].transferAccountId).toBeNull()
  })

  it('first matching transfer rule wins', () => {
    const result = applyMapping(
      [{ 'Posting Date': '03/05/2026', Amount: '1200.00', Description: 'PAYMENT THANK YOU ACH' }],
      {
        ...transferConfig,
        transferRules: [
          { keyword: 'payment thank you', transferAccountId: 42 },
          { keyword: 'ach', transferAccountId: 99 },
        ],
      },
    )
    expect(result[0].transferAccountId).toBe(42)
  })

  it('empty transferRules leaves type and transferAccountId unchanged', () => {
    const result = applyMapping(
      [{ 'Posting Date': '03/01/2026', Amount: '-40.00', Description: 'Coffee' }],
      { ...transferConfig, transferRules: [] },
    )
    expect(result[0].type).toBe('expense')
    expect(result[0].transferAccountId).toBeNull()
  })
})
