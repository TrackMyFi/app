import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { TransactionPage } from '../lib/types/TransactionPage'
import type { Transaction } from '../lib/types/Transaction'
import * as api from '../lib/api/transactions'

const EMPTY: TransactionPage = { rows: [], totalCount: 0, totalIncome: 0, totalExpense: 0, net: 0 }

export const useTransactionsStore = defineStore('transactions', () => {
  const page = ref<TransactionPage>(EMPTY)
  const filter = ref<api.TransactionFilter>({ limit: 200, offset: 0 })

  async function load() {
    page.value = await api.listTransactions(filter.value)
  }
  async function setFilter(patch: Partial<api.TransactionFilter>) {
    filter.value = { ...filter.value, ...patch, offset: 0 }
    await load()
  }
  async function create(t: api.NewTransaction) { await api.createTransaction(t); await load() }
  async function update(t: api.UpdateTransaction) { await api.updateTransaction(t); await load() }
  async function remove(id: number) { await api.deleteTransaction(id); await load() }

  return { page, filter, load, setFilter, create, update, remove }
})

export type { Transaction }
