import { describe, it, expect } from 'vitest'
import { applyMapping, detectDuplicates, type MappingConfig } from './mapping'

const config: MappingConfig = {
  dateColumn: 'Posting Date',
  amountColumn: 'Amount',
  descriptionColumn: 'Description',
  dateFormat: 'MM/dd/yyyy',
  amountSign: 'negative-is-expense',
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
