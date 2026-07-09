import { describe, it, expect } from 'vitest'
import { coreText, normalizeKey, isGenericKey, displayName, groupByMerchant, OTHER_MERCHANT_KEY } from './merchants'
import type { Transaction } from '../types/Transaction'
import type { Account } from '../types/Account'

const accounts: Account[] = [
  { id: 1, name: 'Checking', type: 'checking', institution: null, isActive: true, includeInFireCalculations: false, createdAt: '', simplefinId: null, countPaymentsAsExpense: false, traditionalPct: null },
]

function tx(overrides: Partial<Transaction>): Transaction {
  return {
    id: 1, accountId: 1, transferAccountId: null, amount: 10, description: '', date: '2026-05-01',
    type: 'expense', category: 'discretionary', isContribution: false, isWithdrawal: false, isRefund: false, importSource: 'manual',
    generatedBalanceId: null, generatedBalanceToId: null, paycheckId: null, vendorCategory: null, simplefinId: null, suppressedAs: null, rawDescription: null, createdAt: '', updatedAt: '',
    ...overrides,
  }
}

describe('coreText', () => {
  it('strips processor prefixes', () => {
    expect(coreText('SQ *BLUE BOTTLE COFFEE #123')).toBe('BLUE BOTTLE COFFEE')
  })

  it('strips CHECKCARD prefix, posting date, and a trailing .com TLD', () => {
    expect(coreText('CHECKCARD 0615 NETFLIX.COM')).toBe('NETFLIX')
  })

  it('strips trailing store numbers', () => {
    expect(coreText('STARBUCKS STORE #04471')).toBe('STARBUCKS STORE')
  })

  it('leaves an already-clean description alone', () => {
    expect(coreText('Local Coffee Shop')).toBe('Local Coffee Shop')
  })
})

describe('normalizeKey / isGenericKey', () => {
  it('uppercases the core text for case-insensitive clustering', () => {
    expect(normalizeKey('Netflix.com')).toBe('NETFLIX')
  })

  it('flags short, purely-numeric, or known-generic descriptions as generic', () => {
    expect(isGenericKey('AB')).toBe(true)
    expect(isGenericKey('1234567')).toBe(true)
    expect(isGenericKey('DEBIT CARD PURCHASE')).toBe(true)
    expect(isGenericKey('NETFLIX')).toBe(false)
  })
})

describe('displayName', () => {
  it('title-cases the normalized core', () => {
    expect(displayName('SQ *BLUE BOTTLE COFFEE #123')).toBe('Blue Bottle Coffee')
  })
})

describe('groupByMerchant', () => {
  it('clusters repeat payees across noisy description variants and sorts by total spend', () => {
    const groups = groupByMerchant([
      tx({ description: 'SQ *BLUE BOTTLE COFFEE #123', amount: 6, category: 'discretionary' }),
      tx({ description: 'BLUE BOTTLE COFFEE #456', amount: 7, category: 'discretionary' }),
      tx({ description: 'CHECKCARD 0615 NETFLIX.COM', amount: 15.49, category: 'discretionary' }),
    ], accounts)

    expect(groups).toHaveLength(2)
    // Sorted by total spend descending: Netflix (15.49) before Blue Bottle Coffee (13)
    expect(groups[0]).toMatchObject({ displayName: 'Netflix', total: 15.49, count: 1 })
    expect(groups[1]).toMatchObject({ displayName: 'Blue Bottle Coffee', total: 13, count: 2, category: 'discretionary' })
  })

  it('excludes savings/contributions and transactions with no cash-flow bucket', () => {
    const groups = groupByMerchant([
      tx({ description: 'BROKERAGE CONTRIBUTION', amount: 500, type: 'transfer', isContribution: true }),
      tx({ description: 'PAYCHECK', amount: 2000, type: 'income' }),
    ], accounts)
    expect(groups).toHaveLength(0)
  })

  it('pools generic descriptions into a single "Other purchases" group instead of scattering', () => {
    const groups = groupByMerchant([
      tx({ description: 'DEBIT CARD PURCHASE', amount: 20 }),
      tx({ description: '1234567', amount: 5 }),
    ], accounts)
    expect(groups).toHaveLength(1)
    expect(groups[0]).toMatchObject({ key: OTHER_MERCHANT_KEY, displayName: 'Other purchases', total: 25, count: 2 })
  })

  it('assigns share as a fraction of total grouped spend', () => {
    const groups = groupByMerchant([
      tx({ description: 'NETFLIX', amount: 25 }),
      tx({ description: 'HULU', amount: 75 }),
    ], accounts)
    const netflix = groups.find((g) => g.displayName === 'Netflix')!
    const hulu = groups.find((g) => g.displayName === 'Hulu')!
    expect(netflix.share).toBeCloseTo(0.25)
    expect(hulu.share).toBeCloseTo(0.75)
  })

  it('merges noisy description variants into one vendor when a rule matches', () => {
    const groups = groupByMerchant([
      tx({ description: 'PIZZA HUT 029908 HEBRON KY NULL', amount: 20 }),
      tx({ description: 'PIZZA HUT 029908 HEBRON KY', amount: 15 }),
    ], accounts, [{ keyword: 'pizza hut', vendorName: 'Pizza Hut' }])

    expect(groups).toHaveLength(1)
    expect(groups[0]).toMatchObject({ displayName: 'Pizza Hut', total: 35, count: 2, searchTerm: 'pizza hut' })
  })

  it('rescues an otherwise-generic description via a rule', () => {
    const groups = groupByMerchant([
      tx({ description: 'DEBIT CARD PURCHASE KYGOV KY TAXPMNT', amount: 40 }),
    ], accounts, [{ keyword: 'kygov', vendorName: 'KY State Taxes' }])

    expect(groups).toHaveLength(1)
    expect(groups[0]).toMatchObject({ displayName: 'KY State Taxes', total: 40 })
  })

  it('prefers the longest matching keyword when multiple rules match', () => {
    const groups = groupByMerchant([
      tx({ description: 'AMAZON MKTPL*0U36P84J3 SEATTLE WA', amount: 30 }),
    ], accounts, [
      { keyword: 'amazon', vendorName: 'Amazon (general)' },
      { keyword: 'amazon mktpl', vendorName: 'Amazon Marketplace' },
    ])

    expect(groups[0].displayName).toBe('Amazon Marketplace')
  })

  it('preserves vendor name casing as the user typed it', () => {
    const groups = groupByMerchant([
      tx({ description: 'sq *blue bottle coffee #123', amount: 6 }),
    ], accounts, [{ keyword: 'blue bottle', vendorName: 'Blue Bottle Coffee Co.' }])

    expect(groups[0].displayName).toBe('Blue Bottle Coffee Co.')
  })
})
