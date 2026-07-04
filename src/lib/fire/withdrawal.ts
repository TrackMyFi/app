/**
 * The 4% rule — the inverse of the ×25 multiplier in `fireNumber`. If either
 * changes, they must change together or FI progress and safe income disagree.
 */
export const SAFE_WITHDRAWAL_RATE = 0.04

/**
 * Sustainable monthly income the portfolio could pay out today at the safe
 * withdrawal rate. A portfolio underwater from liabilities pays nothing.
 */
export function safeMonthlyWithdrawal(investable: number, rate: number = SAFE_WITHDRAWAL_RATE): number {
  return (Math.max(0, investable) * rate) / 12
}
