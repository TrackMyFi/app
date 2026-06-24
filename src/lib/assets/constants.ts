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
