/**
 * Minimal shape needed to order balance snapshots by recency.
 * `id` is optional so inputs that don't carry it (older FIRE fixtures) still
 * compile; when absent, ties fall back to existing iteration/stable order.
 */
export interface RecencyKey {
  recordedAt: string
  id?: number
}

/**
 * Canonical recency ordering for balance snapshots: the latest `recordedAt`
 * wins, ties broken by highest `id` (a later insert is more recent).
 *
 * `recorded_at` is stored at date-only granularity, so several snapshots can
 * share a date (e.g. a manual snapshot plus a transaction-linked one recorded
 * the same day). This mirrors the Rust `base_balance` query
 * (`ORDER BY recorded_at DESC, id DESC`) and `balancePreview` so every surface
 * agrees on which same-date snapshot is "current".
 */
export function isNewer(a: RecencyKey, b: RecencyKey): boolean {
  if (a.recordedAt !== b.recordedAt) return a.recordedAt > b.recordedAt
  return (a.id ?? -Infinity) > (b.id ?? -Infinity)
}

/** Picks the most recent snapshot from a list, or undefined if empty. */
export function latestSnapshot<T extends RecencyKey>(items: readonly T[]): T | undefined {
  let best: T | undefined
  for (const item of items) {
    if (!best || isNewer(item, best)) best = item
  }
  return best
}

/** Ascending recency comparator (oldest first); same-date ties ordered by id. */
export function byRecencyAsc(a: RecencyKey, b: RecencyKey): number {
  if (a.recordedAt !== b.recordedAt) return a.recordedAt < b.recordedAt ? -1 : 1
  if (a.id == null || b.id == null) return 0
  return a.id - b.id
}

/** Descending recency comparator (newest first); same-date ties newest by id. */
export function byRecencyDesc(a: RecencyKey, b: RecencyKey): number {
  return byRecencyAsc(b, a)
}
