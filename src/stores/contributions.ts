import { defineStore } from 'pinia'
import { ref } from 'vue'
import { DateTime } from 'luxon'
import type { Transaction } from '../lib/types/Transaction'
import * as api from '../lib/api/contributions'

export const useContributionsStore = defineStore('contributions', () => {
  const txns = ref<Transaction[]>([])
  const years = ref<number[]>([])

  async function loadYears() {
    const raw = await api.listContributionYears()
    years.value = raw.map(Number)
    // Ensure the current year is always selectable even with no data yet.
    const current = DateTime.now().year
    if (!years.value.includes(current)) {
      years.value = [current, ...years.value].sort((a, b) => b - a)
    }
  }

  // The selected year is owned by the page; the store just fetches its window.
  async function load(year: number) {
    txns.value = await api.listContributionTxns(year)
  }

  // Lifetime history — the dashboard needs it for Roth basis, which sums
  // every contribution ever made, not just a year window.
  async function loadAll() {
    txns.value = await api.listAllContributionTxns()
  }

  return { txns, years, loadYears, load, loadAll }
})
