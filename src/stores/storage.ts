import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import * as storageApi from '../lib/api/storage'
import type { SaveStorageConfigArgs } from '../lib/api/storage'
import type { StorageInfo } from '../lib/types/StorageInfo'

export const useStorageStore = defineStore('storage', () => {
  const config = ref<StorageInfo | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  const provider = computed(() => config.value?.provider ?? 'local')

  const isCloudProvider = computed(() =>
    ['r2', 'gcs', 's3'].includes(provider.value),
  )

  const providerLabel = computed(() => {
    switch (provider.value) {
      case 'r2':  return 'Cloudflare R2'
      case 'gcs': return 'Google Cloud Storage'
      case 's3':  return 'Amazon S3'
      default:    return 'Local storage'
    }
  })

  const syncLabel = computed(() =>
    isCloudProvider.value
      ? `Synced via ${providerLabel.value}`
      : 'Stored locally · not synced across devices',
  )

  async function load() {
    loading.value = true
    error.value = null
    try {
      config.value = await storageApi.getStorageConfig()
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function save(args: SaveStorageConfigArgs) {
    await storageApi.saveStorageConfig(args)
    await load()
  }

  async function clear() {
    await storageApi.clearStorageConfig()
    await load()
  }

  return {
    config,
    loading,
    error,
    provider,
    isCloudProvider,
    providerLabel,
    syncLabel,
    load,
    save,
    clear,
  }
})
