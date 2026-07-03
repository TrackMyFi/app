import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { TransactionPage } from '../lib/types/TransactionPage'
import type { Transaction } from '../lib/types/Transaction'
import * as api from '../lib/api/transactions'

const EMPTY: TransactionPage = { rows: [], totalCount: 0, totalIncome: 0, totalExpense: 0, net: 0, suppressedCount: 0 }
const PAGE_SIZE = 200

export const useTransactionsStore = defineStore('transactions', () => {
  const page = ref<TransactionPage>(EMPTY)
  const filter = ref<api.TransactionFilter>({ limit: PAGE_SIZE, offset: 0 })
  const loading = ref(false)

  const hasMore = computed(() => page.value.rows.length < page.value.totalCount)

  async function load() {
    loading.value = true
    try {
      page.value = await api.listTransactions(filter.value)
    } finally {
      loading.value = false
    }
  }

  async function setFilter(patch: Partial<api.TransactionFilter>) {
    filter.value = { ...filter.value, ...patch, offset: 0, limit: PAGE_SIZE }
    await load()
  }

  async function loadMore() {
    if (!hasMore.value || loading.value) return
    loading.value = true
    try {
      const nextFilter = { ...filter.value, offset: page.value.rows.length }
      const result = await api.listTransactions(nextFilter)
      page.value = { ...result, rows: [...page.value.rows, ...result.rows] }
    } finally {
      loading.value = false
    }
  }

  async function create(t: api.NewTransaction) { await api.createTransaction(t); await load() }
  async function update(t: api.UpdateTransaction) { await api.updateTransaction(t); await load() }
  async function remove(id: number) { await api.deleteTransaction(id); await load() }

  return { page, filter, loading, hasMore, load, setFilter, loadMore, create, update, remove }
})

export type { Transaction }
