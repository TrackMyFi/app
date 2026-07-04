/**
 * The 4% rule — the default when the profile doesn't override `withdrawalRate`.
 * `fireNumber` divides by the same rate, so FI progress and safe income agree
 * for any configured rate.
 */
export const SAFE_WITHDRAWAL_RATE = 0.04

/**
 * Sustainable monthly income the portfolio could pay out today at the safe
 * withdrawal rate. A portfolio underwater from liabilities pays nothing.
 */
export function safeMonthlyWithdrawal(investable: number, rate: number = SAFE_WITHDRAWAL_RATE): number {
  return (Math.max(0, investable) * rate) / 12
}
