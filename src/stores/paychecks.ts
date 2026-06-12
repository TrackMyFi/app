import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Paycheck } from '../lib/types/Paycheck'
import * as api from '../lib/api/paychecks'

export const usePaychecksStore = defineStore('paychecks', () => {
  const paychecks = ref<Paycheck[]>([])
  const filter = ref<api.PaycheckFilter>({})

  async function load() {
    paychecks.value = await api.listPaychecks(filter.value)
  }
  async function setFilter(patch: Partial<api.PaycheckFilter>) {
    filter.value = { ...filter.value, ...patch }
    await load()
  }
  async function create(p: api.NewPaycheck) {
    await api.createPaycheck(p)
    await load()
  }
  async function update(p: api.UpdatePaycheck) {
    await api.updatePaycheck(p)
    await load()
  }
  async function remove(id: number) {
    await api.deletePaycheck(id)
    await load()
  }

  return { paychecks, filter, load, setFilter, create, update, remove }
})

export type { Paycheck }
