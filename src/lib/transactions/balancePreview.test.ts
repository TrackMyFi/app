import { describe, it, expect } from 'vitest'
import { balancePreview } from './balancePreview'
import type { AccountBalance } from '../types/AccountBalance'

const balances: AccountBalance[] = [
  { id: 1, accountId: 10, balance: 1000, recordedAt: '2026-02-01' },
  { id: 2, accountId: 20, balance: 200, recordedAt: '2026-02-01' },
]

describe('balancePreview', () => {
  it('previews an expense against the latest on/before date', () => {
    const p = balancePreview(balances, { type: 'expense', amount: 40, accountId: 10, transferAccountId: null, date: '2026-03-01' })
    expect(p).toEqual([{ accountId: 10, from: 1000, to: 960 }])
  })

  it('previews income', () => {
    const p = balancePreview(balances, { type: 'income', amount: 500, accountId: 10, transferAccountId: null, date: '2026-03-01' })
    expect(p).toEqual([{ accountId: 10, from: 1000, to: 1500 }])
  })

  it('previews both sides of a transfer', () => {
    const p = balancePreview(balances, { type: 'transfer', amount: 300, accountId: 10, transferAccountId: 20, date: '2026-03-01' })
    expect(p).toEqual([
      { accountId: 10, from: 1000, to: 700 },
      { accountId: 20, from: 200, to: 500 },
    ])
  })

  it('uses base 0 when no prior snapshot exists', () => {
    const p = balancePreview(balances, { type: 'expense', amount: 40, accountId: 99, transferAccountId: null, date: '2026-03-01' })
    expect(p).toEqual([{ accountId: 99, from: 0, to: -40 }])
  })

  it('ignores snapshots dated after the transaction', () => {
    const future: AccountBalance[] = [{ id: 3, accountId: 10, balance: 5000, recordedAt: '2026-12-01' }]
    const p = balancePreview([...balances, ...future], { type: 'expense', amount: 40, accountId: 10, transferAccountId: null, date: '2026-03-01' })
    expect(p).toEqual([{ accountId: 10, from: 1000, to: 960 }])
  })
})
