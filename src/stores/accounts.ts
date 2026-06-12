import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Account } from '../lib/types/Account'
import type { AccountBalance } from '../lib/types/AccountBalance'
import * as api from '../lib/api/accounts'

export const useAccountsStore = defineStore('accounts', () => {
  const accounts = ref<Account[]>([])
  const allBalances = ref<AccountBalance[]>([])

  async function load() {
    accounts.value = await api.listAccounts()
    allBalances.value = await api.listAllBalances()
  }
  async function create(a: Parameters<typeof api.createAccount>[0]) { await api.createAccount(a); await load() }
  async function archive(id: number) { await api.archiveAccount(id); await load() }
  async function unarchive(id: number) { await api.unarchiveAccount(id); await load() }
  async function remove(id: number) { await api.deleteAccount(id); await load() }
  async function addBalanceSnapshot(b: Parameters<typeof api.addBalance>[0]) { await api.addBalance(b); await load() }
  async function update(id: number, a: Parameters<typeof api.updateAccount>[1]) { await api.updateAccount(id, a); await load() }
  async function updateBalanceSnapshot(b: Parameters<typeof api.updateBalance>[0]) { await api.updateBalance(b); await load() }
  async function removeBalanceSnapshot(id: number) { await api.deleteBalance(id); await load() }

  return { accounts, allBalances, load, create, update, archive, unarchive, remove, addBalanceSnapshot, updateBalanceSnapshot, removeBalanceSnapshot }
})
