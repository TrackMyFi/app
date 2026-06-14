import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { SyncStatus } from '../lib/types/SyncStatus'
import { getSyncStatus, onSyncStatus } from '../lib/api/sync'

export const useSyncStore = defineStore('sync', () => {
  const status = ref<SyncStatus | null>(null)
  let subscribed = false

  async function init() {
    status.value = await getSyncStatus()
    if (!subscribed) {
      subscribed = true
      await onSyncStatus((s) => {
        status.value = s
      })
    }
  }

  function setStatus(s: SyncStatus) {
    status.value = s
  }

  return { status, init, setStatus }
})
