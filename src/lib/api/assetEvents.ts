import { invoke } from '@tauri-apps/api/core'
import type { AssetEvent } from '../types/AssetEvent'

export interface AssetEventFilter {
  accountId?: number | null
  kind?: string | null
  startDate?: string | null
  endDate?: string | null
  search?: string | null
}

export interface NewAssetEvent {
  accountId?: number | null
  assetLabel?: string | null
  date: string
  description: string
  kind: string
  cost: number
  assetValue?: number | null
  vendor?: string | null
  notes?: string | null
  lifeExpectancy?: string | null
  linkedTransactionId?: number | null
  createdAt: string
}

export interface UpdateAssetEvent {
  id: number
  accountId?: number | null
  assetLabel?: string | null
  date: string
  description: string
  kind: string
  cost: number
  assetValue?: number | null
  vendor?: string | null
  notes?: string | null
  lifeExpectancy?: string | null
  linkedTransactionId?: number | null
  updatedAt: string
}

export const listAssetEvents = (filter: AssetEventFilter = {}) =>
  invoke<AssetEvent[]>('list_asset_events_cmd', { filter })
export const getAssetEvent = (id: number) =>
  invoke<AssetEvent>('get_asset_event_cmd', { id })
export const createAssetEvent = (event: NewAssetEvent) =>
  invoke<AssetEvent>('create_asset_event_cmd', { event })
export const updateAssetEvent = (event: UpdateAssetEvent) =>
  invoke<AssetEvent>('update_asset_event_cmd', { event })
export const deleteAssetEvent = (id: number) =>
  invoke<void>('delete_asset_event_cmd', { id })
