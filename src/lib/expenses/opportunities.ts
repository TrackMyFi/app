/** A single row in the Savings Opportunities list — recurring charge or category spike, normalized for display. */
export interface OpportunityItem {
  id: string
  tone: 'warning' | 'error'
  icon: string
  title: string
  subtitle: string
  trailing: string
  /** Present for recurring charges — lets the row drill into Transactions. */
  searchTerm?: string
}

export interface CategorySpike {
  category: 'fixed' | 'discretionary'
  amount: number
  typical: number
  /** Positive fraction — how far above typical the current period ran. */
  pct: number
}

export interface CategorySpikeOptions {
  /** Minimum fractional increase vs. typical to bother flagging. */
  minPct?: number
  /** Minimum absolute dollar increase vs. typical to bother flagging. */
  minDelta?: number
}

/**
 * Flags Fixed/Discretionary totals running meaningfully above a typical
 * period — only increases matter here, since a decrease is already good news
 * the trend arrow on the stat block conveys. Small percentage swings on tiny
 * categories are filtered by `minDelta` so a $3 category doesn't "spike" 200%.
 */
export function detectCategorySpikes(
  current: { fixed: number; discretionary: number },
  typical: { fixed: number; discretionary: number } | null,
  options: CategorySpikeOptions = {},
): CategorySpike[] {
  if (!typical) return []
  const minPct = options.minPct ?? 0.15
  const minDelta = options.minDelta ?? 25

  const spikes: CategorySpike[] = []
  for (const category of ['fixed', 'discretionary'] as const) {
    const amount = current[category]
    const typicalAmount = typical[category]
    if (typicalAmount <= 0) continue

    const delta = amount - typicalAmount
    const pct = delta / typicalAmount
    if (pct >= minPct && delta >= minDelta) {
      spikes.push({ category, amount, typical: typicalAmount, pct })
    }
  }

  return spikes.sort((a, b) => b.pct - a.pct)
}
