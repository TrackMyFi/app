/**
 * Shared "this period vs. a typical period" comparison helpers, used by any
 * page that compares a current figure against `computeMedian`'s reference
 * periods (Transactions, Expenses).
 */

/** Signed % change of `current` vs. `med`, or null when there's no baseline to compare against. */
export function pctVsMedian(current: number, med: number): number | null {
  if (med === 0) return null
  return (current - med) / Math.abs(med)
}

export type TrendField = 'income' | 'expense' | 'savings' | 'net'

// Favorable direction: income/savings/net → higher is better; expense → lower is better.
export function changeColor(field: TrendField, pct: number | null): string {
  if (pct == null || Math.abs(pct) < 0.005) return 'text-muted'
  const favorable = field === 'expense' ? pct < 0 : pct > 0
  return favorable ? 'text-success' : 'text-error'
}

// A calm directional cue instead of a raw percentage delta: an arrow that says
// "above / below / on par with a typical period." Raw "% vs median" produces
// absurd figures (+5102%) when the baseline is near-zero.
export function trendIcon(pct: number | null): string {
  if (pct == null || Math.abs(pct) < 0.005) return 'i-ph-minus'
  return pct > 0 ? 'i-ph-arrow-up' : 'i-ph-arrow-down'
}
