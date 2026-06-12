import type { AccountBalance } from '../types/AccountBalance'
import { signedDelta } from './constants'

export interface PreviewInput {
  type: string
  amount: number
  accountId: number
  transferAccountId: number | null
  date: string
}

export interface PreviewLine {
  accountId: number
  from: number
  to: number
}

function baseBalance(balances: AccountBalance[], accountId: number, date: string): number {
  const candidates = balances
    .filter((b) => b.accountId === accountId && b.recordedAt <= date)
    .sort((a, b) => (a.recordedAt < b.recordedAt ? 1 : a.recordedAt > b.recordedAt ? -1 : b.id - a.id))
  return candidates.length ? candidates[0].balance : 0
}

/** Mirrors the Rust `materialize_snapshots` math for the form preview. */
export function balancePreview(balances: AccountBalance[], input: PreviewInput): PreviewLine[] {
  if (input.type === 'transfer') {
    if (input.transferAccountId == null) return []
    const srcBase = baseBalance(balances, input.accountId, input.date)
    const dstBase = baseBalance(balances, input.transferAccountId, input.date)
    return [
      { accountId: input.accountId, from: srcBase, to: srcBase - input.amount },
      { accountId: input.transferAccountId, from: dstBase, to: dstBase + input.amount },
    ]
  }
  const base = baseBalance(balances, input.accountId, input.date)
  return [{ accountId: input.accountId, from: base, to: base + signedDelta(input.type, input.amount) }]
}
