import type { Account } from '../types/Account'
import type { SimpleFinStatus } from '../types/SimpleFinStatus'

/**
 * Local accounts whose SimpleFIN connection needs attention, mapped to the
 * message explaining why. Two sources:
 *
 * - `lastError`: the whole connection failed (revoked token, bridge down) —
 *   every linked account is affected.
 * - `bridgeErrors`: per-institution messages like "Connection to <bank> may
 *   need attention". These name the bank, not the account, so they're
 *   attributed by matching the remote account's `org` inside the message.
 *   Errors naming no known org can't be placed on an account and are only
 *   shown on the Bank Sync settings page.
 */
export function accountsNeedingAttention(
  accounts: Account[],
  status: SimpleFinStatus | null,
): Map<number, string> {
  const result = new Map<number, string>()
  if (!status?.connected) return result

  const remoteById = new Map(status.accounts.map((r) => [r.id, r]))

  for (const account of accounts) {
    if (account.simplefinId == null) continue
    if (status.lastError) {
      result.set(account.id, status.lastError)
      continue
    }
    const org = remoteById.get(account.simplefinId)?.org
    if (!org) continue
    const match = status.bridgeErrors.find((e) =>
      e.toLowerCase().includes(org.toLowerCase()),
    )
    if (match) result.set(account.id, match)
  }

  return result
}
