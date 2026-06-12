import { describe, it, expect } from 'vitest'
import { contributionItems, paycheckTotals } from './index'
import type { Paycheck } from '../types/Paycheck'

describe('contributionItems', () => {
  it('includes a deduction that has both contributionAccountType and accountId', () => {
    const deductions = [
      { label: '401k', amount: 750, preTax: true, contributionAccountType: '401k', accountId: 1 },
    ]
    const items = contributionItems(deductions, [])
    expect(items).toHaveLength(1)
    expect(items[0]).toEqual({ label: '401k', amount: 750, accountId: 1 })
  })

  it('excludes a deduction missing accountId', () => {
    const deductions = [
      { label: '401k', amount: 750, preTax: true, contributionAccountType: '401k', accountId: null },
    ]
    expect(contributionItems(deductions, [])).toHaveLength(0)
  })

  it('excludes a deduction missing contributionAccountType', () => {
    const deductions = [
      { label: 'Health', amount: 180, preTax: true, contributionAccountType: null, accountId: 1 },
    ]
    expect(contributionItems(deductions, [])).toHaveLength(0)
  })

  it('includes an employer match item that has accountId', () => {
    const match = [{ label: '401k Match', amount: 375, accountId: 2 }]
    const items = contributionItems([], match)
    expect(items).toHaveLength(1)
    expect(items[0]).toEqual({ label: '401k Match', amount: 375, accountId: 2 })
  })

  it('excludes an employer match item without accountId', () => {
    const match = [{ label: '401k Match', amount: 375, accountId: null }]
    expect(contributionItems([], match)).toHaveLength(0)
  })

  it('combines qualifying deductions and match items', () => {
    const deductions = [
      { label: '401k', amount: 750, preTax: true, contributionAccountType: '401k', accountId: 1 },
      { label: 'Health', amount: 180, preTax: true, contributionAccountType: null, accountId: null },
    ]
    const match = [
      { label: '401k Match', amount: 375, accountId: 1 },
      { label: 'Unlinked Match', amount: 100, accountId: null },
    ]
    const items = contributionItems(deductions, match)
    expect(items).toHaveLength(2)
  })

  it('returns empty for empty inputs', () => {
    expect(contributionItems([], [])).toHaveLength(0)
  })
})

describe('paycheckTotals', () => {
  it('sums grossAmount and netAmount across paychecks', () => {
    const paychecks = [
      { grossAmount: 5000, netAmount: 3500 },
      { grossAmount: 4800, netAmount: 3200 },
    ] as Paycheck[]
    expect(paycheckTotals(paychecks)).toEqual({ totalGross: 9800, totalNet: 6700, count: 2 })
  })

  it('returns zeros for an empty array', () => {
    expect(paycheckTotals([])).toEqual({ totalGross: 0, totalNet: 0, count: 0 })
  })

  it('handles fractional dollar amounts without floating-point drift', () => {
    const paychecks = [
      { grossAmount: 5000.50, netAmount: 3200.47 },
      { grossAmount: 4999.50, netAmount: 3199.53 },
    ] as Paycheck[]
    expect(paycheckTotals(paychecks)).toEqual({ totalGross: 10000, totalNet: 6400, count: 2 })
  })
})
