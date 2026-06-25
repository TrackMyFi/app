/**
 * Major balance milestones an account can cross on the way to financial
 * independence. Sparse on purpose: these are the figures that feel like life
 * markers — $10k, the first $100k, the first million — not every round number.
 * Crossing one is the moment the FIRE journey feels real, so it earns a beat
 * of celebration in the UI.
 */
const BASE_MILESTONES = [
  1_000, 5_000, 10_000, 25_000, 50_000, 100_000, 250_000, 500_000,
]

// 1M, 2M, 3M … 100M — every million stays meaningful past the first.
const MILLIONS = Array.from({ length: 100 }, (_, i) => (i + 1) * 1_000_000)

/** Ascending list of celebrated milestones. */
export const MILESTONES: readonly number[] = [...BASE_MILESTONES, ...MILLIONS]

/**
 * The highest milestone strictly above `prev` and at or below `next`, or null
 * if the balance didn't cross one. `prev` may be null for a brand-new account's
 * first balance (treated as 0). Only upward movement crosses a milestone — a
 * flat or falling balance always returns null.
 */
export function crossedMilestone(prev: number | null, next: number): number | null {
  const floor = prev ?? 0
  if (next <= floor) return null
  let crossed: number | null = null
  for (const m of MILESTONES) {
    if (m <= floor) continue
    if (m <= next) crossed = m
    else break
  }
  return crossed
}
