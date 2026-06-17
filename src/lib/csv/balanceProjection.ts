import { signedDelta } from '../transactions/constants'

export interface ProjectionRow {
  date: string
  amount: number
  type: string
}

/**
 * Returns a running balance for each row after cascading through included rows
 * in date order. Excluded rows return null and do not affect the running total.
 * Rows are keyed by their original array index so the result maps 1:1 to input order.
 * Caller must ensure `included.length === rows.length`.
 *
 * Rows are rendered in array order (bank exports list newest first). Within a single
 * date the cascade therefore processes by DESCENDING index — the reverse of display
 * order — so each rendered row differs from the one below it by exactly its own delta
 * and the Balance column reads consistently instead of climbing within a same-date run.
 */
export function projectRunningBalances(
  rows: ProjectionRow[],
  included: boolean[],
  baseBalance: number,
  isLiability = false,
): (number | null)[] {
  const sorted = rows
    .map((row, i) => ({ row, i }))
    .filter(({ i }) => included[i])
    .sort((a, b) => a.row.date.localeCompare(b.row.date) || b.i - a.i)

  const balanceAt = new Map<number, number>()
  let running = baseBalance
  for (const { row, i } of sorted) {
    running += signedDelta(row.type, row.amount, isLiability)
    balanceAt.set(i, running)
  }

  return rows.map((_, i) => balanceAt.get(i) ?? null)
}
