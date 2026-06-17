import type { AccountBalance } from '../types/AccountBalance'
import { signedDelta, transferLegDelta } from './constants'

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

/**
 * Mirrors the Rust `materialize_snapshots` math for the form preview.
 * `liabilityIds` carries the set of accounts whose balances represent debt owed,
 * so their sign inverts relative to assets.
 */
export function balancePreview(
  balances: AccountBalance[],
  input: PreviewInput,
  liabilityIds: Set<number> = new Set(),
): PreviewLine[] {
  if (input.type === 'transfer') {
    if (input.transferAccountId == null) return []
    const srcBase = baseBalance(balances, input.accountId, input.date)
    const dstBase = baseBalance(balances, input.transferAccountId, input.date)
    return [
      {
        accountId: input.accountId,
        from: srcBase,
        to: srcBase + transferLegDelta('source', input.amount, liabilityIds.has(input.accountId)),
      },
      {
        accountId: input.transferAccountId,
        from: dstBase,
        to: dstBase + transferLegDelta('destination', input.amount, liabilityIds.has(input.transferAccountId)),
      },
    ]
  }
  const base = baseBalance(balances, input.accountId, input.date)
  const delta = signedDelta(input.type, input.amount, liabilityIds.has(input.accountId))
  return [{ accountId: input.accountId, from: base, to: base + delta }]
}
