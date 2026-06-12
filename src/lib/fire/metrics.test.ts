import { describe, it, expect } from 'vitest'
import { fireNumber, latestBalances, currentNetWorth, investableNetWorth, fiProgress } from './metrics'
import type { FireAccount, FireBalance } from './types'

const accounts: FireAccount[] = [
  { id: 1, type: 'brokerage', includeInFireCalculations: true },
  { id: 2, type: 'checking', includeInFireCalculations: false },
  { id: 3, type: 'liability', includeInFireCalculations: false },
]
const balances: FireBalance[] = [
  { accountId: 1, balance: 100, recordedAt: '2026-01-01' },
  { accountId: 1, balance: 200, recordedAt: '2026-02-01' },
  { accountId: 2, balance: 50, recordedAt: '2026-01-15' },
  { accountId: 3, balance: 30, recordedAt: '2026-01-10' },
]

describe('fire metrics', () => {
  it('fireNumber is 25x expenses', () => { expect(fireNumber(40000)).toBe(1_000_000) })
  it('latestBalances picks most recent per account', () => {
    const m = latestBalances(balances)
    expect(m.get(1)).toBe(200); expect(m.get(2)).toBe(50)
  })
  it('currentNetWorth subtracts liabilities', () => { expect(currentNetWorth(accounts, balances)).toBe(220) })
  it('investableNetWorth counts only included accounts', () => { expect(investableNetWorth(accounts, balances)).toBe(200) })
  it('fiProgress is investable / fireNumber * 100', () => { expect(fiProgress(200_000, 1_000_000)).toBe(20) })
})
