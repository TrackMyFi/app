import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { SimpleFinStatus } from '../lib/types/SimpleFinStatus'
import * as api from '../lib/api/simplefin'

export const useSimpleFinStore = defineStore('simplefin', () => {
  const status = ref<SimpleFinStatus | null>(null)
  let subscribed = false

  async function load() {
    status.value = await api.getSimpleFinStatus()
    if (!subscribed) {
      subscribed = true
      await api.onSimpleFinStatus((s) => {
        status.value = s
      })
    }
  }

  async function connect(setupToken: string) {
    status.value = await api.connectSimpleFin(setupToken)
  }

  async function link(simplefinId: string, accountId: number | null) {
    await api.linkSimpleFinAccount(simplefinId, accountId)
    await load()
  }

  async function syncNow() {
    const summary = await api.syncSimpleFinNow()
    await load()
    return summary
  }

  async function syncRange(startDate: string, endDate: string) {
    const summary = await api.syncSimpleFinRange(startDate, endDate)
    await load()
    return summary
  }

  async function disconnect() {
    await api.disconnectSimpleFin()
    await load()
  }

  return { status, load, connect, link, syncNow, syncRange, disconnect }
})
