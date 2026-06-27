export const ASSET_EVENT_KINDS = ['purchase', 'improvement', 'maintenance', 'repair'] as const
export type AssetEventKind = typeof ASSET_EVENT_KINDS[number]

export const ASSET_EVENT_KIND_LABELS: Record<AssetEventKind, string> = {
  purchase: 'Purchase',
  improvement: 'Improvement',
  maintenance: 'Maintenance',
  repair: 'Repair',
}

export const labelForAssetEventKind = (kind: string): string =>
  ASSET_EVENT_KIND_LABELS[kind as AssetEventKind] ?? kind

export const assetEventKindItems = ASSET_EVENT_KINDS.map((k) => ({
  label: ASSET_EVENT_KIND_LABELS[k],
  value: k,
}))

// NuxtUI semantic color per kind, used for badges.
export const ASSET_EVENT_KIND_COLOR: Record<AssetEventKind, 'primary' | 'success' | 'info' | 'warning'> = {
  purchase: 'primary',
  improvement: 'success',
  maintenance: 'info',
  repair: 'warning',
}

export const colorForAssetEventKind = (kind: string) =>
  ASSET_EVENT_KIND_COLOR[kind as AssetEventKind] ?? 'neutral'

// Common life expectancy suggestions for the purchase form field.
// Free-text input is always allowed; these are autocomplete hints only.
export const LIFE_EXPECTANCY_SUGGESTIONS = [
  'Asphalt roof: 20–30 years',
  'Metal roof: 40–70 years',
  'Furnace: 15–25 years',
  'AC unit: 15–20 years',
  'Heat pump: 10–15 years',
  'Water heater (tank): 8–12 years',
  'Water heater (tankless): 20 years',
  'Pressure relief valve: 4–6 years',
  'Gutters & downspouts: 20–30 years',
  'Siding (vinyl): 20–40 years',
  'Siding (wood): 10–15 years',
  'Windows: 15–30 years',
  'Garage door: 15–30 years',
  'Deck (wood): 15–20 years',
  'Driveway (asphalt): 25–30 years',
  'Septic tank: 25–30 years',
  'Dishwasher: 9–16 years',
  'Refrigerator: 10–20 years',
  'Washer / dryer: 10–15 years',
  'Oven / range: 15–20 years',
  'Garbage disposal: 10–15 years',
  'Smoke & CO detectors: 7–10 years',
]
