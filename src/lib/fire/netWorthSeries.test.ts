import { describe, it, expect } from 'vitest'
import { netWorthOverTime } from './netWorthSeries'
import type { FireAccount, FireBalance } from './types'

const accounts: FireAccount[] = [
  { id: 1, type: 'brokerage', includeInFireCalculations: true },
  { id: 2, type: 'liability', includeInFireCalculations: false },
]
const balances: FireBalance[] = [
  { accountId: 1, balance: 100, recordedAt: '2026-01-01' },
  { accountId: 2, balance: 40, recordedAt: '2026-01-15' },
  { accountId: 1, balance: 300, recordedAt: '2026-02-01' },
]

describe('netWorthOverTime', () => {
  it('computes net worth at each distinct date using carry-forward', () => {
    expect(netWorthOverTime(accounts, balances)).toEqual([
      { date: '2026-01-01', netWorth: 100 },
      { date: '2026-01-15', netWorth: 60 },
      { date: '2026-02-01', netWorth: 260 },
    ])
  })
  it('returns empty for no balances', () => { expect(netWorthOverTime(accounts, [])).toEqual([]) })
})
