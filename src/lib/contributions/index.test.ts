import { describe, it, expect } from 'vitest'
import { buildContributionRows } from './index'
import { resolveYearLimits } from './irsLimits'
import type { Transaction } from '../types/Transaction'
import type { Account } from '../types/Account'

const limits = resolveYearLimits(2025).limits

function acct(id: number, type: string): Account {
  return {
    id, name: `${type} account`, type, institution: null,
    isActive: true, includeInFireCalculations: true, createdAt: '2025-01-01',
  }
}

function txn(id: number, accountId: number, amount: number, date: string): Transaction {
  return {
    id, accountId, transferAccountId: null, amount, description: 'Contribution',
    date, type: 'income', category: 'savings', isContribution: true, isWithdrawal: false,
    importSource: 'manual', generatedBalanceId: null, generatedBalanceToId: null,
    paycheckId: null, createdAt: date, updatedAt: date,
  }
}

// A withdrawal: a transfer OUT of an investment account (the source, `accountId`)
// to cash. Flagged as a contribution (the gate) with isWithdrawal set.
function withdrawal(id: number, fromInvestmentId: number, toCashId: number, amount: number, date: string): Transaction {
  return {
    id, accountId: fromInvestmentId, transferAccountId: toCashId, amount, description: 'Withdrawal',
    date, type: 'transfer', category: 'transfer', isContribution: true, isWithdrawal: true,
    importSource: 'manual', generatedBalanceId: null, generatedBalanceToId: null,
    paycheckId: null, createdAt: date, updatedAt: date,
  }
}

describe('buildContributionRows', () => {
  it('groups contributions by account type into one row per type', () => {
    const accounts = [acct(1, '401k'), acct(2, 'hsa')]
    const txns = [
      txn(10, 1, 1000, '2025-03-01'),
      txn(11, 2, 500, '2025-03-01'),
    ]
    const rows = buildContributionRows(txns, accounts, 2025, 30, 'self', limits)
    const k401 = rows.find((r) => r.label === '401k / Roth 401k')!
    const hsa = rows.find((r) => r.label === 'HSA')!
    expect(k401.total).toBe(1000)
    expect(hsa.total).toBe(500)
  })

  it('merges 401k + roth_401k into one row with a breakdown', () => {
    const accounts = [acct(1, '401k'), acct(2, 'roth_401k')]
    const txns = [
      txn(10, 1, 12000, '2025-06-01'),
      txn(11, 2, 6000, '2025-06-01'),
    ]
    const rows = buildContributionRows(txns, accounts, 2025, 30, 'self', limits)
    const k401 = rows.find((r) => r.label === '401k / Roth 401k')!
    expect(k401.total).toBe(18000)
    expect(k401.limit).toBe(23500)
    expect(k401.breakdown).toEqual([
      { type: '401k', label: '401k', total: 12000 },
      { type: 'roth_401k', label: 'Roth 401k', total: 6000 },
    ])
    expect(k401.pctUsed).toBeCloseTo(18000 / 23500)
  })

  it('omits the breakdown when only one subtype in a merged group has contributions', () => {
    const accounts = [acct(1, '401k'), acct(2, 'roth_401k')]
    const txns = [txn(10, 1, 12000, '2025-06-01')]
    const rows = buildContributionRows(txns, accounts, 2025, 30, 'self', limits)
    const k401 = rows.find((r) => r.label === '401k / Roth 401k')!
    expect(k401.total).toBe(12000)
    expect(k401.breakdown).toBeUndefined()
  })

  it('counts mixed_401k toward the 401k group and shows it in the breakdown', () => {
    const accounts = [acct(1, '401k'), acct(2, 'roth_401k'), acct(3, 'mixed_401k')]
    const txns = [
      txn(10, 1, 12000, '2025-06-01'),
      txn(11, 2, 6000, '2025-06-01'),
      txn(12, 3, 5000, '2025-06-01'),
    ]
    const rows = buildContributionRows(txns, accounts, 2025, 30, 'self', limits)
    const k401 = rows.find((r) => r.label === '401k / Roth 401k')!
    expect(k401.total).toBe(23000)
    expect(k401.limit).toBe(23500)
    expect(k401.breakdown).toEqual([
      { type: '401k', label: '401k', total: 12000 },
      { type: 'roth_401k', label: 'Roth 401k', total: 6000 },
      { type: 'mixed_401k', label: 'Mixed 401k', total: 5000 },
    ])
  })

  it('omits account types with no contributions in either year', () => {
    const accounts = [acct(1, '401k')]
    const txns = [txn(10, 1, 1000, '2025-03-01')]
    const rows = buildContributionRows(txns, accounts, 2025, 30, 'self', limits)
    expect(rows.find((r) => r.label === 'HSA')).toBeUndefined()
    expect(rows).toHaveLength(1)
  })

  it('orders limited types before unlimited types', () => {
    const accounts = [acct(1, 'brokerage'), acct(2, '401k')]
    const txns = [
      txn(10, 1, 3000, '2025-03-01'),
      txn(11, 2, 1000, '2025-03-01'),
    ]
    const rows = buildContributionRows(txns, accounts, 2025, 30, 'self', limits)
    expect(rows[0].label).toBe('401k / Roth 401k')
    expect(rows[1].label).toBe('Brokerage')
    expect(rows[1].limit).toBeUndefined()
  })

  it('applies 401k catch-up when age >= 50', () => {
    const accounts = [acct(1, '401k')]
    const txns = [txn(10, 1, 1000, '2025-03-01')]
    const rows = buildContributionRows(txns, accounts, 2025, 55, 'self', limits)
    const k401 = rows.find((r) => r.label === '401k / Roth 401k')!
    expect(k401.limitBase).toBe(23500)
    expect(k401.catchUpAmount).toBe(7500)
    expect(k401.limit).toBe(31000)
  })

  it('does not apply catch-up when age < 50', () => {
    const accounts = [acct(1, '401k')]
    const txns = [txn(10, 1, 1000, '2025-03-01')]
    const rows = buildContributionRows(txns, accounts, 2025, 49, 'self', limits)
    const k401 = rows.find((r) => r.label === '401k / Roth 401k')!
    expect(k401.catchUpAmount).toBeUndefined()
    expect(k401.limit).toBe(23500)
  })

  it('applies HSA catch-up at age >= 55, not at 50', () => {
    const accounts = [acct(1, 'hsa')]
    const txns = [txn(10, 1, 100, '2025-03-01')]
    const at50 = buildContributionRows(txns, accounts, 2025, 50, 'self', limits)
      .find((r) => r.label === 'HSA')!
    expect(at50.catchUpAmount).toBeUndefined()
    const at55 = buildContributionRows(txns, accounts, 2025, 55, 'self', limits)
      .find((r) => r.label === 'HSA')!
    expect(at55.catchUpAmount).toBe(1000)
    expect(at55.limit).toBe(5300) // 4300 self + 1000
  })

  it('uses the family HSA limit when coverage is family', () => {
    const accounts = [acct(1, 'hsa')]
    const txns = [txn(10, 1, 100, '2025-03-01')]
    const rows = buildContributionRows(txns, accounts, 2025, 30, 'family', limits)
    const hsa = rows.find((r) => r.label === 'HSA')!
    expect(hsa.limit).toBe(8550)
  })

  it('computes yoyDelta as this year total minus prior year total', () => {
    const accounts = [acct(1, '401k')]
    const txns = [
      txn(10, 1, 5000, '2025-03-01'),
      txn(11, 1, 3000, '2024-03-01'),
    ]
    const rows = buildContributionRows(txns, accounts, 2025, 30, 'self', limits)
    const k401 = rows.find((r) => r.label === '401k / Roth 401k')!
    expect(k401.total).toBe(5000)
    expect(k401.yoyDelta).toBe(2000)
  })

  it('lets pctUsed exceed 1.0 on over-contribution (no clamping)', () => {
    const accounts = [acct(1, 'traditional_ira')]
    const txns = [txn(10, 1, 8000, '2025-03-01')]
    const rows = buildContributionRows(txns, accounts, 2025, 30, 'self', limits)
    const ira = rows.find((r) => r.label === 'Traditional / Roth IRA')!
    expect(ira.pctUsed).toBeGreaterThan(1)
  })

  it('sums multiple accounts of the same type into one row', () => {
    const accounts = [acct(1, '401k'), acct(2, '401k')]
    const txns = [
      txn(10, 1, 4000, '2025-03-01'),
      txn(11, 2, 3000, '2025-04-01'),
    ]
    const rows = buildContributionRows(txns, accounts, 2025, 30, 'self', limits)
    const k401 = rows.find((r) => r.label === '401k / Roth 401k')!
    expect(k401.total).toBe(7000)
  })

  it('ignores contributions whose account no longer exists (orphan txn)', () => {
    const accounts = [acct(1, '401k')]
    const txns = [
      txn(10, 1, 1000, '2025-03-01'),
      txn(11, 99, 5000, '2025-03-01'), // accountId 99 has no matching account
    ]
    const rows = buildContributionRows(txns, accounts, 2025, 30, 'self', limits)
    expect(rows).toHaveLength(1)
    expect(rows[0].total).toBe(1000)
  })

  it('nets down a row total with a negative (refund) contribution amount', () => {
    const accounts = [acct(1, '401k')]
    const txns = [
      txn(10, 1, 5000, '2025-03-01'),
      txn(11, 1, -1000, '2025-04-01'), // correcting refund of contribution
    ]
    const rows = buildContributionRows(txns, accounts, 2025, 30, 'self', limits)
    const k401 = rows.find((r) => r.label === '401k / Roth 401k')!
    expect(k401.total).toBe(4000)
  })

  it('nets a withdrawal out of an unlimited (brokerage) total, attributed to its source account', () => {
    const accounts = [acct(1, 'checking'), acct(2, 'brokerage')]
    const txns = [
      txn(10, 2, 5000, '2025-03-01'),            // contribute into brokerage
      withdrawal(11, 2, 1, 2000, '2025-04-01'),  // pull $2k back out to checking
    ]
    const rows = buildContributionRows(txns, accounts, 2025, 30, 'self', limits)
    const brokerage = rows.find((r) => r.label === 'Brokerage')!
    expect(brokerage.total).toBe(3000) // 5000 − 2000; source-attributed, not lost to checking
  })

  it('ignores withdrawals for IRS-limited groups — limit counts gross inflows only', () => {
    const accounts = [acct(1, 'checking'), acct(2, 'roth_ira')]
    const txns = [
      txn(10, 2, 7000, '2025-03-01'),            // contribute into Roth IRA
      withdrawal(11, 2, 1, 3000, '2025-04-01'),  // withdraw $3k — must not restore limit room
    ]
    const rows = buildContributionRows(txns, accounts, 2025, 30, 'self', limits)
    const ira = rows.find((r) => r.label === 'Traditional / Roth IRA')!
    expect(ira.total).toBe(7000) // withdrawal does not net the inflow-only total down
    expect(ira.pctUsed).toBeCloseTo(7000 / 7000)
  })
})
