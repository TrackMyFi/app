import { describe, it, expect } from 'vitest'
import { contributionItems, findDuplicateDeposit, paycheckBreakdown, paycheckTotals, type ExistingTxnRef } from './index'
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

describe('paycheckBreakdown', () => {
  const make = (over: Partial<Paycheck>): Paycheck => ({
    federalTax: 0, stateTax: 0, localTax: 0, socialSecurityTax: 0, medicareTax: 0,
    deductions: [], ...over,
  } as Paycheck)

  it('splits gross into net plus withheld, itemizing taxes and deductions', () => {
    const paychecks = [
      make({
        grossAmount: 5000, netAmount: 3500,
        federalTax: 600, stateTax: 200, localTax: 0, socialSecurityTax: 310, medicareTax: 90,
        deductions: [
          { label: '401k', amount: 250, preTax: true, contributionAccountType: '401k', accountId: 1 },
          { label: 'Health', amount: 50, preTax: true, contributionAccountType: null, accountId: null },
        ],
      }),
    ]
    expect(paycheckBreakdown(paychecks)).toEqual({
      totalGross: 5000,
      totalNet: 3500,
      totalTaxes: 1200,
      totalDeductions: 300,
      totalWithheld: 1500,
      takeHomeRate: 0.7,
      count: 1,
    })
  })

  it('sums taxes and deductions across multiple paychecks without drift', () => {
    const paychecks = [
      make({ grossAmount: 2500.25, netAmount: 1800.10, federalTax: 300.15, deductions: [{ label: 'a', amount: 100.05, preTax: true, contributionAccountType: null, accountId: null }] }),
      make({ grossAmount: 2499.75, netAmount: 1799.90, federalTax: 299.85, deductions: [{ label: 'b', amount: 99.95, preTax: false, contributionAccountType: null, accountId: null }] }),
    ]
    const r = paycheckBreakdown(paychecks)
    expect(r.totalGross).toBe(5000)
    expect(r.totalNet).toBe(3600)
    expect(r.totalTaxes).toBe(600)
    expect(r.totalDeductions).toBe(200)
    expect(r.totalWithheld).toBe(1400)
  })

  it('returns a zero take-home rate (not NaN) when there is no gross', () => {
    expect(paycheckBreakdown([])).toEqual({
      totalGross: 0, totalNet: 0, totalTaxes: 0, totalDeductions: 0,
      totalWithheld: 0, takeHomeRate: 0, count: 0,
    })
  })

  it('clamps withheld to zero when net somehow exceeds gross', () => {
    const paychecks = [make({ grossAmount: 1000, netAmount: 1200 })]
    expect(paycheckBreakdown(paychecks).totalWithheld).toBe(0)
  })
})

describe('findDuplicateDeposit', () => {
  const existingDeposit: ExistingTxnRef = {
    id: 1,
    amount: 1500,
    date: '2026-05-21',
    description: 'DIRECT DEP ACME CORP PAYROLL',
    type: 'income',
    paycheckId: null,
  }

  it('finds a same-amount deposit within the date window', () => {
    expect(findDuplicateDeposit({ amount: 1500, date: '2026-05-20' }, [existingDeposit])).toEqual(existingDeposit)
  })

  it('returns null when the date is outside the ±3-day window', () => {
    expect(findDuplicateDeposit({ amount: 1500, date: '2026-05-26' }, [existingDeposit])).toBeNull()
  })

  it('returns null when amounts differ', () => {
    expect(findDuplicateDeposit({ amount: 900, date: '2026-05-21' }, [existingDeposit])).toBeNull()
  })

  it('ignores a transaction already linked to a paycheck (contribution or a prior save of the same paycheck)', () => {
    const linked: ExistingTxnRef = { ...existingDeposit, paycheckId: 42 }
    expect(findDuplicateDeposit({ amount: 1500, date: '2026-05-21' }, [linked])).toBeNull()
  })

  it('ignores non-income rows (e.g. an expense of the same amount)', () => {
    const expense: ExistingTxnRef = { ...existingDeposit, type: 'expense' }
    expect(findDuplicateDeposit({ amount: 1500, date: '2026-05-21' }, [expense])).toBeNull()
  })

  it('returns null when there is nothing to match', () => {
    expect(findDuplicateDeposit({ amount: 1500, date: '2026-05-21' }, [])).toBeNull()
  })
})
