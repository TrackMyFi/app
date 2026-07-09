import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { HsaExpense } from '../lib/types/HsaExpense'
import * as api from '../lib/api/hsaExpenses'

export const useHsaExpensesStore = defineStore('hsaExpenses', () => {
  const hsaExpenses = ref<HsaExpense[]>([])
  const filter = ref<api.HsaExpenseFilter>({})

  async function load() {
    hsaExpenses.value = await api.listHsaExpenses(filter.value)
  }
  async function setFilter(patch: Partial<api.HsaExpenseFilter>) {
    filter.value = { ...filter.value, ...patch }
    await load()
  }
  async function create(e: api.NewHsaExpense) {
    const result = await api.createHsaExpense(e)
    await load()
    return result
  }
  async function update(e: api.UpdateHsaExpense) {
    const result = await api.updateHsaExpense(e)
    await load()
    return result
  }
  async function remove(id: number) {
    await api.deleteHsaExpense(id)
    await load()
  }

  return { hsaExpenses, filter, load, setFilter, create, update, remove }
})

export type { HsaExpense }
