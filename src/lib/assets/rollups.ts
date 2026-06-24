import type { AssetEvent } from '../types/AssetEvent'

const MS_PER_YEAR = 365.25 * 24 * 60 * 60 * 1000

const sum = (events: AssetEvent[]) => events.reduce((acc, e) => acc + e.cost, 0)

/** Total of every event cost, regardless of kind. */
export const lifetimeCost = (events: AssetEvent[]): number => sum(events)

/** Sum of `improvement` costs — the amount added to an asset's cost basis. */
export const costBasisAdded = (events: AssetEvent[]): number =>
  sum(events.filter((e) => e.kind === 'improvement'))

/** Sum of `purchase` costs — the asset's acquisition cost. */
export const purchaseCost = (events: AssetEvent[]): number =>
  sum(events.filter((e) => e.kind === 'purchase'))

/**
 * Sum of recurring upkeep (maintenance + repair). Excludes one-time capital
 * events — purchases and improvements.
 */
export const upkeepCost = (events: AssetEvent[]): number =>
  sum(events.filter((e) => e.kind === 'maintenance' || e.kind === 'repair'))

/**
 * Calendar years spanned from the earliest to the latest event.
 * Floored at 1 so a single event (or a sub-year span) doesn't inflate the
 * annualized figure by dividing by a fraction.
 */
export const yearsSpanned = (events: AssetEvent[]): number => {
  if (events.length < 2) return 1
  const times = events.map((e) => Date.parse(e.date)).filter((t) => !Number.isNaN(t))
  if (times.length < 2) return 1
  const span = (Math.max(...times) - Math.min(...times)) / MS_PER_YEAR
  return Math.max(1, span)
}

/**
 * Ongoing cost of ownership spread across the years it spans — the input to a
 * maintenance reserve / sinking fund. Includes improvements (lumpy capital costs
 * like a roof are exactly what a reserve smooths) but EXCLUDES the one-time
 * purchase, which would otherwise wildly inflate the per-year figure. The
 * purchase date still anchors the span, so the reserve is spread over the full
 * ownership period.
 */
export const annualizedCost = (events: AssetEvent[]): number => {
  if (events.length === 0) return 0
  const ongoing = lifetimeCost(events) - purchaseCost(events)
  return ongoing / yearsSpanned(events)
}

/**
 * The asset's "last known value" — the `assetValue` from its most recent event
 * (by date, then id) that has one set. Returns null if no event carries a value.
 * Used for free-text assets; account-linked (real-estate) assets derive value
 * from their account's latest balance snapshot instead.
 */
export const currentValue = (events: AssetEvent[]): number | null => {
  const withValue = events.filter((e) => e.assetValue != null)
  if (withValue.length === 0) return null
  const latest = withValue.reduce((best, e) => {
    if (e.date > best.date) return e
    if (e.date === best.date && e.id > best.id) return e
    return best
  })
  return latest.assetValue
}

export interface AssetGroup {
  key: string
  accountId: number | null
  label: string | null
  events: AssetEvent[]
}

/**
 * Group events by the asset they belong to — keyed by account id when linked,
 * otherwise by the (case-insensitive) free-text label. Groups are returned
 * sorted by descending lifetime cost.
 */
export const groupByAsset = (events: AssetEvent[]): AssetGroup[] => {
  const groups = new Map<string, AssetGroup>()
  for (const e of events) {
    const key =
      e.accountId != null
        ? `acct:${e.accountId}`
        : `label:${(e.assetLabel ?? '').trim().toLowerCase()}`
    let group = groups.get(key)
    if (!group) {
      group = { key, accountId: e.accountId, label: e.assetLabel, events: [] }
      groups.set(key, group)
    }
    group.events.push(e)
  }
  return [...groups.values()].sort((a, b) => lifetimeCost(b.events) - lifetimeCost(a.events))
}
