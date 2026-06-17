import { describe, it, expect } from 'vitest'
import { balancePreview } from './balancePreview'
import type { AccountBalance } from '../types/AccountBalance'

const balances: AccountBalance[] = [
  { id: 1, accountId: 10, balance: 1000, recordedAt: '2026-02-01', linkedTransactionId: null },
  { id: 2, accountId: 20, balance: 200, recordedAt: '2026-02-01', linkedTransactionId: null },
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
    const future: AccountBalance[] = [{ id: 3, accountId: 10, balance: 5000, recordedAt: '2026-12-01', linkedTransactionId: null }]
    const p = balancePreview([...balances, ...future], { type: 'expense', amount: 40, accountId: 10, transferAccountId: null, date: '2026-03-01' })
    expect(p).toEqual([{ accountId: 10, from: 1000, to: 960 }])
  })

  it('inverts income/expense for a liability account (debt owed)', () => {
    const liab = new Set([10])
    const purchase = balancePreview(balances, { type: 'expense', amount: 40, accountId: 10, transferAccountId: null, date: '2026-03-01' }, liab)
    expect(purchase).toEqual([{ accountId: 10, from: 1000, to: 1040 }]) // purchase raises debt
    const refund = balancePreview(balances, { type: 'income', amount: 100, accountId: 10, transferAccountId: null, date: '2026-03-01' }, liab)
    expect(refund).toEqual([{ accountId: 10, from: 1000, to: 900 }]) // refund lowers debt
  })

  it('a payment from an asset into a liability lowers both balances', () => {
    // accountId 10 = checking (asset, source), accountId 20 = card (liability, destination)
    const p = balancePreview(balances, { type: 'transfer', amount: 150, accountId: 10, transferAccountId: 20, date: '2026-03-01' }, new Set([20]))
    expect(p).toEqual([
      { accountId: 10, from: 1000, to: 850 }, // asset source falls
      { accountId: 20, from: 200, to: 50 },   // liability destination: debt falls
    ])
  })
})
