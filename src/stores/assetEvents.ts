import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { AssetEvent } from '../lib/types/AssetEvent'
import * as api from '../lib/api/assetEvents'

export const useAssetEventsStore = defineStore('assetEvents', () => {
  const assetEvents = ref<AssetEvent[]>([])
  const filter = ref<api.AssetEventFilter>({})

  async function load() {
    assetEvents.value = await api.listAssetEvents(filter.value)
  }
  async function setFilter(patch: Partial<api.AssetEventFilter>) {
    filter.value = { ...filter.value, ...patch }
    await load()
  }
  async function create(e: api.NewAssetEvent) {
    const result = await api.createAssetEvent(e)
    await load()
    return result
  }
  async function update(e: api.UpdateAssetEvent) {
    const result = await api.updateAssetEvent(e)
    await load()
    return result
  }
  async function remove(id: number) {
    await api.deleteAssetEvent(id)
    await load()
  }

  return { assetEvents, filter, load, setFilter, create, update, remove }
})

export type { AssetEvent }
