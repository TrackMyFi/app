import { defineStore } from 'pinia'
import { ref } from 'vue'
import { DateTime } from 'luxon'
import type { Transaction } from '../lib/types/Transaction'
import * as api from '../lib/api/contributions'

export const useContributionsStore = defineStore('contributions', () => {
  const txns = ref<Transaction[]>([])
  const years = ref<number[]>([])
  const selectedYear = ref<number>(DateTime.now().year)

  async function loadYears() {
    const raw = await api.listContributionYears()
    years.value = raw.map(Number)
    // Ensure the current year is always selectable even with no data yet.
    const current = DateTime.now().year
    if (!years.value.includes(current)) {
      years.value = [current, ...years.value].sort((a, b) => b - a)
    }
  }

  async function load(year: number) {
    selectedYear.value = year
    txns.value = await api.listContributionTxns(year)
  }

  return { txns, years, selectedYear, loadYears, load }
})
